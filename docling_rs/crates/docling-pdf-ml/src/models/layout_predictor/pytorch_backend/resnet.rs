// ResNet implementation for RT-DETR v2 backbone
// Ported from transformers/models/resnet/modeling_resnet.py

use tch::{nn, Tensor};

/// ResNet convolutional layer with BatchNorm and activation
/// Python: ResNetConvLayer (lines 39-54)
#[derive(Debug)]
pub struct ResNetConvLayer {
    convolution: nn::Conv2D,
    normalization: nn::BatchNorm,
    activation: Option<Activation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    ReLU,
}

impl std::fmt::Display for Activation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReLU => write!(f, "relu"),
        }
    }
}

impl std::str::FromStr for Activation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "relu" => Ok(Self::ReLU),
            _ => Err(format!("unknown activation function: '{s}'")),
        }
    }
}

impl ResNetConvLayer {
    pub fn new(
        vs: &nn::Path,
        in_channels: i64,
        out_channels: i64,
        kernel_size: i64,
        stride: i64,
        activation: Option<Activation>,
    ) -> Self {
        let padding = kernel_size / 2;
        let conv_config = nn::ConvConfig {
            stride,
            padding,
            bias: false,
            ..Default::default()
        };

        let convolution = nn::conv2d(
            vs / "convolution",
            in_channels,
            out_channels,
            kernel_size,
            conv_config,
        );
        let normalization =
            nn::batch_norm2d(vs / "normalization", out_channels, Default::default());

        Self {
            convolution,
            normalization,
            activation,
        }
    }

    pub fn forward(&self, input: &Tensor) -> Tensor {
        let mut hidden_state = input.apply(&self.convolution);
        hidden_state = hidden_state.apply_t(&self.normalization, false); // false = eval mode (frozen BN)

        if let Some(act) = self.activation {
            hidden_state = match act {
                Activation::ReLU => hidden_state.relu(),
            };
        }

        hidden_state
    }
}

/// ResNet shortcut for residual connections
/// Python: ResNetShortCut (lines 81-95)
/// For stages 1-3, wrapped in Sequential[AvgPool2d, ResNetShortCut]
#[derive(Debug)]
pub struct ResNetShortCut {
    convolution: nn::Conv2D,
    normalization: nn::BatchNorm,
    use_avgpool: bool,   // Whether to apply AvgPool2d before convolution
    avgpool_stride: i64, // Stride for AvgPool2d (usually 2)
}

impl ResNetShortCut {
    pub fn new(
        vs: &nn::Path,
        in_channels: i64,
        out_channels: i64,
        stride: i64,
        use_avgpool: bool,
    ) -> Self {
        // When use_avgpool=true (stages 1-3), the Sequential contains AvgPool2d at index 0
        // and the actual shortcut uses stride=1 (avgpool handles downsampling)
        let conv_stride = if use_avgpool { 1 } else { stride };

        let conv_config = nn::ConvConfig {
            stride: conv_stride,
            bias: false,
            ..Default::default()
        };

        let convolution = nn::conv2d(
            vs / "convolution",
            in_channels,
            out_channels,
            1,
            conv_config,
        );
        let normalization =
            nn::batch_norm2d(vs / "normalization", out_channels, Default::default());

        Self {
            convolution,
            normalization,
            use_avgpool,
            avgpool_stride: stride, // AvgPool2d uses the original stride
        }
    }

    pub fn forward(&self, input: &Tensor) -> Tensor {
        let mut hidden_state = input.shallow_clone();

        // Apply AvgPool2d if wrapped in Sequential (stages 1-3)
        if self.use_avgpool {
            // AvgPool2d(kernel_size=stride, stride=stride, padding=0)
            hidden_state = hidden_state.avg_pool2d(
                [self.avgpool_stride, self.avgpool_stride],
                [self.avgpool_stride, self.avgpool_stride],
                [0, 0],
                false, // ceil_mode
                true,  // count_include_pad
                None,  // divisor_override
            );
        }

        // Apply convolution and normalization
        hidden_state = hidden_state.apply(&self.convolution);
        hidden_state = hidden_state.apply_t(&self.normalization, false); // false = eval mode
        hidden_state
    }
}

/// ResNet bottleneck layer (1x1 -> 3x3 -> 1x1 conv)
/// Python: ResNetBottleNeckLayer (lines 124-163)
#[derive(Debug)]
pub struct ResNetBottleNeckLayer {
    shortcut: Option<ResNetShortCut>,
    layer1: ResNetConvLayer,
    layer2: ResNetConvLayer,
    layer3: ResNetConvLayer,
    activation: Activation,
}

impl ResNetBottleNeckLayer {
    #[allow(
        clippy::too_many_arguments,
        reason = "ResNet bottleneck layer requires many configuration params"
    )]
    pub fn new(
        vs: &nn::Path,
        in_channels: i64,
        out_channels: i64,
        stride: i64,
        activation: Activation,
        reduction: i64,
        downsample_in_bottleneck: bool,
        use_sequential_shortcut: bool, // NEW: whether shortcut is wrapped in Sequential
    ) -> Self {
        let should_apply_shortcut = in_channels != out_channels || stride != 1;
        let reduces_channels = out_channels / reduction;

        let shortcut = if should_apply_shortcut {
            // HuggingFace RT-DETR v2 uses different shortcut paths:
            // - Stage 0: shortcut.convolution (direct)
            // - Stages 1-3: shortcut.1.convolution (wrapped in Sequential with AvgPool2d at 0)
            let shortcut_path = if use_sequential_shortcut {
                vs / "shortcut" / "1" // Stages 1-3: Sequential[AvgPool2d, ResNetShortCut]
            } else {
                vs / "shortcut" // Stage 0: Direct ResNetShortCut
            };
            Some(ResNetShortCut::new(
                &shortcut_path,
                in_channels,
                out_channels,
                stride,
                use_sequential_shortcut,
            ))
        } else {
            None
        };

        // Layer 1: 1x1 conv (reduce channels)
        let stride1 = if downsample_in_bottleneck { stride } else { 1 };
        let layer1 = ResNetConvLayer::new(
            &(vs / "layer" / "0"),
            in_channels,
            reduces_channels,
            1,
            stride1,
            Some(activation),
        );

        // Layer 2: 3x3 conv (process)
        let stride2 = if !downsample_in_bottleneck { stride } else { 1 };
        let layer2 = ResNetConvLayer::new(
            &(vs / "layer" / "1"),
            reduces_channels,
            reduces_channels,
            3,
            stride2,
            Some(activation),
        );

        // Layer 3: 1x1 conv (expand channels, no activation)
        let layer3 = ResNetConvLayer::new(
            &(vs / "layer" / "2"),
            reduces_channels,
            out_channels,
            1,
            1,
            None, // No activation
        );

        Self {
            shortcut,
            layer1,
            layer2,
            layer3,
            activation,
        }
    }

    pub fn forward(&self, input: &Tensor) -> Tensor {
        let residual = input;

        // Forward through layers
        let mut hidden_state = self.layer1.forward(residual);
        hidden_state = self.layer2.forward(&hidden_state);
        hidden_state = self.layer3.forward(&hidden_state);

        // Add shortcut
        let residual = if let Some(ref shortcut) = self.shortcut {
            shortcut.forward(residual)
        } else {
            residual.shallow_clone()
        };

        hidden_state += residual;

        // Final activation
        hidden_state = match self.activation {
            Activation::ReLU => hidden_state.relu(),
        };

        hidden_state
    }
}

/// ResNet stage (stacked bottleneck layers)
/// Python: ResNetStage (lines 166-203)
#[derive(Debug)]
pub struct ResNetStage {
    layers: Vec<ResNetBottleNeckLayer>,
}

impl ResNetStage {
    #[allow(
        clippy::too_many_arguments,
        reason = "ResNet stage requires many configuration params"
    )]
    pub fn new(
        vs: &nn::Path,
        in_channels: i64,
        out_channels: i64,
        stride: i64,
        depth: i64,
        activation: Activation,
        downsample_in_bottleneck: bool,
        use_sequential_shortcut: bool, // NEW: whether shortcuts are wrapped in Sequential
    ) -> Self {
        let mut layers = Vec::new();

        // First layer (may have stride for downsampling)
        layers.push(ResNetBottleNeckLayer::new(
            &(vs / "layers" / "0"),
            in_channels,
            out_channels,
            stride,
            activation,
            4, // reduction factor
            downsample_in_bottleneck,
            use_sequential_shortcut,
        ));

        // Remaining layers (no downsampling)
        for i in 1..depth {
            layers.push(ResNetBottleNeckLayer::new(
                &(vs / "layers" / i),
                out_channels,
                out_channels,
                1, // stride = 1
                activation,
                4, // reduction factor
                downsample_in_bottleneck,
                use_sequential_shortcut,
            ));
        }

        Self { layers }
    }

    pub fn forward(&self, input: &Tensor) -> Tensor {
        let mut hidden_state = input.shallow_clone();
        for layer in self.layers.iter() {
            hidden_state = layer.forward(&hidden_state);
        }
        hidden_state
    }
}

/// ResNet embeddings (stem) - RT-DETR uses 3 sequential conv3x3 layers
/// Python: RTDetrResNetEmbeddings with Sequential[3 x RTDetrResNetConvLayer]
#[derive(Debug)]
pub struct ResNetEmbeddings {
    embedder: Vec<ResNetConvLayer>,
    pooler: bool, // Whether to apply maxpool after embeddings
}

impl ResNetEmbeddings {
    pub fn new(
        vs: &nn::Path,
        num_channels: i64,
        embedding_size: i64,
        activation: Activation,
    ) -> Self {
        // RT-DETR uses 3 conv layers: (3→32 s=2), (32→32 s=1), (32→64 s=1)
        // All use kernel_size=3
        let embedder = vec![
            ResNetConvLayer::new(
                &(vs / "embedder" / "0"),
                num_channels, // 3
                32,           // first output channels
                3,            // kernel_size
                2,            // stride
                Some(activation),
            ),
            ResNetConvLayer::new(
                &(vs / "embedder" / "1"),
                32, // in = first output
                32, // keep same
                3,  // kernel_size
                1,  // stride
                Some(activation),
            ),
            ResNetConvLayer::new(
                &(vs / "embedder" / "2"),
                32,             // in = second output
                embedding_size, // 64 final channels
                3,              // kernel_size
                1,              // stride
                Some(activation),
            ),
        ];

        Self {
            embedder,
            pooler: true, // RT-DETR applies maxpool after embeddings
        }
    }

    pub fn forward(&self, pixel_values: &Tensor) -> Tensor {
        // Apply 3 conv layers sequentially
        let mut embedding = pixel_values.shallow_clone();
        for layer in &self.embedder {
            embedding = layer.forward(&embedding);
        }

        // MaxPool2d: kernel_size=3, stride=2, padding=1
        if self.pooler {
            embedding = embedding.max_pool2d([3, 3], [2, 2], [1, 1], [1, 1], false);
        }

        embedding
    }
}

/// ResNet encoder - multiple stages of bottleneck blocks
/// Python: ResNetEncoder (lines 204-242)
#[derive(Debug)]
pub struct ResNetEncoder {
    stages: Vec<ResNetStage>,
}

impl ResNetEncoder {
    /// Create ResNet-50 encoder with standard configuration
    /// - Stage 1: 3 blocks, 256 channels
    /// - Stage 2: 4 blocks, 512 channels
    /// - Stage 3: 6 blocks, 1024 channels
    /// - Stage 4: 3 blocks, 2048 channels
    pub fn new(
        vs: &nn::Path,
        embedding_size: i64,
        hidden_sizes: &[i64],
        depths: &[i64],
        activation: Activation,
        downsample_in_first_stage: bool,
        downsample_in_bottleneck: bool,
    ) -> Self {
        let mut stages = Vec::new();

        // First stage (may or may not downsample based on config)
        // Stage 0 uses direct shortcut (no Sequential wrapper)
        let stride1 = if downsample_in_first_stage { 2 } else { 1 };
        stages.push(ResNetStage::new(
            &(vs / "stages" / "0"),
            embedding_size,
            hidden_sizes[0],
            stride1,
            depths[0],
            activation,
            downsample_in_bottleneck,
            false, // use_sequential_shortcut = false for stage 0
        ));

        // Remaining stages (always downsample with stride=2)
        // Stages 1-3 use Sequential[AvgPool2d, ResNetShortCut]
        for i in 1..hidden_sizes.len() {
            stages.push(ResNetStage::new(
                &(vs / "stages" / (i as i64)),
                hidden_sizes[i - 1],
                hidden_sizes[i],
                2, // stride = 2 for downsampling
                depths[i],
                activation,
                downsample_in_bottleneck,
                true, // use_sequential_shortcut = true for stages 1-3
            ));
        }

        Self { stages }
    }

    /// Forward pass through all stages
    /// Returns: (final_output, intermediate_outputs)
    /// intermediate_outputs: [stage1_out, stage2_out, stage3_out, stage4_out]
    pub fn forward(
        &self,
        mut hidden_state: Tensor,
        output_hidden_states: bool,
    ) -> (Tensor, Option<Vec<Tensor>>) {
        let mut hidden_states = if output_hidden_states {
            Some(Vec::new())
        } else {
            None
        };

        for (stage_idx, stage) in self.stages.iter().enumerate() {
            if let Some(ref mut states) = hidden_states {
                states.push(hidden_state.shallow_clone());
            }

            hidden_state = stage.forward(&hidden_state);
        }

        if let Some(ref mut states) = hidden_states {
            states.push(hidden_state.shallow_clone());
        }

        (hidden_state, hidden_states)
    }
}

/// Full ResNet-50 backbone
/// Python: ResNetModel (lines 268-304)
#[derive(Debug)]
pub struct ResNetBackbone {
    embeddings: ResNetEmbeddings,
    encoder: ResNetEncoder,
}

impl ResNetBackbone {
    /// Create ResNet-50 backbone with standard configuration
    ///
    /// # Arguments
    ///
    /// * `vs` - Variable store path
    /// * `num_channels` - Number of input channels (3 for RGB)
    /// * `embedding_size` - Size after initial conv7x7 (typically 64)
    /// * `hidden_sizes` - Channel sizes for each stage [256, 512, 1024, 2048]
    /// * `depths` - Number of blocks per stage [3, 4, 6, 3]
    pub fn new(
        vs: &nn::Path,
        num_channels: i64,
        embedding_size: i64,
        hidden_sizes: &[i64],
        depths: &[i64],
    ) -> Self {
        let activation = Activation::ReLU;

        let embeddings = ResNetEmbeddings::new(
            &(vs / "embedder"), // HF model uses "embedder" not "embeddings"
            num_channels,
            embedding_size,
            activation,
        );

        let encoder = ResNetEncoder::new(
            &(vs / "encoder"),
            embedding_size,
            hidden_sizes,
            depths,
            activation,
            false, // downsample_in_first_stage = false (already downsampled in embeddings)
            false, // downsample_in_bottleneck = false (standard ResNet)
        );

        Self {
            embeddings,
            encoder,
        }
    }

    /// Forward pass through backbone
    ///
    /// # Returns
    ///
    /// (final_features, multi_scale_features)
    /// - final_features: Output of last stage (C5)
    /// - multi_scale_features: [C2, C3, C4, C5] for FPN-style models
    pub fn forward(&self, pixel_values: &Tensor) -> (Tensor, Vec<Tensor>) {
        let embedding = self.embeddings.forward(pixel_values);
        let (last_hidden_state, hidden_states) = self.encoder.forward(embedding, true);

        // Extract multi-scale features (C3, C4, C5)
        // hidden_states includes: [after_stage1, after_stage2, after_stage3, after_stage4]
        // C3 = stage2, C4 = stage3, C5 = stage4
        let multi_scale = if let Some(states) = hidden_states {
            states
        } else {
            vec![last_hidden_state.shallow_clone()]
        };

        (last_hidden_state, multi_scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tch::Device;

    #[test]
    fn test_resnet_conv_layer() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let layer = ResNetConvLayer::new(&root, 3, 64, 7, 2, Some(Activation::ReLU));
        let input = Tensor::randn([1, 3, 224, 224], tch::kind::FLOAT_CPU);
        let output = layer.forward(&input);

        // Output should be downsampled by stride=2
        assert_eq!(output.size(), vec![1, 64, 112, 112]);
    }

    #[test]
    fn test_resnet_bottleneck_layer() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let layer = ResNetBottleNeckLayer::new(
            &root,
            64,
            256,
            1,
            Activation::ReLU,
            4,
            false,
            false, // use_sequential_shortcut
        );

        let input = Tensor::randn([1, 64, 56, 56], tch::kind::FLOAT_CPU);
        let output = layer.forward(&input);

        // Output channels should be expanded to 256
        assert_eq!(output.size(), vec![1, 256, 56, 56]);
    }

    #[test]
    fn test_resnet_stage() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let stage = ResNetStage::new(
            &root,
            64,
            256,
            1,
            3, // depth = 3 blocks
            Activation::ReLU,
            false,
            false, // use_sequential_shortcut
        );

        let input = Tensor::randn([1, 64, 56, 56], tch::kind::FLOAT_CPU);
        let output = stage.forward(&input);

        assert_eq!(output.size(), vec![1, 256, 56, 56]);
    }

    #[test]
    fn test_resnet_embeddings() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let embeddings = ResNetEmbeddings::new(&root, 3, 64, Activation::ReLU);
        let input = Tensor::randn([1, 3, 224, 224], tch::kind::FLOAT_CPU);
        let output = embeddings.forward(&input);

        // conv7x7 stride=2: 224 → 112
        // maxpool stride=2: 112 → 56
        assert_eq!(output.size(), vec![1, 64, 56, 56]);
    }

    #[test]
    fn test_resnet_encoder() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let hidden_sizes = vec![256, 512, 1024, 2048];
        let depths = vec![3, 4, 6, 3];
        let encoder = ResNetEncoder::new(
            &root,
            64,
            &hidden_sizes,
            &depths,
            Activation::ReLU,
            false,
            false,
        );

        let input = Tensor::randn([1, 64, 56, 56], tch::kind::FLOAT_CPU);
        let (output, _) = encoder.forward(input, false);

        // After 4 stages with stride=2 each (except first): 56 → 56 → 28 → 14 → 7
        assert_eq!(output.size(), vec![1, 2048, 7, 7]);
    }

    #[test]
    fn test_resnet_backbone() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let hidden_sizes = vec![256, 512, 1024, 2048];
        let depths = vec![3, 4, 6, 3];
        let backbone = ResNetBackbone::new(&root, 3, 64, &hidden_sizes, &depths);

        let input = Tensor::randn([1, 3, 224, 224], tch::kind::FLOAT_CPU);
        let (output, multi_scale) = backbone.forward(&input);

        // Final output (C5): 224 → 112 → 56 → 56 → 28 → 14 → 7
        assert_eq!(output.size(), vec![1, 2048, 7, 7]);

        // Multi-scale features: [before_stage1, after_stage1, after_stage2, after_stage3, after_stage4]
        assert_eq!(multi_scale.len(), 5);
        assert_eq!(multi_scale[0].size(), vec![1, 64, 56, 56]); // Before stage 1
        assert_eq!(multi_scale[1].size(), vec![1, 256, 56, 56]); // C2 (after stage 1)
        assert_eq!(multi_scale[2].size(), vec![1, 512, 28, 28]); // C3 (after stage 2)
        assert_eq!(multi_scale[3].size(), vec![1, 1024, 14, 14]); // C4 (after stage 3)
        assert_eq!(multi_scale[4].size(), vec![1, 2048, 7, 7]); // C5 (after stage 4)
    }

    #[test]
    fn test_activation_from_str() {
        use std::str::FromStr;

        // Standard case
        assert_eq!(Activation::from_str("relu").unwrap(), Activation::ReLU);

        // Case insensitivity
        assert_eq!(Activation::from_str("RELU").unwrap(), Activation::ReLU);
        assert_eq!(Activation::from_str("ReLU").unwrap(), Activation::ReLU);

        // Error case
        assert!(Activation::from_str("unknown").is_err());
    }

    #[test]
    fn test_activation_roundtrip() {
        use std::str::FromStr;

        let act = Activation::ReLU;
        let s = act.to_string();
        assert_eq!(Activation::from_str(&s).unwrap(), act);
    }
}
