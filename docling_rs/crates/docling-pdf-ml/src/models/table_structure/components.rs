use super::helpers::{CellAttention, MLP};
use log;
use tch::{nn, nn::Module, Kind, Tensor};

/// ResNet18 encoder for image feature extraction
///
/// Architecture:
/// - ResNet18 backbone (first 7 layers: conv1, bn1, relu, maxpool, layer1-3)
/// - Removes: layer4, avgpool, flatten, fc (last 4 layers)
/// - Adds: AdaptiveAvgPool2d to (28, 28)
///
/// Input:  (batch, 3, 448, 448) RGB image
/// Output: (batch, 28, 28, 256) spatial features (layer3 output is 256 channels)
pub struct Encoder {
    pub conv1: nn::Conv2D,
    pub bn1: nn::BatchNorm,
    // Layer1: 2 BasicBlocks (64→64)
    pub layer1_block0: BasicBlock,
    pub layer1_block1: BasicBlock,
    // Layer2: 2 BasicBlocks (64→128, first downsamples)
    pub layer2_block0: BasicBlock,
    pub layer2_block1: BasicBlock,
    // Layer3: 2 BasicBlocks (128→256, first downsamples)
    pub layer3_block0: BasicBlock,
    pub layer3_block1: BasicBlock,
}

impl std::fmt::Debug for Encoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Encoder")
            .field("layers", &"[conv1, bn1, layer1-3]")
            .finish()
    }
}

impl Encoder {
    pub fn new(vs: &nn::Path) -> Self {
        // Create VarStore structure to match Python state_dict
        // Structure: _encoder._resnet.{0,1,4,5,6}.*
        // ResNet18 with only first 3 layer groups (output: 256 channels)

        let resnet_path = vs / "_encoder" / "_resnet";

        // Layer 0: Initial Conv2d (7x7, stride=2, padding=3)
        // 3 channels → 64 channels
        let conv1_config = nn::ConvConfig {
            stride: 2,
            padding: 3,
            bias: false,
            ..Default::default()
        };
        let conv1 = nn::conv2d(&(resnet_path.clone() / "0"), 3, 64, 7, conv1_config);

        // Layer 1: BatchNorm2d (64 channels)
        let bn1 = nn::batch_norm2d(&(resnet_path.clone() / "1"), 64, Default::default());

        // Layer 2: ReLU (no weights)
        // Layer 3: MaxPool2d (no weights)

        // Layer 4: ResNet layer1 (2 BasicBlocks, 64→64, no downsample)
        let layer1_block0 = BasicBlock::new(&(resnet_path.clone() / "4" / "0"), 64, 64, 1, false);
        let layer1_block1 = BasicBlock::new(&(resnet_path.clone() / "4" / "1"), 64, 64, 1, false);

        // Layer 5: ResNet layer2 (2 BasicBlocks, 64→128, first has downsample)
        let layer2_block0 = BasicBlock::new(&(resnet_path.clone() / "5" / "0"), 64, 128, 2, true);
        let layer2_block1 = BasicBlock::new(&(resnet_path.clone() / "5" / "1"), 128, 128, 1, false);

        // Layer 6: ResNet layer3 (2 BasicBlocks, 128→256, first has downsample)
        let layer3_block0 = BasicBlock::new(&(resnet_path.clone() / "6" / "0"), 128, 256, 2, true);
        let layer3_block1 = BasicBlock::new(&(resnet_path.clone() / "6" / "1"), 256, 256, 1, false);

        // AdaptiveAvgPool2d to (28, 28) - no weights needed

        Encoder {
            conv1,
            bn1,
            layer1_block0,
            layer1_block1,
            layer2_block0,
            layer2_block1,
            layer3_block0,
            layer3_block1,
        }
    }

    /// Forward pass through ResNet18 encoder
    ///
    /// Input: (batch, 3, 448, 448)
    /// Output: (batch, 28, 28, 256)
    pub fn forward(&self, images: &tch::Tensor) -> tch::Tensor {
        // Initial conv + batchnorm + relu
        let mut x = images.apply(&self.conv1);
        x = x.apply_t(&self.bn1, false); // train=false for inference
        x = x.relu();

        // MaxPool2d (3x3, stride=2, padding=1)
        x = x.max_pool2d([3, 3], [2, 2], [1, 1], [1, 1], false);

        // Layer1: 2 BasicBlocks (64→64)
        x = self.layer1_block0.forward(&x);
        x = self.layer1_block1.forward(&x);

        // Layer2: 2 BasicBlocks (64→128)
        x = self.layer2_block0.forward(&x);
        x = self.layer2_block1.forward(&x);

        // Layer3: 2 BasicBlocks (128→256)
        x = self.layer3_block0.forward(&x);
        x = self.layer3_block1.forward(&x);

        // AdaptiveAvgPool2d to (28, 28)
        x = x.adaptive_avg_pool2d([28, 28]);

        // Permute from (batch, channels, height, width) to (batch, height, width, channels)
        x.permute([0, 2, 3, 1])
    }
}

/// Input filter: 2 ResNet BasicBlocks to project 256→512 channels
///
/// Used in both TagTransformer and BBoxDecoder
///
/// Input:  (batch, 256, 28, 28)
/// Output: (batch, 512, 28, 28)
pub struct InputFilter {
    pub block0: BasicBlock,
    pub block1: BasicBlock,
}

impl std::fmt::Debug for InputFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputFilter")
            .field("blocks", &"[block0: 256→512, block1: 512→512]")
            .finish()
    }
}

impl InputFilter {
    pub fn new(vs: &nn::Path, prefix: &str) -> Self {
        let path = vs / prefix / "_input_filter";

        // BasicBlock 0: 256→512 with downsample
        let block0 = BasicBlock::new(&(path.clone() / "0"), 256, 512, 1, true);

        // BasicBlock 1: 512→512 no downsample
        let block1 = BasicBlock::new(&(path / "1"), 512, 512, 1, false);

        InputFilter { block0, block1 }
    }

    pub fn forward(&self, x: &tch::Tensor) -> tch::Tensor {
        let x = self.block0.forward(x);
        self.block1.forward(&x)
    }
}

/// ResNet BasicBlock
///
/// Standard ResNet building block with optional downsampling
///
/// Architecture:
/// - conv1 (3x3) → bn1 → relu
/// - conv2 (3x3) → bn2
/// - add residual (with optional downsample)
/// - relu
pub struct BasicBlock {
    pub conv1: nn::Conv2D,
    pub bn1: nn::BatchNorm,
    pub conv2: nn::Conv2D,
    pub bn2: nn::BatchNorm,
    pub downsample: Option<(nn::Conv2D, nn::BatchNorm)>,
}

impl std::fmt::Debug for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicBlock")
            .field("has_downsample", &self.downsample.is_some())
            .finish()
    }
}

impl BasicBlock {
    pub fn new(
        vs: &nn::Path,
        in_channels: i64,
        out_channels: i64,
        stride: i64,
        with_downsample: bool,
    ) -> Self {
        let conv_config = nn::ConvConfig {
            stride,
            padding: 1,
            bias: false,
            ..Default::default()
        };

        // Main path
        let conv1 = nn::conv2d(vs / "conv1", in_channels, out_channels, 3, conv_config);
        let bn1 = nn::batch_norm2d(vs / "bn1", out_channels, Default::default());

        let conv2_config = nn::ConvConfig {
            stride: 1,
            padding: 1,
            bias: false,
            ..Default::default()
        };
        let conv2 = nn::conv2d(vs / "conv2", out_channels, out_channels, 3, conv2_config);
        let bn2 = nn::batch_norm2d(vs / "bn2", out_channels, Default::default());

        // Downsample branch (if needed)
        let downsample = if with_downsample {
            let ds_conv_config = nn::ConvConfig {
                stride,
                padding: 0,
                bias: false,
                ..Default::default()
            };
            let ds_conv = nn::conv2d(
                vs / "downsample" / "0",
                in_channels,
                out_channels,
                1,
                ds_conv_config,
            );
            let ds_bn = nn::batch_norm2d(vs / "downsample" / "1", out_channels, Default::default());
            Some((ds_conv, ds_bn))
        } else {
            None
        };

        BasicBlock {
            conv1,
            bn1,
            conv2,
            bn2,
            downsample,
        }
    }

    pub fn forward(&self, x: &tch::Tensor) -> tch::Tensor {
        let identity = if let Some((ds_conv, ds_bn)) = &self.downsample {
            // Downsample residual connection
            let mut residual = x.apply(ds_conv);
            residual = residual.apply_t(ds_bn, false); // train=false for inference
            residual
        } else {
            x.shallow_clone()
        };

        // Main path
        let mut out = x.apply(&self.conv1);
        out = out.apply_t(&self.bn1, false); // train=false for inference
        out = out.relu();

        out = out.apply(&self.conv2);
        out = out.apply_t(&self.bn2, false); // train=false for inference

        // Add residual
        out += identity;
        out.relu()
    }
}

/// Create VarStore structure for a ResNet BasicBlock (DEPRECATED - use BasicBlock::new instead)
///
/// This creates the paths in VarStore so weights can load, without implementing forward pass yet
#[allow(
    dead_code,
    reason = "DEPRECATED - kept for reference during TableFormer development"
)]
fn create_basic_block_structure(
    vs: &nn::Path,
    in_channels: i64,
    out_channels: i64,
    _stride: i64,
    with_downsample: bool,
) {
    let conv_config = nn::ConvConfig {
        stride: 1,
        padding: 1,
        bias: false,
        ..Default::default()
    };

    // Create conv/bn layers (weights will be loaded from .pt file)
    let _conv1 = nn::conv2d(vs / "conv1", in_channels, out_channels, 3, conv_config);
    let _bn1 = nn::batch_norm2d(vs / "bn1", out_channels, Default::default());
    let _conv2 = nn::conv2d(vs / "conv2", out_channels, out_channels, 3, conv_config);
    let _bn2 = nn::batch_norm2d(vs / "bn2", out_channels, Default::default());

    // Downsample branch (if needed)
    if with_downsample {
        let ds_conv_config = nn::ConvConfig {
            stride: 1,
            padding: 0,
            bias: false,
            ..Default::default()
        };
        let _ds_conv = nn::conv2d(
            vs / "downsample" / "0",
            in_channels,
            out_channels,
            1,
            ds_conv_config,
        );
        let _ds_bn = nn::batch_norm2d(vs / "downsample" / "1", out_channels, Default::default());
    }
}

/// Transformer Encoder Layer
///
/// Standard transformer encoder layer:
/// 1. Multi-head self-attention
/// 2. Add & Norm (residual + layer norm)
/// 3. Feedforward (linear → ReLU → linear)
/// 4. Add & Norm (residual + layer norm)
pub struct TransformerEncoderLayer {
    // Multi-head attention weights
    in_proj_weight: tch::Tensor,
    in_proj_bias: tch::Tensor,
    out_proj: nn::Linear,

    // Feedforward network
    linear1: nn::Linear,
    linear2: nn::Linear,

    // Layer norms
    norm1: nn::LayerNorm,
    norm2: nn::LayerNorm,

    // Hyperparameters
    d_model: i64,
    nhead: i64,
}

impl std::fmt::Debug for TransformerEncoderLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformerEncoderLayer")
            .field("d_model", &self.d_model)
            .field("nhead", &self.nhead)
            .finish()
    }
}

impl TransformerEncoderLayer {
    pub fn new(vs: &nn::Path, d_model: i64, nhead: i64, dim_feedforward: i64) -> Self {
        // Load attention weights
        let self_attn_path = vs / "self_attn";
        let in_proj_weight = self_attn_path.var(
            "in_proj_weight",
            &[d_model * 3, d_model],
            nn::Init::Uniform { lo: -0.1, up: 0.1 },
        );
        let in_proj_bias = self_attn_path.var("in_proj_bias", &[d_model * 3], nn::Init::Const(0.0));
        let out_proj = nn::linear(
            &(self_attn_path / "out_proj"),
            d_model,
            d_model,
            Default::default(),
        );

        // Feedforward network
        let linear1 = nn::linear(
            &(vs / "linear1"),
            d_model,
            dim_feedforward,
            Default::default(),
        );
        let linear2 = nn::linear(
            &(vs / "linear2"),
            dim_feedforward,
            d_model,
            Default::default(),
        );

        // Layer norms
        let norm1 = nn::layer_norm(&(vs / "norm1"), vec![d_model], Default::default());
        let norm2 = nn::layer_norm(&(vs / "norm2"), vec![d_model], Default::default());

        TransformerEncoderLayer {
            in_proj_weight,
            in_proj_bias,
            out_proj,
            linear1,
            linear2,
            norm1,
            norm2,
            d_model,
            nhead,
        }
    }

    /// Multi-head self-attention
    ///
    /// Input: (seq_len, batch, d_model)
    /// Output: (seq_len, batch, d_model)
    fn multi_head_attention(&self, x: &tch::Tensor, mask: Option<&tch::Tensor>) -> tch::Tensor {
        let (seq_len, batch, _) = x.size3().unwrap();
        let head_dim = self.d_model / self.nhead;

        // Linear projection to Q, K, V
        // in_proj does: x @ W^T + b where W is [3*d_model, d_model]
        let qkv = x.matmul(&self.in_proj_weight.tr()) + &self.in_proj_bias;

        // Split into Q, K, V
        let chunks = qkv.chunk(3, -1);
        let q = &chunks[0]; // (seq_len, batch, d_model)
        let k = &chunks[1];
        let v = &chunks[2];

        // Reshape for multi-head attention
        // (seq_len, batch, d_model) → (seq_len, batch, nhead, head_dim) → (batch, nhead, seq_len, head_dim)
        let q = q
            .view([seq_len, batch, self.nhead, head_dim])
            .permute([1, 2, 0, 3]);
        let k = k
            .view([seq_len, batch, self.nhead, head_dim])
            .permute([1, 2, 0, 3]);
        let v = v
            .view([seq_len, batch, self.nhead, head_dim])
            .permute([1, 2, 0, 3]);

        // Scaled dot-product attention
        // Q @ K^T / sqrt(head_dim)
        let scale = (head_dim as f64).sqrt();
        let mut attn_weights = q.matmul(&k.transpose(-2, -1)) / scale;

        // Apply mask if provided
        if let Some(mask_tensor) = mask {
            attn_weights += mask_tensor;
        }

        // Softmax over the last dimension
        attn_weights = attn_weights.softmax(-1, tch::Kind::Float);

        // Apply attention to values
        let attn_output = attn_weights.matmul(&v); // (batch, nhead, seq_len, head_dim)

        // Reshape back: (batch, nhead, seq_len, head_dim) → (seq_len, batch, d_model)
        let attn_output = attn_output.permute([2, 0, 1, 3]).contiguous();
        let attn_output = attn_output.view([seq_len, batch, self.d_model]);

        // Output projection
        attn_output.apply(&self.out_proj)
    }

    /// Forward pass through one encoder layer
    ///
    /// Input: (seq_len, batch, d_model)
    /// Output: (seq_len, batch, d_model)
    pub fn forward(&self, src: &tch::Tensor, mask: Option<&tch::Tensor>) -> tch::Tensor {
        // Self-attention with residual and norm
        let attn_output = self.multi_head_attention(src, mask);
        let mut x = (src + attn_output).apply(&self.norm1);

        // Feedforward with residual and norm
        let ff_output = x.apply(&self.linear1).relu().apply(&self.linear2);
        x = (&x + ff_output).apply(&self.norm2);

        x
    }
}

/// Transformer Encoder (stack of encoder layers)
pub struct TransformerEncoder {
    layers: Vec<TransformerEncoderLayer>,
}

impl std::fmt::Debug for TransformerEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformerEncoder")
            .field("num_layers", &self.layers.len())
            .finish()
    }
}

impl TransformerEncoder {
    pub fn new(
        vs: &nn::Path,
        num_layers: i64,
        d_model: i64,
        nhead: i64,
        dim_feedforward: i64,
    ) -> Self {
        let mut layers = Vec::new();
        for i in 0..num_layers {
            let layer_path = vs / "layers" / i;
            layers.push(TransformerEncoderLayer::new(
                &layer_path,
                d_model,
                nhead,
                dim_feedforward,
            ));
        }

        TransformerEncoder { layers }
    }

    /// Forward pass through all encoder layers
    ///
    /// Input: (seq_len, batch, d_model)
    /// Output: (seq_len, batch, d_model)
    pub fn forward(&self, src: &tch::Tensor, mask: Option<&tch::Tensor>) -> tch::Tensor {
        let mut output = src.shallow_clone();
        for layer in &self.layers {
            output = layer.forward(&output, mask);
        }
        output
    }
}

/// Decoder Cache: Stores previous decoder layer outputs for autoregressive generation
///
/// Shape: (num_layers, seq_len_accumulated, batch, d_model)
/// Each layer stores all previous token outputs for efficient inference
#[allow(
    dead_code,
    reason = "infrastructure for cached autoregressive decoding, not yet integrated"
)]
pub struct DecoderCache {
    /// Cache for each of 6 decoder layers
    /// Each tensor has shape (seq_len_accumulated, batch, d_model)
    layer_caches: Vec<tch::Tensor>,
}

impl std::fmt::Debug for DecoderCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecoderCache")
            .field("num_layers", &self.layer_caches.len())
            .finish()
    }
}

impl DecoderCache {
    /// Create empty cache for num_layers decoder layers
    pub fn new(num_layers: usize) -> Self {
        DecoderCache {
            layer_caches: Vec::with_capacity(num_layers),
        }
    }

    /// Update cache for a specific layer
    /// Concatenates the new output with the existing cache for that layer
    pub fn update(&mut self, layer_idx: usize, new_output: &tch::Tensor) {
        if layer_idx >= self.layer_caches.len() {
            // First time seeing this layer, just store the output
            self.layer_caches.push(new_output.shallow_clone());
        } else {
            // Concatenate with existing cache along sequence dimension (dim=0)
            let concatenated = tch::Tensor::cat(&[&self.layer_caches[layer_idx], new_output], 0);
            self.layer_caches[layer_idx] = concatenated;
        }
    }

    /// Get cache for a specific layer (None if no cache yet)
    #[must_use = "layer cache is retrieved but not used"]
    pub fn get(&self, layer_idx: usize) -> Option<&tch::Tensor> {
        self.layer_caches.get(layer_idx)
    }
}

/// Transformer Decoder Layer
///
/// Key difference from encoder: processes only the last token during inference
/// 1. Self-attention: Query from last token, Key/Value from all tokens (including cache)
/// 2. Add & Norm
/// 3. Cross-attention: Attends to encoder output (memory)
/// 4. Add & Norm
/// 5. Feedforward (linear → ReLU → linear)
/// 6. Add & Norm
///
/// Returns: Output of last token only (1, batch, d_model)
pub struct TransformerDecoderLayer {
    // Self-attention weights (masked, attends to previous tokens)
    self_attn_in_proj_weight: tch::Tensor,
    self_attn_in_proj_bias: tch::Tensor,
    self_attn_out_proj: nn::Linear,

    // Cross-attention weights (attends to encoder output)
    multihead_attn_in_proj_weight: tch::Tensor,
    multihead_attn_in_proj_bias: tch::Tensor,
    multihead_attn_out_proj: nn::Linear,

    // Feedforward network
    linear1: nn::Linear,
    linear2: nn::Linear,

    // Layer norms (3 in decoder vs 2 in encoder)
    norm1: nn::LayerNorm,
    norm2: nn::LayerNorm,
    norm3: nn::LayerNorm,

    // Hyperparameters
    d_model: i64,
    nhead: i64,
}

impl std::fmt::Debug for TransformerDecoderLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformerDecoderLayer")
            .field("d_model", &self.d_model)
            .field("nhead", &self.nhead)
            .finish()
    }
}

impl TransformerDecoderLayer {
    pub fn new(vs: &nn::Path, d_model: i64, nhead: i64, dim_feedforward: i64) -> Self {
        // Self-attention
        let self_attn_path = vs / "self_attn";
        let self_attn_in_proj_weight = self_attn_path.var(
            "in_proj_weight",
            &[d_model * 3, d_model],
            nn::Init::Uniform { lo: -0.1, up: 0.1 },
        );
        let self_attn_in_proj_bias =
            self_attn_path.var("in_proj_bias", &[d_model * 3], nn::Init::Const(0.0));
        let self_attn_out_proj = nn::linear(
            &(self_attn_path / "out_proj"),
            d_model,
            d_model,
            Default::default(),
        );

        // Cross-attention
        let multihead_attn_path = vs / "multihead_attn";
        let multihead_attn_in_proj_weight = multihead_attn_path.var(
            "in_proj_weight",
            &[d_model * 3, d_model],
            nn::Init::Uniform { lo: -0.1, up: 0.1 },
        );
        let multihead_attn_in_proj_bias =
            multihead_attn_path.var("in_proj_bias", &[d_model * 3], nn::Init::Const(0.0));
        let multihead_attn_out_proj = nn::linear(
            &(multihead_attn_path / "out_proj"),
            d_model,
            d_model,
            Default::default(),
        );

        // Feedforward network
        let linear1 = nn::linear(
            &(vs / "linear1"),
            d_model,
            dim_feedforward,
            Default::default(),
        );
        let linear2 = nn::linear(
            &(vs / "linear2"),
            dim_feedforward,
            d_model,
            Default::default(),
        );

        // Layer norms
        let norm1 = nn::layer_norm(&(vs / "norm1"), vec![d_model], Default::default());
        let norm2 = nn::layer_norm(&(vs / "norm2"), vec![d_model], Default::default());
        let norm3 = nn::layer_norm(&(vs / "norm3"), vec![d_model], Default::default());

        TransformerDecoderLayer {
            self_attn_in_proj_weight,
            self_attn_in_proj_bias,
            self_attn_out_proj,
            multihead_attn_in_proj_weight,
            multihead_attn_in_proj_bias,
            multihead_attn_out_proj,
            linear1,
            linear2,
            norm1,
            norm2,
            norm3,
            d_model,
            nhead,
        }
    }

    /// Multi-head self-attention with optimization: only process last token
    ///
    /// Query from last token, Key/Value from all tokens (tgt includes previous outputs from cache)
    ///
    /// tgt: (seq_len, batch, d_model) - all tokens including cache
    /// Returns: (1, batch, d_model) - output for last token only
    fn masked_self_attention(&self, tgt: &tch::Tensor) -> tch::Tensor {
        let (seq_len, batch, _) = tgt.size3().unwrap();
        let head_dim = self.d_model / self.nhead;

        // Extract last token for query
        let tgt_last_tok = tgt.narrow(0, seq_len - 1, 1); // (1, batch, d_model)

        // Project last token to Q
        let q_qkv =
            tgt_last_tok.matmul(&self.self_attn_in_proj_weight.tr()) + &self.self_attn_in_proj_bias;
        let q = q_qkv.narrow(2, 0, self.d_model); // (1, batch, d_model)

        // Project all tokens to K, V
        let kv_qkv = tgt.matmul(&self.self_attn_in_proj_weight.tr()) + &self.self_attn_in_proj_bias;
        let k = kv_qkv.narrow(2, self.d_model, self.d_model); // (seq_len, batch, d_model)
        let v = kv_qkv.narrow(2, self.d_model * 2, self.d_model); // (seq_len, batch, d_model)

        // Reshape for multi-head attention
        let q = q
            .view([1, batch, self.nhead, head_dim])
            .permute([1, 2, 0, 3]); // (batch, nhead, 1, head_dim)
        let k = k
            .view([seq_len, batch, self.nhead, head_dim])
            .permute([1, 2, 0, 3]); // (batch, nhead, seq_len, head_dim)
        let v = v
            .view([seq_len, batch, self.nhead, head_dim])
            .permute([1, 2, 0, 3]); // (batch, nhead, seq_len, head_dim)

        // Scaled dot-product attention
        let scale = (head_dim as f64).sqrt();
        let mut attn_weights = q.matmul(&k.transpose(-2, -1)) / scale; // (batch, nhead, 1, seq_len)

        // Softmax
        attn_weights = attn_weights.softmax(-1, tch::Kind::Float);

        // Apply attention to values
        let attn_output = attn_weights.matmul(&v); // (batch, nhead, 1, head_dim)

        // Reshape back: (batch, nhead, 1, head_dim) → (1, batch, d_model)
        let attn_output = attn_output.permute([2, 0, 1, 3]).contiguous();
        let attn_output = attn_output.view([1, batch, self.d_model]);

        // Output projection
        attn_output.apply(&self.self_attn_out_proj)
    }

    /// Multi-head cross-attention
    ///
    /// tgt: (1, batch, d_model) - last token from decoder
    /// memory: (enc_seq_len, batch, d_model) - encoder output
    /// Returns: (1, batch, d_model)
    fn cross_attention(&self, tgt: &tch::Tensor, memory: &tch::Tensor) -> tch::Tensor {
        let (tgt_len, batch, _) = tgt.size3().unwrap(); // tgt_len should be 1
        let (mem_len, _, _) = memory.size3().unwrap();
        let head_dim = self.d_model / self.nhead;

        // Project tgt to Q
        let q_qkv = tgt.matmul(&self.multihead_attn_in_proj_weight.tr())
            + &self.multihead_attn_in_proj_bias;
        let q = q_qkv.narrow(2, 0, self.d_model); // (1, batch, d_model)

        // Project memory to K, V
        let kv_qkv = memory.matmul(&self.multihead_attn_in_proj_weight.tr())
            + &self.multihead_attn_in_proj_bias;
        let k = kv_qkv.narrow(2, self.d_model, self.d_model); // (mem_len, batch, d_model)
        let v = kv_qkv.narrow(2, self.d_model * 2, self.d_model); // (mem_len, batch, d_model)

        // Reshape for multi-head attention
        let q = q
            .view([tgt_len, batch, self.nhead, head_dim])
            .permute([1, 2, 0, 3]); // (batch, nhead, 1, head_dim)
        let k = k
            .view([mem_len, batch, self.nhead, head_dim])
            .permute([1, 2, 0, 3]); // (batch, nhead, mem_len, head_dim)
        let v = v
            .view([mem_len, batch, self.nhead, head_dim])
            .permute([1, 2, 0, 3]); // (batch, nhead, mem_len, head_dim)

        // Scaled dot-product attention
        let scale = (head_dim as f64).sqrt();
        let mut attn_weights = q.matmul(&k.transpose(-2, -1)) / scale; // (batch, nhead, 1, mem_len)

        // Softmax
        attn_weights = attn_weights.softmax(-1, tch::Kind::Float);

        // Apply attention to values
        let attn_output = attn_weights.matmul(&v); // (batch, nhead, 1, head_dim)

        // Reshape back: (batch, nhead, 1, head_dim) → (1, batch, d_model)
        let attn_output = attn_output.permute([2, 0, 1, 3]).contiguous();
        let attn_output = attn_output.view([1, batch, self.d_model]);

        // Output projection
        attn_output.apply(&self.multihead_attn_out_proj)
    }

    /// Forward pass through one decoder layer
    ///
    /// tgt: (seq_len, batch, d_model) - all tokens including cache
    /// memory: (enc_seq_len, batch, d_model) - encoder output
    /// Returns: (1, batch, d_model) - output for last token only
    pub fn forward(&self, tgt: &tch::Tensor, memory: &tch::Tensor) -> tch::Tensor {
        let (seq_len, _batch, _) = tgt.size3().unwrap();
        let tgt_last_tok = tgt.narrow(0, seq_len - 1, 1); // (1, batch, d_model)

        // Self-attention with residual and norm
        let self_attn_out = self.masked_self_attention(tgt);
        let mut x = (&tgt_last_tok + self_attn_out).apply(&self.norm1);

        // Cross-attention with residual and norm
        let cross_attn_out = self.cross_attention(&x, memory);
        x = (&x + cross_attn_out).apply(&self.norm2);

        // Feedforward with residual and norm
        let ff_output = x.apply(&self.linear1).relu().apply(&self.linear2);
        x = (&x + ff_output).apply(&self.norm3);

        x
    }

    /// Debug version of forward pass with detailed logging
    pub fn forward_debug(&self, tgt: &tch::Tensor, memory: &tch::Tensor) -> tch::Tensor {
        let (seq_len, _batch, _) = tgt.size3().unwrap();
        let tgt_last_tok = tgt.narrow(0, seq_len - 1, 1); // (1, batch, d_model)

        log::debug!("  [1] tgt_last_tok:");
        let first10: Vec<f32> = (0..10)
            .map(|j| tgt_last_tok.double_value(&[0, 0, j]) as f32)
            .collect();
        log::debug!("      First 10: {:?}", first10);

        // Self-attention with residual and norm
        let self_attn_out = self.masked_self_attention(tgt);
        log::debug!("  [2] self_attn_out:");
        let first10: Vec<f32> = (0..10)
            .map(|j| self_attn_out.double_value(&[0, 0, j]) as f32)
            .collect();
        log::debug!("      First 10: {:?}", first10);

        let after_self_attn = &tgt_last_tok + &self_attn_out;
        log::debug!("  [3] after_self_attn (before norm1):");
        let first10: Vec<f32> = (0..10)
            .map(|j| after_self_attn.double_value(&[0, 0, j]) as f32)
            .collect();
        log::debug!("      First 10: {:?}", first10);

        let mut x = after_self_attn.apply(&self.norm1);
        log::debug!("  [3b] after_norm1:");
        let first10: Vec<f32> = (0..10).map(|j| x.double_value(&[0, 0, j]) as f32).collect();
        log::debug!("      First 10: {:?}", first10);

        // Cross-attention with residual and norm
        let cross_attn_out = self.cross_attention(&x, memory);
        log::debug!("  [4] cross_attn_out:");
        let first10: Vec<f32> = (0..10)
            .map(|j| cross_attn_out.double_value(&[0, 0, j]) as f32)
            .collect();
        log::debug!("      First 10: {:?}", first10);

        let after_cross_attn = &x + &cross_attn_out;
        log::debug!("  [5] after_cross_attn (before norm2):");
        let first10: Vec<f32> = (0..10)
            .map(|j| after_cross_attn.double_value(&[0, 0, j]) as f32)
            .collect();
        log::debug!("      First 10: {:?}", first10);

        x = after_cross_attn.apply(&self.norm2);
        log::debug!("  [5b] after_norm2:");
        let first10: Vec<f32> = (0..10).map(|j| x.double_value(&[0, 0, j]) as f32).collect();
        log::debug!("      First 10: {:?}", first10);

        // Feedforward with residual and norm
        let ff_output = x.apply(&self.linear1).relu().apply(&self.linear2);
        log::debug!("  [6] ff_output:");
        let first10: Vec<f32> = (0..10)
            .map(|j| ff_output.double_value(&[0, 0, j]) as f32)
            .collect();
        log::debug!("      First 10: {:?}", first10);

        let after_ff = &x + &ff_output;
        log::debug!("  [7] after_ff (before norm3):");
        let first10: Vec<f32> = (0..10)
            .map(|j| after_ff.double_value(&[0, 0, j]) as f32)
            .collect();
        log::debug!("      First 10: {:?}", first10);

        x = after_ff.apply(&self.norm3);
        log::debug!("  [7b] final_output:");
        let first10: Vec<f32> = (0..10).map(|j| x.double_value(&[0, 0, j]) as f32).collect();
        log::debug!("      First 10: {:?}", first10);

        x
    }
}

/// Transformer Decoder (stack of decoder layers with caching)
#[allow(
    dead_code,
    reason = "TableFormer component - decoder used via transformer forward pass"
)]
pub struct TransformerDecoder {
    layers: Vec<TransformerDecoderLayer>,
}

impl std::fmt::Debug for TransformerDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformerDecoder")
            .field("num_layers", &self.layers.len())
            .finish()
    }
}

impl TransformerDecoder {
    pub fn new(
        vs: &nn::Path,
        num_layers: i64,
        d_model: i64,
        nhead: i64,
        dim_feedforward: i64,
    ) -> Self {
        let mut layers = Vec::new();
        for i in 0..num_layers {
            let layer_path = vs / "layers" / i;
            layers.push(TransformerDecoderLayer::new(
                &layer_path,
                d_model,
                nhead,
                dim_feedforward,
            ));
        }

        TransformerDecoder { layers }
    }

    /// Forward pass through all decoder layers with caching
    ///
    /// Python reference: transformer_rs.py lines 62-74
    ///
    /// tgt: (seq_len, batch, d_model) - target sequence (can be just last token during inference)
    /// memory: (enc_seq_len, batch, d_model) - encoder output
    /// cache: Optional cache from previous decoding steps
    ///
    /// Returns: (output, updated_cache)
    ///   - output: (seq_len_with_cache, batch, d_model) - includes all cached outputs
    ///   - updated_cache: DecoderCache with outputs from all layers
    pub fn forward(
        &self,
        tgt: &tch::Tensor,
        memory: &tch::Tensor,
        cache: Option<&DecoderCache>,
    ) -> (tch::Tensor, DecoderCache) {
        let mut output = tgt.shallow_clone();
        let mut layer_outputs = Vec::new();

        // Debug: Check if this is first iteration (no cache)
        let is_first_iter = cache.is_none();

        for (i, layer) in self.layers.iter().enumerate() {
            // Process current token(s) through the layer
            // Use debug version for layer 0 on first iteration
            let layer_output = if is_first_iter && i == 0 {
                log::debug!("[DECODER-DEBUG] Layer 0 - DETAILED:");
                layer.forward_debug(&output, memory)
            } else {
                layer.forward(&output, memory)
            };

            // Debug: Print layer output for first iteration
            if is_first_iter {
                log::debug!("[DECODER-DEBUG] Layer {} output:", i);
                log::debug!("  Shape: {:?}", layer_output.size());
                let first10: Vec<f32> = (0..10)
                    .map(|j| layer_output.double_value(&[0, 0, j]) as f32)
                    .collect();
                log::debug!("  First 10 values: {:?}", first10);
            }

            // Store this layer's output for building the cache at the end
            layer_outputs.push(layer_output.shallow_clone());

            // If we have a cache from previous steps, concatenate it with layer output
            // This concatenated result becomes input to the NEXT layer
            // Python: output = torch.cat([cache[i], output], dim=0)
            if let Some(prev_cache) = cache {
                if let Some(prev_layer_cache) = prev_cache.get(i) {
                    output = tch::Tensor::cat(&[prev_layer_cache, &layer_output], 0);
                } else {
                    output = layer_output;
                }
            } else {
                output = layer_output;
            }
        }

        // Build the output cache by stacking all layer outputs
        // Python logic:
        //   if cache is not None:
        //       out_cache = torch.cat([cache, torch.stack(tag_cache, dim=0)], dim=1)
        //   else:
        //       out_cache = torch.stack(tag_cache, dim=0)
        //
        // Cache shape: (num_layers, seq_len, batch, d_model)
        let new_cache = if let Some(prev_cache) = cache {
            // Stack the current layer outputs: (num_layers, 1, batch, d_model)
            let stacked_outputs = tch::Tensor::stack(&layer_outputs, 0);

            // Get the previous cache as a stacked tensor
            // We need to convert Vec<Tensor> to a single stacked tensor
            let prev_cache_vec: Vec<_> = (0..self.layers.len())
                .filter_map(|i| prev_cache.get(i).map(|t| t.shallow_clone()))
                .collect();

            if prev_cache_vec.len() == self.layers.len() {
                let prev_cache_stacked = tch::Tensor::stack(&prev_cache_vec, 0);
                // Concatenate along sequence dimension (dim=1)
                // Result: (num_layers, seq_len_accumulated, batch, d_model)
                let concatenated = tch::Tensor::cat(&[&prev_cache_stacked, &stacked_outputs], 1);

                // Convert back to DecoderCache by unstacking
                let mut cache = DecoderCache::new(self.layers.len());
                for i in 0..self.layers.len() {
                    cache.update(i, &concatenated.get(i as i64));
                }
                cache
            } else {
                // Fallback if cache is incomplete
                let mut cache = DecoderCache::new(self.layers.len());
                for (i, out) in layer_outputs.iter().enumerate() {
                    cache.update(i, out);
                }
                cache
            }
        } else {
            // No previous cache, just store current layer outputs
            let mut cache = DecoderCache::new(self.layers.len());
            for (i, out) in layer_outputs.iter().enumerate() {
                cache.update(i, out);
            }
            cache
        };

        (output, new_cache)
    }
}

/// Tag Transformer: Generates tag sequence for table structure
///
/// Components:
/// - Embedding layer (13 tags → 512 dims)
/// - Positional encoding (sinusoidal)
/// - Transformer encoder (6 layers, 8 heads)
/// - Transformer decoder (6 layers, 8 heads, with caching)
/// - Output linear layer (512 → 13 tags)
pub struct TagTransformer {
    pub input_filter: InputFilter,
    embedding: nn::Embedding,
    positional_encoding: Tensor,
    pub encoder: TransformerEncoder,
    decoder: TransformerDecoder,
    fc: nn::Linear,
    #[allow(
        dead_code,
        reason = "stored for potential debugging/logging of model configuration"
    )]
    nhead: i64,
}

impl std::fmt::Debug for TagTransformer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TagTransformer")
            .field("nhead", &self.nhead)
            .field("encoder", &self.encoder)
            .field("decoder", &self.decoder)
            .finish()
    }
}

impl TagTransformer {
    pub fn new(
        vs: &nn::Path,
        hidden_dim: i64,
        vocab_size: i64,
        enc_layers: i64,
        dec_layers: i64,
        nheads: i64,
    ) -> Self {
        let path = vs / "_tag_transformer";

        // Input filter (256→512)
        let input_filter = InputFilter::new(vs, "_tag_transformer");

        // Embedding: vocab_size=13, hidden_dim=512
        let embedding = nn::embedding(
            &(path.clone() / "_embedding"),
            vocab_size,
            hidden_dim,
            Default::default(),
        );

        // Positional encoding - load from weights
        let positional_encoding = (path.clone() / "_positional_encoding").var(
            "pe",
            &[1024, 1, hidden_dim],
            nn::Init::Const(0.0),
        );

        // Transformer encoder
        let encoder_path = path.clone() / "_encoder";
        let encoder = TransformerEncoder::new(&encoder_path, enc_layers, hidden_dim, nheads, 1024);

        // Transformer decoder
        let decoder_path = path.clone() / "_decoder";
        let decoder = TransformerDecoder::new(&decoder_path, dec_layers, hidden_dim, nheads, 1024);

        // Output linear layer
        let fc = nn::linear(&(path / "_fc"), hidden_dim, vocab_size, Default::default());

        TagTransformer {
            input_filter,
            embedding,
            positional_encoding,
            encoder,
            decoder,
            fc,
            nhead: nheads,
        }
    }

    /// Apply positional encoding
    ///
    /// Input: (seq_len, batch, d_model)
    /// Output: (seq_len, batch, d_model)
    fn apply_positional_encoding(&self, x: &tch::Tensor) -> tch::Tensor {
        let seq_len = x.size()[0];
        let pe = self.positional_encoding.narrow(0, 0, seq_len);
        x + pe
    }

    /// Generate tag sequence using autoregressive decoding
    ///
    /// This implements the autoregressive loop from tablemodel04_rs.py predict() method.
    ///
    /// encoder_output: (seq_len, batch, d_model) - output from transformer encoder
    /// max_steps: maximum sequence length (default 1024)
    ///
    /// Returns: (tag_sequence, tag_h, bboxes_to_merge)
    ///   - tag_sequence: `Vec<i64>` - predicted tag sequence (indices)
    ///   - tag_h: `Vec<Tensor>` - decoder outputs for BBox decoder
    ///   - bboxes_to_merge: `HashMap<usize, usize>` - maps start_index → end_index for horizontal spans
    ///
    /// Reference: tablemodel04_rs.py lines 110-328
    pub fn generate_tag_sequence(
        &self,
        encoder_output: &tch::Tensor,
        max_steps: i64,
    ) -> (
        Vec<i64>,
        Vec<Tensor>,
        std::collections::HashMap<usize, usize>,
    ) {
        // OTSL tag constants
        const TAG_START: i64 = 2;
        const TAG_END: i64 = 3;

        let device = encoder_output.device();

        // Start with <start> token
        // Shape: (1, 1) - one token, batch size 1
        let mut decoded_tags = Tensor::from_slice(&[TAG_START])
            .to_kind(Kind::Int64)
            .to(device)
            .unsqueeze(1); // (1, 1)

        // Output includes <start> token (matches Python baseline)
        let mut output_tags = vec![TAG_START];
        let mut cache: Option<DecoderCache> = None;
        let mut tag_h = Vec::new(); // Decoder outputs for BBox decoder

        // Python's skip_next_tag logic (tablemodel04_rs.py:168)
        // Python initializes skip_next_tag = True to skip the FIRST cell token
        // Then sets it to False after, so all SUBSEQUENT cell tokens are saved
        // This results in 63 saved cells (skipping the first ched)
        let mut skip_next_tag = true;

        // Python's lcel tracking (tablemodel04_rs.py:173)
        // first_lcel tracks whether we're at the beginning of a horizontal span
        let mut first_lcel = true;

        // BBox merging tracking (Python tablemodel04_rs.py:172-177, 238-253)
        // Maps start_index → end_index for horizontal cell spans (lcel sequences)
        let mut bboxes_to_merge: std::collections::HashMap<usize, usize> =
            std::collections::HashMap::new();
        let mut cur_bbox_ind: isize = -1; // Current bbox span start (-1 means not in span)
        let mut bbox_ind: usize = 0; // Current bbox index

        // Autoregressive loop
        // Each iteration generates one token based on all previous tokens
        for _step in 0..max_steps {
            // Step a: Embed current token(s)
            // embedding: (seq_len, batch, d_model)
            let decoded_embedding = self.embedding.forward(&decoded_tags);

            // Debug: Print embedding output for first iteration (before positional encoding)
            if _step == 0 {
                log::debug!("\n[ITER0-DEBUG] Step-by-step intermediate outputs:");
                log::debug!("  (a) Embedding (before pos enc):");
                log::debug!("      Shape: {:?}", decoded_embedding.size());
                let emb_first5: Vec<f32> = (0..5)
                    .map(|i| decoded_embedding.double_value(&[0, 0, i]) as f32)
                    .collect();
                log::debug!("      First 5 values: {:?}", emb_first5);
            }

            let decoded_embedding = self.apply_positional_encoding(&decoded_embedding);

            // Debug: Print embedding output after positional encoding
            if _step == 0 {
                log::debug!("  (b) Embedding (after pos enc):");
                log::debug!("      Shape: {:?}", decoded_embedding.size());
                let emb_pos_first5: Vec<f32> = (0..5)
                    .map(|i| decoded_embedding.double_value(&[0, 0, i]) as f32)
                    .collect();
                log::debug!("      First 5 values: {:?}", emb_pos_first5);
            }

            // Step b: Run decoder with cache
            // The decoder uses cached outputs from previous steps to avoid recomputation
            // decoded: (seq_len_accumulated, batch, d_model)
            let (decoded, new_cache) =
                self.decoder
                    .forward(&decoded_embedding, encoder_output, cache.as_ref());
            cache = Some(new_cache);

            // Debug: Print decoder output
            if _step == 0 {
                log::debug!("  (c) Decoder output:");
                log::debug!("      Shape: {:?}", decoded.size());
                let dec_first5: Vec<f32> = (0..5)
                    .map(|i| decoded.double_value(&[0, 0, i]) as f32)
                    .collect();
                log::debug!("      First 5 values (at position 0): {:?}", dec_first5);
            }

            // Step c: Predict next token
            // Use LAST token output: decoded[-1, :, :]
            // This is critical - we only use the output for the newest token
            let last_token = decoded.get(-1); // (batch, d_model) = (1, 512)

            // Debug: Print FC input (last decoder token)
            if _step == 0 {
                log::debug!("  (d) FC input (last decoder token):");
                log::debug!("      Shape: {:?}", last_token.size());
                let fc_in_first5: Vec<f32> = (0..5)
                    .map(|i| last_token.double_value(&[0, i]) as f32)
                    .collect();
                log::debug!("      First 5 values: {:?}", fc_in_first5);
            }

            let logits = self.fc.forward(&last_token); // (1, vocab_size)

            // Greedy decoding: take the highest scoring token
            let new_tag = logits.argmax(1, false).int64_value(&[0]);

            // Save decoder output for BBox decoder
            // Python has TWO places where it saves decoder outputs (tablemodel04_rs.py):
            // 1. Lines 243-253: Special handling for lcel (horizontal span)
            //    - Save FIRST lcel in a sequence (line 248)
            //    - Mark as start of horizontal span for merging (line 250)
            // 2. Lines 225-240: Skip_next_tag logic for other tokens
            //    - Save tokens when skip_next_tag==False AND token in list
            //    - Update merge end index if after lcel (line 238-240)

            const LCEL: i64 = 6;
            let mut should_save = false;

            // FIRST: Handle lcel special case (Python lines 243-253)
            // This happens BEFORE the skip_next_tag check
            if new_tag == LCEL {
                // This is an lcel token
                if first_lcel {
                    // This is the FIRST lcel in a horizontal span
                    // Python saves this (line 248: tag_H_buf.append(decoded[-1, :, :]))
                    should_save = true;
                    first_lcel = false;

                    // Mark this bbox as the START of a horizontal span (Python line 250)
                    // bboxes_to_merge[bbox_ind] = -1  (placeholder, will be updated when we see the end)
                    cur_bbox_ind = bbox_ind as isize;
                    bboxes_to_merge.insert(bbox_ind, usize::MAX); // usize::MAX represents -1 (to be updated)
                }
            }

            // SECOND: Handle other tokens with skip_next_tag logic (Python lines 225-240)
            if !should_save && !skip_next_tag {
                // Python line 226-234: token in [fcel, ecel, ched, rhed, srow, nl, ucel]
                const SAVEABLE_TOKENS: &[i64] = &[5, 4, 10, 11, 12, 9, 7]; // fcel, ecel, ched, rhed, srow, nl, ucel
                if SAVEABLE_TOKENS.contains(&new_tag) {
                    should_save = true;

                    // If we just finished an lcel sequence, mark this bbox as the END (Python lines 238-240)
                    // if first_lcel is not True: bboxes_to_merge[cur_bbox_ind] = bbox_ind
                    if !first_lcel && cur_bbox_ind >= 0 {
                        bboxes_to_merge.insert(cur_bbox_ind as usize, bbox_ind);
                        cur_bbox_ind = -1; // Reset after updating
                    }
                }
            }

            // THIRD: Reset first_lcel if we're NOT on an lcel token (Python line 244)
            // This must happen AFTER the merge map update above
            if new_tag != LCEL {
                first_lcel = true;
            }

            // Save the decoder output if needed
            if should_save {
                // We need to reshape last_token from (1, 512) to (1, 1, 512) for BBox decoder
                let cell_output = last_token.unsqueeze(1); // (1, 512) → (1, 1, 512)

                // Debug: Print first 3 values of first 3 cells
                if tag_h.len() < 3 {
                    let first3: Vec<f32> = (0..3)
                        .map(|i| cell_output.double_value(&[0, 0, i]) as f32)
                        .collect();
                    log::debug!(
                        "    [DEBUG] Saving cell {} (step {}, token {}): first 3 vals = {:?}",
                        tag_h.len(),
                        _step,
                        new_tag,
                        first3
                    );
                }

                tag_h.push(cell_output);
                bbox_ind += 1; // Increment bbox index after saving
            }

            // Update skip_next_tag based on current token (Python line 255-258)
            // nl(9), ucel(7), xcel(8) set skip_next_tag=True
            // All other tokens set skip_next_tag=False
            skip_next_tag = new_tag == 9 || new_tag == 7 || new_tag == 8;

            // DEBUG: Log all tokens and which ones are saved
            let token_names = [
                "<pad>", "<unk>", "<start>", "<end>", "ecel", "fcel", "lcel", "ucel", "xcel", "nl",
                "ched", "rhed", "srow",
            ];
            let token_name = if (0..13).contains(&new_tag) {
                token_names[new_tag as usize]
            } else {
                "???"
            };
            let save_mark = if should_save { "✓ SAVE" } else { "" };
            let position = _step + 1; // Position in sequence (START is position 0, this is position 1+)
            log::debug!(
                "Step {:2}: Predict position {:2} → {:8} (token {}) {}",
                _step,
                position,
                token_name,
                new_tag,
                save_mark
            );

            // Debug: Print first iteration logits
            if _step == 0 {
                log::debug!("  (e) FC output (logits):");
                log::debug!("      Shape: {:?}", logits.size());
                let logits_vec: Vec<f32> = (0..13)
                    .map(|i| logits.double_value(&[0, i]))
                    .map(|v| v as f32)
                    .collect();
                log::debug!("      Logits: {:?}", logits_vec);
                let top5 = logits.topk(5, 1, true, true);
                let (top5_values, top5_indices) = (top5.0, top5.1);
                log::debug!("  Top 5 predictions:");
                for i in 0..5 {
                    let idx = top5_indices.int64_value(&[0, i as i64]);
                    let val = top5_values.double_value(&[0, i as i64]);
                    log::debug!("    {} (token {}) → {:.4}", i + 1, idx, val);
                }
                log::debug!("  Predicted: {} (argmax)\n", new_tag);
            }

            // Step d: Structure error correction (optional)
            // Python has corrections at lines 198-208 to fix invalid table structures
            // For Phase 1, we'll skip this and see if we need it
            // let new_tag = self.apply_structure_correction(new_tag, &output_tags);

            // Step e: Check for end token
            if new_tag == TAG_END {
                output_tags.push(new_tag);
                break;
            }

            output_tags.push(new_tag);

            // Step f: Append new token to decoded_tags for next iteration
            let new_token_tensor = Tensor::from_slice(&[new_tag])
                .to_kind(Kind::Int64)
                .to(device)
                .unsqueeze(1); // (1, 1)

            // Concatenate along sequence dimension (dim 0)
            decoded_tags = tch::Tensor::cat(&[&decoded_tags, &new_token_tensor], 0);
        }

        log::debug!("\n=== Tag Sequence Generation Complete ===");
        log::debug!("Total tags generated: {}", output_tags.len());
        log::debug!("Total cell tokens saved in tag_h: {}", tag_h.len());
        log::debug!("BBox merge map size: {}", bboxes_to_merge.len());
        log::debug!("Expected after merging: 63 cells");
        log::debug!("=========================================\n");

        (output_tags, tag_h, bboxes_to_merge)
    }
}

/// BBox Decoder: Predicts bounding boxes for each cell
///
/// Components:
/// - Input filter (256→512)
/// - CellAttention module
/// - init_h: Linear(512, 512)
/// - f_beta: Linear(512, 512)
/// - class_embed: Linear(512, 3) - 3 classes (span, row header, col header)
/// - bbox_embed: MLP(512→256→256→4)
pub struct BBoxDecoder {
    input_filter: InputFilter,
    attention: CellAttention,
    init_h: nn::Linear,
    f_beta: nn::Linear,
    class_embed: nn::Linear,
    bbox_embed: MLP,
}

impl std::fmt::Debug for BBoxDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BBoxDecoder")
            .field("input_filter", &self.input_filter)
            .field("attention", &"<CellAttention>")
            .field("bbox_embed", &"<MLP>")
            .finish()
    }
}

impl BBoxDecoder {
    pub fn new(vs: &nn::Path, hidden_dim: i64) -> Self {
        let path = vs / "_bbox_decoder";

        // Input filter (256→512)
        let input_filter = InputFilter::new(vs, "_bbox_decoder");

        // CellAttention
        let attention = CellAttention::new(&(path.clone() / "_attention"), hidden_dim);

        // Linear layers
        let init_h = nn::linear(
            &(path.clone() / "_init_h"),
            hidden_dim,
            hidden_dim,
            Default::default(),
        );
        let f_beta = nn::linear(
            &(path.clone() / "_f_beta"),
            hidden_dim,
            hidden_dim,
            Default::default(),
        );
        let class_embed = nn::linear(
            &(path.clone() / "_class_embed"),
            hidden_dim,
            3,
            Default::default(),
        );

        // MLP for bbox prediction: 512→256→256→4
        let bbox_embed = MLP::new(&(path / "_bbox_embed"), &[hidden_dim, 256, 256, 4]);

        BBoxDecoder {
            input_filter,
            attention,
            init_h,
            f_beta,
            class_embed,
            bbox_embed,
        }
    }

    /// BBox decoder inference
    ///
    /// # Arguments
    /// * `encoder_out` - Encoder output in BHWC format: (1, 28, 28, 256)
    /// * `tag_h` - List of decoder outputs for each cell: Vec<(1, 1, 512)>
    ///
    /// # Returns
    /// * (class_logits, bbox_coords) where:
    ///   - class_logits: (num_cells, 3) - class logits for each cell
    ///   - bbox_coords: (num_cells, 4) - normalized bbox coordinates [cx, cy, w, h]
    pub fn inference(&self, encoder_out: &Tensor, tag_h: &[Tensor]) -> (Tensor, Tensor) {
        // 1. Apply input_filter: (1, 28, 28, 256) → (1, 512, 28, 28) → (1, 28, 28, 512)
        let encoder_bchw = encoder_out.permute([0, 3, 1, 2]); // BHWC → BCHW
        let filtered = self.input_filter.forward(&encoder_bchw);
        let encoder_filtered = filtered.permute([0, 2, 3, 1]); // BCHW → BHWC

        // 2. Flatten to (1, 784, 512)
        let encoder_dim = encoder_filtered.size()[3];
        let encoder_flat = encoder_filtered.view([1, -1, encoder_dim]); // (1, 784, 512)

        let num_cells = tag_h.len();
        let mut predictions_bboxes = Vec::with_capacity(num_cells);
        let mut predictions_classes = Vec::with_capacity(num_cells);

        // 3. For each cell
        for cell_tag_h in tag_h {
            // Initialize hidden state from mean encoder output
            let mean_encoder = encoder_flat.mean_dim(&[1i64][..], false, Kind::Float); // (1, 512)
            let h = mean_encoder.apply(&self.init_h); // (1, 512)

            // Squeeze cell_tag_h from (1, 1, 512) to (1, 512)
            let cell_tag_h_squeezed = cell_tag_h.squeeze_dim(1); // (1, 512)

            // Attention over encoder features
            let (awe, _alpha) = self
                .attention
                .forward(&encoder_flat, &cell_tag_h_squeezed, &h);

            // Gating mechanism
            let gate = h.apply(&self.f_beta).sigmoid(); // (1, 512)
            let awe_gated = &gate * &awe; // (1, 512)
            let h_updated = &awe_gated * &h; // (1, 512)

            // Predict bbox and class
            let bbox = self.bbox_embed.forward(&h_updated).sigmoid(); // (1, 4)
            let class_logits = h_updated.apply(&self.class_embed); // (1, 3)

            predictions_bboxes.push(bbox);
            predictions_classes.push(class_logits);
        }

        // 4. Stack results
        // Each prediction is (1, 3) or (1, 4), we need to stack them to (num_cells, 3) or (num_cells, 4)
        let class_logits = if !predictions_classes.is_empty() {
            Tensor::cat(&predictions_classes.iter().collect::<Vec<_>>(), 0) // (num_cells, 3)
        } else {
            Tensor::empty([0, 3], (Kind::Float, encoder_out.device()))
        };

        let bbox_coords = if !predictions_bboxes.is_empty() {
            Tensor::cat(&predictions_bboxes.iter().collect::<Vec<_>>(), 0) // (num_cells, 4)
        } else {
            Tensor::empty([0, 4], (Kind::Float, encoder_out.device()))
        };

        (class_logits, bbox_coords)
    }
}
