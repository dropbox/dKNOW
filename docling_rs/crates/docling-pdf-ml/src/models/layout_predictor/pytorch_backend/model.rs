// RT-DETR v2 Full Model Implementation
// Ported from transformers/models/rt_detr_v2/modeling_rt_detr_v2.py:1350-1998

// Debug logging macro - disabled by default for performance
// To enable: Uncomment the macro below
// macro_rules! debug_log { ($($arg:tt)*) => { log::warn!($($arg)*) }; }
macro_rules! debug_log {
    ($($arg:tt)*) => {
        ()
    };
}

// Profiling macro - disabled by default for performance
// To enable: Set environment variable PROFILE_MODEL=1
macro_rules! profile_log { ($($arg:tt)*) => {
    if std::env::var("PROFILE_MODEL").is_ok() {
        log::warn!($($arg)*)
    }
}; }

use log::trace;
use std::time::Instant;
use tch::{nn, nn::ModuleT, IndexOp, Tensor};

use super::decoder::{self as decoder_module, RTDetrV2Decoder, RTDetrV2MLPPredictionHead};
use super::encoder::{self as encoder_module, RTDetrV2HybridEncoder};
use super::resnet::ResNetBackbone;
use super::transformer;

/// Type alias for model results
pub type Result<T> = std::result::Result<T, String>;

/// Configuration for RT-DETR v2 model
/// Python: RTDetrV2Config
#[derive(Debug, Clone, PartialEq)]
pub struct RTDetrV2Config {
    // Image and architecture
    pub num_channels: i64,
    pub num_labels: i64,
    pub d_model: i64,

    // Backbone (ResNet)
    pub backbone_embedding_size: i64,
    pub backbone_hidden_sizes: Vec<i64>,
    pub backbone_depths: Vec<i64>,

    // Encoder
    pub encoder_hidden_dim: i64,
    pub encoder_layers: i64,
    pub encoder_attention_heads: i64,
    pub encoder_ffn_dim: i64,
    pub num_feature_levels: i64,
    pub encode_proj_layers: Vec<i64>, // Which feature levels have transformer encoders

    // Decoder
    pub decoder_layers: i64,
    pub decoder_attention_heads: i64,
    pub decoder_ffn_dim: i64,
    pub decoder_n_levels: i64,
    pub decoder_n_points: i64,

    // Queries
    pub num_queries: i64,

    // Dropout
    pub dropout: f64,
    pub attention_dropout: f64,
    pub activation_dropout: f64,

    // Layer norm
    pub layer_norm_eps: f64,

    // Deformable attention
    pub offset_scale: f64,
    pub method: String,

    // Activation functions
    pub activation_function: transformer::Activation,
    pub encoder_activation_function: transformer::Activation,
    pub decoder_activation_function: transformer::Activation,
}

impl Default for RTDetrV2Config {
    #[inline]
    fn default() -> Self {
        Self {
            num_channels: 3,
            num_labels: 80,
            d_model: 256,

            // ResNet-50 configuration
            backbone_embedding_size: 64,
            backbone_hidden_sizes: vec![256, 512, 1024, 2048],
            backbone_depths: vec![3, 4, 6, 3],

            encoder_hidden_dim: 256,
            encoder_layers: 1,
            encoder_attention_heads: 8,
            encoder_ffn_dim: 1024,
            num_feature_levels: 3,
            encode_proj_layers: vec![2], // Only feature level 2 has transformer encoder

            decoder_layers: 6,
            decoder_attention_heads: 8,
            decoder_ffn_dim: 1024,
            decoder_n_levels: 3,
            decoder_n_points: 4,

            num_queries: 300,

            dropout: 0.0,
            attention_dropout: 0.0,
            activation_dropout: 0.0,

            layer_norm_eps: 1e-5,

            offset_scale: 0.5, // FIXED: Was 2.0 (4x too large), Python uses 0.5
            method: "default".to_string(),

            activation_function: transformer::Activation::SiLU, // General activation (from config.json)
            encoder_activation_function: transformer::Activation::GELU, // Encoder-specific (from config.json)
            decoder_activation_function: transformer::Activation::ReLU, // Decoder-specific (from config.json)
        }
    }
}

/// Output of RTDetrV2Model forward pass
pub struct RTDetrV2ModelOutput {
    pub last_hidden_state: Tensor,
    pub intermediate_hidden_states: Tensor,
    pub intermediate_logits: Option<Tensor>,
    pub intermediate_reference_points: Tensor,
    pub encoder_memory: Option<Tensor>, // Added for encoder validation
}

impl std::fmt::Debug for RTDetrV2ModelOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RTDetrV2ModelOutput")
            .field(
                "last_hidden_state",
                &format!("{:?}", self.last_hidden_state.size()),
            )
            .field(
                "intermediate_hidden_states",
                &format!("{:?}", self.intermediate_hidden_states.size()),
            )
            .field(
                "intermediate_logits",
                &self
                    .intermediate_logits
                    .as_ref()
                    .map(|t| format!("{:?}", t.size())),
            )
            .field(
                "intermediate_reference_points",
                &format!("{:?}", self.intermediate_reference_points.size()),
            )
            .field(
                "encoder_memory",
                &self
                    .encoder_memory
                    .as_ref()
                    .map(|t| format!("{:?}", t.size())),
            )
            .finish()
    }
}

/// Output of RTDetrV2ForObjectDetection
pub struct RTDetrV2ObjectDetectionOutput {
    pub logits: Tensor,     // [batch, num_queries, num_labels]
    pub pred_boxes: Tensor, // [batch, num_queries, 4]
    pub last_hidden_state: Tensor,
    pub intermediate_hidden_states: Tensor,
    pub intermediate_logits: Option<Tensor>,
    pub intermediate_reference_points: Tensor,
}

impl std::fmt::Debug for RTDetrV2ObjectDetectionOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RTDetrV2ObjectDetectionOutput")
            .field("logits", &format!("{:?}", self.logits.size()))
            .field("pred_boxes", &format!("{:?}", self.pred_boxes.size()))
            .field(
                "last_hidden_state",
                &format!("{:?}", self.last_hidden_state.size()),
            )
            .field(
                "intermediate_hidden_states",
                &format!("{:?}", self.intermediate_hidden_states.size()),
            )
            .field(
                "intermediate_logits",
                &self
                    .intermediate_logits
                    .as_ref()
                    .map(|t| format!("{:?}", t.size())),
            )
            .field(
                "intermediate_reference_points",
                &format!("{:?}", self.intermediate_reference_points.size()),
            )
            .finish()
    }
}

/// Projection layer: Conv2d + BatchNorm2d
/// BatchNorm doesn't implement Module trait, so we can't use Sequential
pub struct ProjectionLayer {
    pub conv: nn::Conv2D,
    pub bn: nn::BatchNorm,
}

impl std::fmt::Debug for ProjectionLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionLayer")
            .field("conv", &"<nn::Conv2D>")
            .field("bn", &"<nn::BatchNorm>")
            .finish()
    }
}

impl ProjectionLayer {
    pub fn new(vs: &nn::Path, in_channels: i64, out_channels: i64, kernel_size: i64) -> Self {
        let conv = nn::conv2d(
            &(vs / "0"),
            in_channels,
            out_channels,
            kernel_size,
            nn::ConvConfig {
                bias: false,
                ..Default::default()
            },
        );
        let bn = nn::batch_norm2d(&(vs / "1"), out_channels, Default::default());
        Self { conv, bn }
    }

    pub fn forward(&self, xs: &Tensor) -> Tensor {
        // BatchNorm doesn't implement Module trait, so call forward directly
        let conv_output = xs.apply(&self.conv);

        self.bn.forward_t(&conv_output, false)
    }

    /// Forward with debug output (saves intermediate tensors)
    pub fn forward_debug(&self, xs: &Tensor, layer_name: &str) -> Tensor {
        debug_log!(
            "[PROJ_DEBUG] {} input: {:?}, mean={:.6f}",
            layer_name,
            xs.size(),
            xs.mean(tch::Kind::Float).double_value(&[])
        );

        // Save input if debugging encoder_input_proj_2
        if layer_name.contains("encoder_input_proj_2") {
            Self::save_debug_tensor(xs, &format!("debug_rust_{}_input", layer_name));
        }

        // Conv2d
        let conv_output = xs.apply(&self.conv);
        debug_log!(
            "[PROJ_DEBUG] {} after Conv2d: {:?}, mean={:.6f}",
            layer_name,
            conv_output.size(),
            conv_output.mean(tch::Kind::Float).double_value(&[])
        );

        // Save conv output if debugging encoder_input_proj_2
        if layer_name.contains("encoder_input_proj_2") {
            Self::save_debug_tensor(
                &conv_output,
                &format!("debug_rust_{}_conv_output", layer_name),
            );
        }

        // BatchNorm
        let bn_output = self.bn.forward_t(&conv_output, false);
        debug_log!(
            "[PROJ_DEBUG] {} after BatchNorm: {:?}, mean={:.6f}",
            layer_name,
            bn_output.size(),
            bn_output.mean(tch::Kind::Float).double_value(&[])
        );

        // Save output if debugging encoder_input_proj_2
        if layer_name.contains("encoder_input_proj_2") {
            Self::save_debug_tensor(&bn_output, &format!("debug_rust_{}_output", layer_name));
        }

        bn_output
    }

    fn save_debug_tensor(tensor: &Tensor, name: &str) {
        use ndarray::Array4;
        use std::fs::File;
        use std::io::BufWriter;

        let size = tensor.size();
        if size.len() != 4 {
            return; // Only save 4D tensors
        }

        let (batch, channels, height, width) = (size[0], size[1], size[2], size[3]);
        let tensor_cpu = tensor.to_kind(tch::Kind::Float).to(tch::Device::Cpu);
        let tensor_flat = tensor_cpu.flatten(0, -1);
        let data: Vec<f32> = match Vec::try_from(&tensor_flat) {
            Ok(v) => v,
            Err(_) => return,
        };

        let array = match Array4::from_shape_vec(
            (
                batch as usize,
                channels as usize,
                height as usize,
                width as usize,
            ),
            data,
        ) {
            Ok(a) => a,
            Err(_) => return,
        };

        let path = format!("{}.npy", name);
        match File::create(&path) {
            Ok(file) => {
                let writer = BufWriter::new(file);
                match ndarray_npy::WriteNpyExt::write_npy(&array, writer) {
                    Ok(_) => debug_log!("[DEBUG] ✓ Saved {} to {}", name, path),
                    Err(e) => debug_log!("[DEBUG] ✗ Failed to write {}: {:?}", name, e),
                }
            }
            Err(e) => debug_log!("[DEBUG] ✗ Failed to create {}: {:?}", path, e),
        }
    }
}

/// RT-DETR v2 base model
/// Python: RTDetrV2Model
pub struct RTDetrV2Model {
    config: RTDetrV2Config,
    pub backbone: ResNetBackbone,
    pub encoder_input_proj: Vec<ProjectionLayer>,
    pub encoder: RTDetrV2HybridEncoder,
    pub enc_output_linear: nn::Linear,
    pub enc_output_layer_norm: nn::LayerNorm,
    pub enc_score_head: nn::Linear,
    pub enc_bbox_head: RTDetrV2MLPPredictionHead,
    pub decoder_input_proj: Vec<ProjectionLayer>,
    pub decoder: RTDetrV2Decoder,
}

impl std::fmt::Debug for RTDetrV2Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RTDetrV2Model")
            .field("config", &self.config)
            .field("backbone", &"<ResNetBackbone>")
            .field(
                "encoder_input_proj",
                &format!("[{} layers]", self.encoder_input_proj.len()),
            )
            .field("encoder", &"<RTDetrV2HybridEncoder>")
            .field(
                "decoder_input_proj",
                &format!("[{} layers]", self.decoder_input_proj.len()),
            )
            .field("decoder", &"<RTDetrV2Decoder>")
            .finish()
    }
}

impl RTDetrV2Model {
    #[must_use = "constructor returns Result that should be handled"]
    pub fn new(vs: &nn::Path, config: RTDetrV2Config) -> Result<Self> {
        // 1. Create backbone (ResNet-50)
        // HF model has extra "model" layer: model.backbone.model.embedder...
        let backbone = ResNetBackbone::new(
            &(vs / "model" / "backbone" / "model"),
            config.num_channels,
            config.backbone_embedding_size,
            &config.backbone_hidden_sizes,
            &config.backbone_depths,
        );

        // 2. Create encoder input projection layers
        // These project backbone features to encoder_hidden_dim
        let num_backbone_outs = 3; // C3, C4, C5 (last 3 stages)
        let intermediate_channel_sizes =
            &config.backbone_hidden_sizes[config.backbone_hidden_sizes.len() - 3..];

        let mut encoder_input_proj = Vec::new();
        for (i, &in_channels) in intermediate_channel_sizes.iter().enumerate() {
            let proj = ProjectionLayer::new(
                &(vs / "model" / "encoder_input_proj" / i.to_string()),
                in_channels,
                config.encoder_hidden_dim,
                1, // kernel_size
            );
            encoder_input_proj.push(proj);
        }

        // 3. Create encoder (RTDetrV2HybridEncoder)
        // Note: RTDetrV2HybridEncoder has a complex API - for now using simplified version
        let encoder_config = encoder_module::EncoderLayerConfig {
            encoder_hidden_dim: config.encoder_hidden_dim,
            num_attention_heads: config.encoder_attention_heads,
            encoder_ffn_dim: config.encoder_ffn_dim,
            dropout: config.dropout,
            activation_dropout: config.activation_dropout,
            encoder_activation_function: config.encoder_activation_function,
            normalize_before: false, // From config.json (normalize_before: false)
            layer_norm_eps: config.layer_norm_eps,
        };

        let encoder = RTDetrV2HybridEncoder::new(
            &(vs / "model" / "encoder"),
            config.encoder_hidden_dim,
            config.encoder_layers,
            config.encode_proj_layers.clone(), // Which feature levels have transformer encoders
            2,                                 // num_fpn_stages
            2,                                 // num_pan_stages
            config.activation_function, // Use general activation for FPN/PAN (lateral_convs, etc.)
            encoder_config,
            10000.0, // positional_encoding_temperature
            None,    // eval_size
        );

        // 4. Create encoder output heads
        let enc_output_linear = nn::linear(
            &(vs / "model" / "enc_output" / "0"),
            config.d_model,
            config.d_model,
            Default::default(),
        );

        let enc_output_layer_norm = nn::layer_norm(
            &(vs / "model" / "enc_output" / "1"),
            vec![config.d_model],
            nn::LayerNormConfig {
                eps: config.layer_norm_eps,
                ..Default::default()
            },
        );

        let enc_score_head = nn::linear(
            &(vs / "model" / "enc_score_head"),
            config.d_model,
            config.num_labels,
            Default::default(),
        );

        let enc_bbox_head = RTDetrV2MLPPredictionHead::new(
            &(vs / "model" / "enc_bbox_head"),
            config.d_model,
            config.d_model,
            4,
            3,
        );

        // 5. Create decoder input projection layers
        let mut decoder_input_proj = Vec::new();
        for i in 0..config.num_feature_levels {
            let proj = ProjectionLayer::new(
                &(vs / "model" / "decoder_input_proj" / i.to_string()),
                config.encoder_hidden_dim,
                config.d_model,
                1, // kernel_size
            );
            decoder_input_proj.push(proj);
        }

        // 6. Create decoder
        // Convert transformer::Activation to decoder::Activation
        let decoder_activation = match config.decoder_activation_function {
            transformer::Activation::ReLU => decoder_module::Activation::ReLU,
            transformer::Activation::GELU => decoder_module::Activation::GELU,
            transformer::Activation::SiLU => {
                log::warn!("Warning: SiLU not supported in decoder, using ReLU");
                decoder_module::Activation::ReLU
            }
        };

        let decoder = RTDetrV2Decoder::new(
            &(vs / "model" / "decoder"),
            config.d_model,
            config.decoder_layers,
            config.decoder_attention_heads,
            config.decoder_ffn_dim,
            config.dropout,
            config.attention_dropout,
            config.activation_dropout,
            decoder_activation,
            config.layer_norm_eps,
            config.decoder_n_levels,
            config.decoder_n_points,
            config.offset_scale,
            &config.method,
        );

        Ok(Self {
            config,
            backbone,
            encoder_input_proj,
            encoder,
            enc_output_linear,
            enc_output_layer_norm,
            enc_score_head,
            enc_bbox_head,
            decoder_input_proj,
            decoder,
        })
    }

    /// Generate anchors for RT-DETR
    /// Python: RTDetrV2Model.generate_anchors()
    ///
    /// # Arguments
    /// * `spatial_shapes` - List of (height, width) tuples for each feature level
    /// * `grid_size` - Base grid size (default 0.05)
    /// * `device` - Device to create tensors on
    ///
    /// # Returns
    /// (anchors, valid_mask) where:
    /// - anchors: [1, total_pixels, 4] - (x, y, w, h) in logit space
    /// - valid_mask: [1, total_pixels, 1] - valid pixel mask
    fn generate_anchors(
        &self,
        spatial_shapes: &[(i64, i64)],
        grid_size: f64,
        device: tch::Device,
    ) -> (Tensor, Tensor) {
        debug_log!(
            "[ANCHORS] Generating anchors for {} levels, grid_size={}",
            spatial_shapes.len(),
            grid_size
        );
        let dtype = tch::Kind::Float;
        let mut anchors_list = Vec::new();

        for (level, &(height, width)) in spatial_shapes.iter().enumerate() {
            debug_log!("[ANCHORS]   Level {}: {}x{}", level, height, width);
            // Create grid coordinates
            debug_log!("[ANCHORS]   Creating arange tensors...");
            let grid_y = Tensor::arange(height, (dtype, device));
            let grid_x = Tensor::arange(width, (dtype, device));
            debug_log!("[ANCHORS]   Arange complete");

            // meshgrid equivalent: grid_y, grid_x
            debug_log!("[ANCHORS]   Creating meshgrid...");
            let grid_y = grid_y.view([height, 1]).repeat([1, width]);
            let grid_x = grid_x.view([1, width]).repeat([height, 1]);

            // Stack [grid_x, grid_y] along last dimension
            debug_log!("[ANCHORS]   Stacking grid_x and grid_y...");
            let grid_xy = Tensor::stack(&[grid_x, grid_y], -1);

            // Add 0.5 offset and normalize
            debug_log!("[ANCHORS]   Adding offset...");
            let grid_xy = grid_xy.unsqueeze(0) + 0.5;

            // Normalize x by width, y by height
            debug_log!("[ANCHORS]   Normalizing...");
            let grid_xy_x = grid_xy.i((.., .., .., 0)) / (width as f64);
            let grid_xy_y = grid_xy.i((.., .., .., 1)) / (height as f64);
            let grid_xy_normalized = Tensor::stack(&[grid_xy_x, grid_xy_y], -1);

            // Create width/height: ones * grid_size * (2^level)
            debug_log!("[ANCHORS]   Creating width/height...");
            let wh =
                Tensor::ones_like(&grid_xy_normalized) * grid_size * f64::powi(2.0, level as i32);

            // Concatenate [x, y, w, h]
            debug_log!("[ANCHORS]   Concatenating anchor...");
            let anchor =
                Tensor::cat(&[grid_xy_normalized, wh], -1).reshape([-1, height * width, 4]);

            debug_log!(
                "[ANCHORS]   Level {} complete, anchor shape: {:?}",
                level,
                anchor.size()
            );
            anchors_list.push(anchor);
        }

        // Concatenate all levels
        debug_log!(
            "[ANCHORS] Concatenating all {} levels...",
            anchors_list.len()
        );
        let anchors = Tensor::cat(&anchors_list, 1);
        debug_log!("[ANCHORS] All anchors concatenated: {:?}", anchors.size());

        // Create valid mask: (anchors > eps) & (anchors < 1 - eps) for all dimensions
        debug_log!("[ANCHORS] Creating valid mask...");
        let eps = 1e-2;
        let valid_mask = (&anchors.gt(eps) * &anchors.lt(1.0 - eps)).all_dim(-1, true);
        debug_log!("[ANCHORS] Valid mask: {:?}", valid_mask.size());

        // Convert to logit space: log(x / (1 - x))
        debug_log!("[ANCHORS] Converting to logit space...");
        let anchors_logit = (&anchors / &(Tensor::from(1.0) - &anchors)).log();
        debug_log!("[ANCHORS] Logit anchors: {:?}", anchors_logit.size());

        // Clamp invalid anchors to max float value
        debug_log!("[ANCHORS] Clamping invalid anchors...");
        let max_val = f32::MAX as f64;
        // Broadcast valid_mask to match anchors_logit shape: [B, N, 1] → [B, N, 4]
        let logit_size = anchors_logit.size();
        let valid_mask_broadcast = valid_mask.broadcast_to(&logit_size[..]);
        debug_log!(
            "[ANCHORS] Broadcasted valid mask: {:?}",
            valid_mask_broadcast.size()
        );
        // Use masked_fill to set invalid anchors to max value
        // Python: torch.where(valid_mask, anchors_logit, max_val)
        // Rust: anchors_logit.masked_fill(&(!valid_mask), max_val)
        let invalid_mask = valid_mask_broadcast.logical_not();
        let anchors_final = anchors_logit.masked_fill(&invalid_mask, max_val);
        debug_log!("[ANCHORS] Final anchors: {:?}", anchors_final.size());

        debug_log!("[ANCHORS] Anchor generation complete");
        (anchors_final, valid_mask)
    }

    /// Forward pass through the model
    ///
    /// # Arguments
    /// * `pixel_values` - Input images [batch_size, 3, height, width]
    ///
    /// # Returns
    /// RTDetrV2ModelOutput with decoder outputs
    #[must_use = "forward pass returns output that should be processed"]
    pub fn forward(&self, pixel_values: &Tensor) -> Result<RTDetrV2ModelOutput> {
        debug_log!("[FORWARD] Starting forward pass");
        let device = pixel_values.device();
        let size = pixel_values.size();
        let batch_size = size[0];
        let height = size[2];
        let width = size[3];
        debug_log!(
            "[FORWARD] Input: batch={}, h={}, w={}",
            batch_size,
            height,
            width
        );

        // 1. Create pixel mask (all ones - no masking)
        let pixel_mask = Tensor::ones([batch_size, height, width], (tch::Kind::Float, device));
        debug_log!("[FORWARD] Created pixel mask");

        // 2. Extract backbone features (multi-scale)
        debug_log!("[FORWARD] Running backbone...");
        let backbone_start = Instant::now();
        let backbone_output = self.backbone.forward(pixel_values);
        let backbone_time = backbone_start.elapsed();
        profile_log!(
            "[PROFILE] Backbone: {:.2} ms",
            backbone_time.as_secs_f64() * 1000.0
        );
        debug_log!(
            "[FORWARD] Backbone complete, {} features",
            backbone_output.1.len()
        );

        // DEBUG: Save backbone outputs for comparison with Python
        use ndarray::Array4;
        use tch::Kind;
        for (i, feat) in backbone_output.1.iter().enumerate() {
            let save_path = format!("debug_rust_backbone_stage_{}.npy", i);

            // Convert to numpy format
            let size = feat.size();
            let (batch, channels, height, width) = (size[0], size[1], size[2], size[3]);

            let tensor_cpu = feat.to_kind(Kind::Float).to(tch::Device::Cpu);
            let tensor_flat = tensor_cpu.flatten(0, -1);
            let data: Vec<f32> = Vec::try_from(&tensor_flat).unwrap();

            let array = Array4::from_shape_vec(
                (
                    batch as usize,
                    channels as usize,
                    height as usize,
                    width as usize,
                ),
                data,
            )
            .unwrap();

            if let Err(e) = ndarray_npy::write_npy(&save_path, &array) {
                log::warn!("Warning: Failed to save {}: {}", save_path, e);
            } else {
                debug_log!("[DEBUG] Saved backbone stage {} to {}", i, save_path);
            }
        }

        // Backbone returns (pooled_output, Vec<feature_maps>)
        // We need the multi-scale feature maps (last 3 stages: C3, C4, C5)
        // Backbone outputs 5 feature maps from stages 0-4, we only use the last 3
        let all_features = &backbone_output.1;
        let features: Vec<&Tensor> = all_features.iter().skip(all_features.len() - 3).collect();

        // 3. Project backbone features to encoder hidden dim
        debug_log!("[FORWARD] Projecting backbone features...");
        debug_log!(
            "[FORWARD] Backbone features (total): {}, Using last 3, Projection layers: {}",
            all_features.len(),
            self.encoder_input_proj.len()
        );
        for (i, feat) in features.iter().enumerate() {
            debug_log!("[FORWARD]   Feature {}: {:?}", i, feat.size());
        }

        // DEBUG: Print backbone feature values (before projection)
        for (i, feat) in features.iter().enumerate() {
            debug_log!(
                "[DEBUG] backbone_feature[{}][0, :5, 0, 0] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
                i,
                feat.double_value(&[0, 0, 0, 0]),
                feat.double_value(&[0, 1, 0, 0]),
                feat.double_value(&[0, 2, 0, 0]),
                feat.double_value(&[0, 3, 0, 0]),
                feat.double_value(&[0, 4, 0, 0]),
            );
        }

        let proj_feats: Vec<Tensor> = features
            .iter()
            .zip(self.encoder_input_proj.iter())
            .enumerate()
            .map(|(i, (feat, proj))| {
                debug_log!(
                    "[FORWARD]   Projecting feature {} with shape {:?}",
                    i,
                    feat.size()
                );

                // Use debug version for encoder_input_proj_2 to capture internals
                let result = if i == 2 && std::env::var("DEBUG_ENCODER_PROJ_2").is_ok() {
                    proj.forward_debug(feat, "encoder_input_proj_2")
                } else {
                    proj.forward(feat)
                };

                debug_log!("[FORWARD]   Projection {} complete: {:?}", i, result.size());
                result
            })
            .collect();
        debug_log!(
            "[FORWARD] Projection complete, {} projected features",
            proj_feats.len()
        );

        // DEBUG: Print projected feature values (encoder inputs)
        for (i, feat) in proj_feats.iter().enumerate() {
            debug_log!(
                "[DEBUG] projected_feature[{}][0, :5, 0, 0] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
                i,
                feat.double_value(&[0, 0, 0, 0]),
                feat.double_value(&[0, 1, 0, 0]),
                feat.double_value(&[0, 2, 0, 0]),
                feat.double_value(&[0, 3, 0, 0]),
                feat.double_value(&[0, 4, 0, 0]),
            );

            // Save encoder_input_2 for comparison (20x20, 256ch)
            if i == 2 {
                use ndarray::Array4;
                let size = feat.size();
                let (batch, channels, height, width) = (size[0], size[1], size[2], size[3]);
                let tensor_cpu = feat.to_kind(tch::Kind::Float).to(tch::Device::Cpu);
                let tensor_flat = tensor_cpu.flatten(0, -1);
                let data: Vec<f32> = Vec::try_from(&tensor_flat).unwrap();
                let array = Array4::from_shape_vec(
                    (
                        batch as usize,
                        channels as usize,
                        height as usize,
                        width as usize,
                    ),
                    data,
                )
                .unwrap();
                if let Err(e) = ndarray_npy::write_npy("debug_rust_encoder_input_2.npy", &array) {
                    debug_log!("[WARNING] Failed to save encoder_input_2: {}", e);
                } else {
                    debug_log!("[DEBUG] Saved encoder_input_2 to debug_rust_encoder_input_2.npy");
                }
            }
        }

        // 4. Run encoder
        debug_log!("[FORWARD] Running encoder...");
        let encoder_start = Instant::now();
        let encoder_outputs = self
            .encoder
            .forward(&proj_feats, false, false)
            .map_err(|e| format!("Encoder forward failed: {}", e))?;
        let encoder_time = encoder_start.elapsed();
        profile_log!(
            "[PROFILE] Encoder: {:.2} ms",
            encoder_time.as_secs_f64() * 1000.0
        );
        debug_log!(
            "[FORWARD] Encoder complete, {} outputs",
            encoder_outputs.len()
        );

        // DEBUG: Save encoder outputs (PAN feature maps) for comparison
        for (i, out) in encoder_outputs.iter().enumerate() {
            let save_path = format!("debug_rust_pan_output_{}.npy", i);
            let size = out.size();
            let (batch, channels, height, width) = (size[0], size[1], size[2], size[3]);
            let tensor_cpu = out.to_kind(tch::Kind::Float).to(tch::Device::Cpu);
            let tensor_flat = tensor_cpu.flatten(0, -1);
            let data: Vec<f32> = Vec::try_from(&tensor_flat).unwrap();
            let array = Array4::from_shape_vec(
                (
                    batch as usize,
                    channels as usize,
                    height as usize,
                    width as usize,
                ),
                data,
            )
            .unwrap();
            if let Err(e) = ndarray_npy::write_npy(&save_path, &array) {
                debug_log!("[WARNING] Failed to save PAN output {}: {}", i, e);
            } else {
                debug_log!("[DEBUG] Saved PAN output {} to {}", i, save_path);
            }
        }

        // 5. Prepare decoder inputs by projecting encoder outputs
        debug_log!("[FORWARD] Projecting encoder outputs...");
        let sources: Vec<Tensor> = encoder_outputs
            .iter()
            .zip(self.decoder_input_proj.iter())
            .enumerate()
            .map(|(i, (source, proj))| {
                debug_log!("[FORWARD]   Encoder output {}: {:?}", i, source.size());
                let result = proj.forward(source);
                debug_log!(
                    "[FORWARD]   Decoder projection {} complete: {:?}",
                    i,
                    result.size()
                );
                result
            })
            .collect();
        debug_log!("[FORWARD] Decoder projection complete");

        // 6. Flatten multi-scale features
        debug_log!("[FORWARD] Flattening multi-scale features...");
        let mut source_flatten_vec = Vec::new();
        let mut spatial_shapes = Vec::new();

        for (i, source) in sources.iter().enumerate() {
            let source_size = source.size();
            let h = source_size[2];
            let w = source_size[3];
            debug_log!(
                "[FORWARD]   Source {}: {:?}, spatial: {}x{}",
                i,
                source.size(),
                h,
                w
            );
            spatial_shapes.push((h, w));

            // Flatten spatial dimensions and transpose
            // flatten(2, 3) flattens dims 2 and 3 (H and W) → [B, C, H*W]
            // transpose(1, 2) swaps dims 1 and 2 → [B, H*W, C]
            debug_log!("[FORWARD]   Flattening source {}...", i);
            let source_flat = source.flatten(2, 3).transpose(1, 2);
            debug_log!("[FORWARD]   Flattened to: {:?}", source_flat.size());
            source_flatten_vec.push(source_flat);
        }

        debug_log!(
            "[FORWARD] Concatenating {} flattened sources along dim 1...",
            source_flatten_vec.len()
        );
        let source_flatten = Tensor::cat(&source_flatten_vec, 1);
        debug_log!("[FORWARD] Concatenated shape: {:?}", source_flatten.size());

        // 7. Generate anchors
        debug_log!("[FORWARD] Generating anchors...");
        let (anchors, valid_mask) = self.generate_anchors(&spatial_shapes, 0.05, device);
        debug_log!(
            "[FORWARD] Anchors: {:?}, Valid mask: {:?}",
            anchors.size(),
            valid_mask.size()
        );

        // 8. Apply valid mask to features
        // Python: memory = valid_mask.to(source_flatten.dtype) * source_flatten
        debug_log!("[FORWARD] Applying valid mask...");
        let valid_mask_float = valid_mask.to_kind(tch::Kind::Float);
        let memory = &valid_mask_float * &source_flatten;
        debug_log!("[FORWARD] Memory: {:?}", memory.size());

        // 9. Encoder output projection: Linear + LayerNorm
        // DEBUG: Print memory (encoder outputs) before transformation
        debug_log!(
            "[DEBUG] memory (before linear)[0, 0, :5] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            f64::try_from(memory.i((0, 0, 0))).unwrap(),
            f64::try_from(memory.i((0, 0, 1))).unwrap(),
            f64::try_from(memory.i((0, 0, 2))).unwrap(),
            f64::try_from(memory.i((0, 0, 3))).unwrap(),
            f64::try_from(memory.i((0, 0, 4))).unwrap(),
        );
        debug_log!(
            "[DEBUG] memory (before linear)[0, 8185, :5] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            f64::try_from(memory.i((0, 8185, 0))).unwrap(),
            f64::try_from(memory.i((0, 8185, 1))).unwrap(),
            f64::try_from(memory.i((0, 8185, 2))).unwrap(),
            f64::try_from(memory.i((0, 8185, 3))).unwrap(),
            f64::try_from(memory.i((0, 8185, 4))).unwrap(),
        );

        debug_log!("[FORWARD] Applying enc_output_linear...");
        let output_memory = memory.apply(&self.enc_output_linear);
        debug_log!("[FORWARD] After linear: {:?}", output_memory.size());
        debug_log!("[FORWARD] Applying enc_output_layer_norm...");
        let output_memory = output_memory.apply(&self.enc_output_layer_norm);
        debug_log!("[FORWARD] After layer_norm: {:?}", output_memory.size());

        // DEBUG: Print output_memory sample values
        debug_log!(
            "[DEBUG] output_memory[0, 0, :5] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            f64::try_from(output_memory.i((0, 0, 0))).unwrap(),
            f64::try_from(output_memory.i((0, 0, 1))).unwrap(),
            f64::try_from(output_memory.i((0, 0, 2))).unwrap(),
            f64::try_from(output_memory.i((0, 0, 3))).unwrap(),
            f64::try_from(output_memory.i((0, 0, 4))).unwrap(),
        );
        debug_log!(
            "[DEBUG] output_memory[0, 8185, :5] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            f64::try_from(output_memory.i((0, 8185, 0))).unwrap(),
            f64::try_from(output_memory.i((0, 8185, 1))).unwrap(),
            f64::try_from(output_memory.i((0, 8185, 2))).unwrap(),
            f64::try_from(output_memory.i((0, 8185, 3))).unwrap(),
            f64::try_from(output_memory.i((0, 8185, 4))).unwrap(),
        );

        // 10. Encoder output heads
        debug_log!("[FORWARD] Applying enc_score_head...");
        let enc_outputs_class = output_memory.apply(&self.enc_score_head);
        debug_log!("[FORWARD] Class scores: {:?}", enc_outputs_class.size());

        // DEBUG: Print enc_outputs_class at specific indices
        debug_log!(
            "[DEBUG] enc_outputs_class[0, 0, :5] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            f64::try_from(enc_outputs_class.i((0, 0, 0))).unwrap(),
            f64::try_from(enc_outputs_class.i((0, 0, 1))).unwrap(),
            f64::try_from(enc_outputs_class.i((0, 0, 2))).unwrap(),
            f64::try_from(enc_outputs_class.i((0, 0, 3))).unwrap(),
            f64::try_from(enc_outputs_class.i((0, 0, 4))).unwrap(),
        );
        debug_log!(
            "[DEBUG] enc_outputs_class[0, 8185, :5] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            f64::try_from(enc_outputs_class.i((0, 8185, 0))).unwrap(),
            f64::try_from(enc_outputs_class.i((0, 8185, 1))).unwrap(),
            f64::try_from(enc_outputs_class.i((0, 8185, 2))).unwrap(),
            f64::try_from(enc_outputs_class.i((0, 8185, 3))).unwrap(),
            f64::try_from(enc_outputs_class.i((0, 8185, 4))).unwrap(),
        );

        debug_log!("[FORWARD] Applying enc_bbox_head...");
        let enc_bbox_out = self.enc_bbox_head.forward(&output_memory);
        debug_log!("[FORWARD] Bbox output: {:?}", enc_bbox_out.size());
        let enc_outputs_coord_logits = enc_bbox_out + &anchors;
        debug_log!(
            "[FORWARD] Coord logits: {:?}",
            enc_outputs_coord_logits.size()
        );

        // 11. Top-k selection (select best num_queries)
        let num_queries = self.config.num_queries;
        debug_log!("[FORWARD] Top-k selection, num_queries={}", num_queries);

        // Get max class score per query
        debug_log!("[FORWARD] Computing max scores...");
        let max_scores = enc_outputs_class.max_dim(-1, false).0;
        debug_log!("[FORWARD] Max scores: {:?}", max_scores.size());

        // DEBUG: Print max_scores at specific indices to compare with Python
        debug_log!("[DEBUG] max_scores at Python topk_ind positions:");
        let python_topk_indices = [
            8185i64, 8353, 7667, 6885, 7486, 7622, 7703, 7507, 8285, 8156,
        ];
        for (i, &idx) in python_topk_indices.iter().enumerate() {
            let score = f64::try_from(max_scores.i((0, idx))).unwrap();
            debug_log!(
                "[DEBUG]   idx {} (Python rank {}): score = {:.6}",
                idx,
                i,
                score
            );
        }

        debug_log!("[FORWARD] Computing topk...");

        // Original topk (no tie-breaking needed - see N=6 investigation)
        // When encoder scores differ by only ~2e-6, topk order may vary
        // This is acceptable accumulated precision, not a bug
        let topk_result = max_scores.topk(num_queries, 1, true, true);
        let topk_ind = topk_result.1;

        debug_log!("[FORWARD] Topk indices: {:?}", topk_ind.size());

        // DEBUG: Print topk_ind sample values
        debug_log!(
            "[DEBUG] topk_ind[0, :10] = [{}, {}, {}, {}, {}, {}, {}, {}, {}, {}]",
            i64::try_from(topk_ind.i((0, 0))).unwrap(),
            i64::try_from(topk_ind.i((0, 1))).unwrap(),
            i64::try_from(topk_ind.i((0, 2))).unwrap(),
            i64::try_from(topk_ind.i((0, 3))).unwrap(),
            i64::try_from(topk_ind.i((0, 4))).unwrap(),
            i64::try_from(topk_ind.i((0, 5))).unwrap(),
            i64::try_from(topk_ind.i((0, 6))).unwrap(),
            i64::try_from(topk_ind.i((0, 7))).unwrap(),
            i64::try_from(topk_ind.i((0, 8))).unwrap(),
            i64::try_from(topk_ind.i((0, 9))).unwrap(),
        );

        // 12. Gather top-k predictions
        debug_log!("[FORWARD] Gathering top-k predictions...");
        debug_log!("[FORWARD] topk_ind: {:?}", topk_ind.size());
        debug_log!("[FORWARD] Expanding topk_ind...");
        let topk_ind_expanded =
            topk_ind
                .unsqueeze(-1)
                .repeat([1, 1, enc_outputs_coord_logits.size()[2]]);
        debug_log!(
            "[FORWARD] topk_ind_expanded: {:?}",
            topk_ind_expanded.size()
        );
        debug_log!("[FORWARD] Gathering coord_logits...");
        let reference_points_unact = enc_outputs_coord_logits.gather(1, &topk_ind_expanded, false);
        debug_log!(
            "[FORWARD] reference_points_unact: {:?}",
            reference_points_unact.size()
        );

        debug_log!("[FORWARD] Applying sigmoid...");
        let enc_topk_bboxes = reference_points_unact.sigmoid();
        debug_log!("[FORWARD] enc_topk_bboxes: {:?}", enc_topk_bboxes.size());

        debug_log!("[FORWARD] Gathering class logits...");
        let topk_ind_class = topk_ind
            .unsqueeze(-1)
            .repeat([1, 1, enc_outputs_class.size()[2]]);
        debug_log!("[FORWARD] topk_ind_class: {:?}", topk_ind_class.size());
        let enc_topk_logits = enc_outputs_class.gather(1, &topk_ind_class, false);
        debug_log!("[FORWARD] enc_topk_logits: {:?}", enc_topk_logits.size());

        // DEBUG: Save intermediate values for target preparation debugging
        // Only save if DEBUG_SAVE_TARGET_PREP env var is set
        if std::env::var("DEBUG_SAVE_TARGET_PREP").is_ok() {
            use ndarray_npy::WriteNpyExt;
            use std::fs::File;
            use std::io::BufWriter;
            use std::path::Path;

            let base_path = "baseline_data/arxiv_2206.01062/page_0/layout/pytorch_intermediate";
            if let Some(parent) = Path::new(base_path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            // Save memory (encoder output before enc_output layers): [1, 8400, 256]
            {
                let shape = memory.size();
                let tensor_flat = memory.flatten(0, -1).to_kind(tch::Kind::Float);
                let tensor_vec: Vec<f32> =
                    Vec::try_from(tensor_flat).expect("Failed to convert memory tensor to Vec");
                let array = ndarray::Array::from_shape_vec(
                    (shape[0] as usize, shape[1] as usize, shape[2] as usize),
                    tensor_vec,
                )
                .expect("Failed to create memory ndarray");
                let output_path = format!("{}/debug_rust_encoder_memory.npy", base_path);
                match File::create(&output_path) {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        match array.write_npy(writer) {
                            Ok(_) => debug_log!("[DEBUG] ✓ Saved memory to: {}", output_path),
                            Err(e) => debug_log!("[DEBUG] ✗ Failed to write memory npy: {:?}", e),
                        }
                    }
                    Err(e) => debug_log!("[DEBUG] ✗ Failed to create memory file: {:?}", e),
                }
            }

            // Save output_memory: [1, 8400, 256]
            {
                let shape = output_memory.size();
                let tensor_flat = output_memory.flatten(0, -1).to_kind(tch::Kind::Float);
                let tensor_vec: Vec<f32> = Vec::try_from(tensor_flat)
                    .expect("Failed to convert output_memory tensor to Vec");
                let array = ndarray::Array::from_shape_vec(
                    (shape[0] as usize, shape[1] as usize, shape[2] as usize),
                    tensor_vec,
                )
                .expect("Failed to create output_memory ndarray");
                let output_path = format!("{}/debug_rust_target_prep_output_memory.npy", base_path);
                match File::create(&output_path) {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        match array.write_npy(writer) {
                            Ok(_) => {
                                debug_log!("[DEBUG] ✓ Saved output_memory to: {}", output_path)
                            }
                            Err(e) => {
                                debug_log!("[DEBUG] ✗ Failed to write output_memory npy: {:?}", e)
                            }
                        }
                    }
                    Err(e) => debug_log!("[DEBUG] ✗ Failed to create output_memory file: {:?}", e),
                }
            }

            // Save topk_ind: [1, 300]
            {
                let shape = topk_ind.size();
                let tensor_flat = topk_ind.flatten(0, -1).to_kind(tch::Kind::Int64);
                let tensor_vec: Vec<i64> =
                    Vec::try_from(tensor_flat).expect("Failed to convert topk_ind tensor to Vec");
                let array = ndarray::Array::from_shape_vec(
                    (shape[0] as usize, shape[1] as usize),
                    tensor_vec,
                )
                .expect("Failed to create topk_ind ndarray");
                let output_path = format!("{}/debug_rust_target_prep_topk_ind.npy", base_path);
                match File::create(&output_path) {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        match array.write_npy(writer) {
                            Ok(_) => debug_log!("[DEBUG] ✓ Saved topk_ind to: {}", output_path),
                            Err(e) => debug_log!("[DEBUG] ✗ Failed to write topk_ind npy: {:?}", e),
                        }
                    }
                    Err(e) => debug_log!("[DEBUG] ✗ Failed to create topk_ind file: {:?}", e),
                }
            }

            // Save max_scores: [1, 8400]
            {
                let shape = max_scores.size();
                let tensor_flat = max_scores.flatten(0, -1).to_kind(tch::Kind::Float);
                let tensor_vec: Vec<f32> =
                    Vec::try_from(tensor_flat).expect("Failed to convert max_scores tensor to Vec");
                let array = ndarray::Array::from_shape_vec(
                    (shape[0] as usize, shape[1] as usize),
                    tensor_vec,
                )
                .expect("Failed to create max_scores ndarray");
                let output_path = format!("{}/debug_rust_target_prep_max_scores.npy", base_path);
                match File::create(&output_path) {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        match array.write_npy(writer) {
                            Ok(_) => debug_log!("[DEBUG] ✓ Saved max_scores to: {}", output_path),
                            Err(e) => {
                                debug_log!("[DEBUG] ✗ Failed to write max_scores npy: {:?}", e)
                            }
                        }
                    }
                    Err(e) => debug_log!("[DEBUG] ✗ Failed to create max_scores file: {:?}", e),
                }
            }

            debug_log!("[DEBUG] All target preparation intermediates saved successfully");
        }

        // 13. Extract query embeddings
        debug_log!("[FORWARD] Extracting query embeddings...");
        let topk_ind_memory = topk_ind
            .unsqueeze(-1)
            .repeat([1, 1, output_memory.size()[2]]);
        debug_log!("[FORWARD] topk_ind_memory: {:?}", topk_ind_memory.size());
        let target = output_memory.gather(1, &topk_ind_memory, false).detach();
        debug_log!("[FORWARD] target: {:?}", target.size());

        // DEBUG: Print target sample values
        debug_log!(
            "[DEBUG] target[0, 0, :5] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            f64::try_from(target.i((0, 0, 0))).unwrap(),
            f64::try_from(target.i((0, 0, 1))).unwrap(),
            f64::try_from(target.i((0, 0, 2))).unwrap(),
            f64::try_from(target.i((0, 0, 3))).unwrap(),
            f64::try_from(target.i((0, 0, 4))).unwrap(),
        );

        // 14. Initialize reference points
        debug_log!("[FORWARD] Initializing reference points...");
        let init_reference_points = reference_points_unact.detach();
        debug_log!(
            "[FORWARD] init_reference_points: {:?}",
            init_reference_points.size()
        );

        // 15. Compute spatial shapes tensor and level_start_index
        debug_log!("[FORWARD] Computing spatial shapes tensor...");
        let spatial_shapes_tensor =
            Tensor::zeros([spatial_shapes.len() as i64, 2], (tch::Kind::Int64, device));
        let mut level_start_vec = vec![0i64];

        for (i, &(h, w)) in spatial_shapes.iter().enumerate() {
            debug_log!("[FORWARD]   Level {}: {}x{}", i, h, w);
            let _ = spatial_shapes_tensor.i((i as i64, 0)).fill_(h);
            let _ = spatial_shapes_tensor.i((i as i64, 1)).fill_(w);
            if i > 0 {
                level_start_vec.push(
                    level_start_vec[i - 1] + spatial_shapes[i - 1].0 * spatial_shapes[i - 1].1,
                );
            }
        }
        debug_log!("[FORWARD] Level start indices: {:?}", level_start_vec);

        let level_start_index = Tensor::from_slice(&level_start_vec).to_device(device);
        debug_log!(
            "[FORWARD] spatial_shapes_tensor: {:?}",
            spatial_shapes_tensor.size()
        );
        debug_log!(
            "[FORWARD] level_start_index: {:?}",
            level_start_index.size()
        );

        // 16. Run decoder
        debug_log!("[FORWARD] Running decoder...");
        debug_log!("[FORWARD]   target: {:?}", target.size());
        debug_log!("[FORWARD]   source_flatten: {:?}", source_flatten.size());
        debug_log!(
            "[FORWARD]   init_reference_points: {:?}",
            init_reference_points.size()
        );
        debug_log!(
            "[FORWARD]   spatial_shapes_tensor: {:?}",
            spatial_shapes_tensor.size()
        );

        // DEBUG: Print sample values
        debug_log!(
            "[DEBUG] source_flatten[0, 0, :5] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            source_flatten.double_value(&[0, 0, 0]),
            source_flatten.double_value(&[0, 0, 1]),
            source_flatten.double_value(&[0, 0, 2]),
            source_flatten.double_value(&[0, 0, 3]),
            source_flatten.double_value(&[0, 0, 4]),
        );
        debug_log!(
            "[DEBUG] target[0, 0, :5] = [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            target.double_value(&[0, 0, 0]),
            target.double_value(&[0, 0, 1]),
            target.double_value(&[0, 0, 2]),
            target.double_value(&[0, 0, 3]),
            target.double_value(&[0, 0, 4]),
        );
        debug_log!(
            "[DEBUG] init_reference_points[0, 0, :] = [{:.6}, {:.6}, {:.6}, {:.6}]",
            init_reference_points.double_value(&[0, 0, 0]),
            init_reference_points.double_value(&[0, 0, 1]),
            init_reference_points.double_value(&[0, 0, 2]),
            init_reference_points.double_value(&[0, 0, 3]),
        );

        // Save decoder inputs to .npy files for comparison with Python baseline
        // Only save if DEBUG_SAVE_DECODER_INPUTS env var is set to pdf/page (e.g., "jfk_scanned/12")
        if let Ok(debug_page) = std::env::var("DEBUG_SAVE_DECODER_INPUTS") {
            use ndarray_npy::WriteNpyExt;
            use std::fs::File;
            use std::io::BufWriter;

            let base_path = format!("baseline_data/{}/layout/rust_decoder_inputs", debug_page);
            let _ = std::fs::create_dir_all(&base_path);

            trace!("[DEBUG] Saving decoder inputs to {}", base_path);

            // Save topk_ind: [1, 300] - which encoder outputs were selected
            {
                let shape = topk_ind.size();
                let tensor_flat = topk_ind.flatten(0, -1).to_kind(tch::Kind::Int64);
                let tensor_vec: Vec<i64> =
                    Vec::try_from(tensor_flat).expect("Failed to convert topk_ind to Vec");

                // Save as int64 array
                use ndarray::Array2;
                let array =
                    Array2::from_shape_vec((shape[0] as usize, shape[1] as usize), tensor_vec)
                        .expect("Failed to create topk_ind ndarray");
                let output_path = format!("{}/rust_topk_indices.npy", base_path);
                match File::create(&output_path) {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        match array.write_npy(writer) {
                            Ok(_) => trace!("[DEBUG] ✓ Saved topk_indices to: {}", output_path),
                            Err(e) => trace!("[DEBUG] ✗ Failed to write topk_indices: {:?}", e),
                        }
                    }
                    Err(e) => trace!("[DEBUG] ✗ Failed to create topk_indices file: {:?}", e),
                }
            }

            // Save target (query embeddings): [1, 300, 256]
            {
                let shape = target.size();
                let tensor_flat = target.flatten(0, -1).to_kind(tch::Kind::Float);
                let tensor_vec: Vec<f32> =
                    Vec::try_from(tensor_flat).expect("Failed to convert target tensor to Vec");
                let array = ndarray::Array::from_shape_vec(
                    (shape[0] as usize, shape[1] as usize, shape[2] as usize),
                    tensor_vec,
                )
                .expect("Failed to create target ndarray");
                let output_path = format!("{}/rust_query_embeddings.npy", base_path);
                match File::create(&output_path) {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        match array.write_npy(writer) {
                            Ok(_) => {
                                trace!("[DEBUG] ✓ Saved query_embeddings to: {}", output_path)
                            }
                            Err(e) => {
                                trace!("[DEBUG] ✗ Failed to write query_embeddings: {:?}", e)
                            }
                        }
                    }
                    Err(e) => {
                        trace!("[DEBUG] ✗ Failed to create query_embeddings file: {:?}", e)
                    }
                }
            }

            // Save init_reference_points: [1, 300, 4]
            {
                let shape = init_reference_points.size();
                let tensor_flat = init_reference_points
                    .flatten(0, -1)
                    .to_kind(tch::Kind::Float);
                let tensor_vec: Vec<f32> = Vec::try_from(tensor_flat)
                    .expect("Failed to convert init_reference_points tensor to Vec");
                let array = ndarray::Array::from_shape_vec(
                    (shape[0] as usize, shape[1] as usize, shape[2] as usize),
                    tensor_vec,
                )
                .expect("Failed to create init_reference_points ndarray");
                let output_path = format!("{}/rust_reference_points.npy", base_path);
                match File::create(&output_path) {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        match array.write_npy(writer) {
                            Ok(_) => {
                                trace!("[DEBUG] ✓ Saved reference_points to: {}", output_path)
                            }
                            Err(e) => {
                                trace!("[DEBUG] ✗ Failed to write reference_points: {:?}", e)
                            }
                        }
                    }
                    Err(e) => {
                        trace!("[DEBUG] ✗ Failed to create reference_points file: {:?}", e)
                    }
                }
            }

            // Save source_flatten (encoder memory): [1, 8400, 256]
            {
                let shape = source_flatten.size();
                let tensor_flat = source_flatten.flatten(0, -1).to_kind(tch::Kind::Float);
                let tensor_vec: Vec<f32> = Vec::try_from(tensor_flat)
                    .expect("Failed to convert source_flatten tensor to Vec");
                let array = ndarray::Array::from_shape_vec(
                    (shape[0] as usize, shape[1] as usize, shape[2] as usize),
                    tensor_vec,
                )
                .expect("Failed to create source_flatten ndarray");
                let output_path =
                    format!("{}/debug_rust_decoder_input_source_flatten.npy", base_path);
                match File::create(&output_path) {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        match array.write_npy(writer) {
                            Ok(_) => {
                                debug_log!("[DEBUG] ✓ Saved source_flatten to: {}", output_path)
                            }
                            Err(e) => {
                                debug_log!("[DEBUG] ✗ Failed to write source_flatten npy: {:?}", e)
                            }
                        }
                    }
                    Err(e) => debug_log!("[DEBUG] ✗ Failed to create source_flatten file: {:?}", e),
                }
            }

            debug_log!("[DEBUG] All decoder inputs saved successfully");
        }

        let decoder_start = Instant::now();
        let decoder_output = self.decoder.forward(
            &target,
            &source_flatten,
            &init_reference_points,
            &spatial_shapes_tensor,
            &spatial_shapes,
            None,  // encoder_attention_mask
            false, // output_attentions
            false, // output_hidden_states
        );
        let decoder_time = decoder_start.elapsed();
        profile_log!(
            "[PROFILE] Decoder: {:.2} ms",
            decoder_time.as_secs_f64() * 1000.0
        );
        debug_log!("[FORWARD] Decoder complete");

        // Optionally save memory (before enc_output) for encoder debugging
        if std::env::var("SAVE_ENCODER_BEFORE_ENC_OUTPUT").is_ok() {
            let debug_path = std::env::var("SAVE_ENCODER_BEFORE_ENC_OUTPUT").unwrap();
            use ndarray_npy::WriteNpyExt;
            use std::fs::File;
            use std::io::BufWriter;

            let save_dir = format!("baseline_data/{}/layout/rust_encoder", debug_path);
            std::fs::create_dir_all(&save_dir).ok();

            let shape = memory.size();
            let flat: Vec<f32> =
                Vec::try_from(memory.flatten(0, -1).to_kind(tch::Kind::Float)).unwrap();
            let array = ndarray::Array::from_shape_vec(
                (shape[0] as usize, shape[1] as usize, shape[2] as usize),
                flat,
            )
            .unwrap();
            let path = format!("{}/memory_before_enc_output.npy", save_dir);
            if let Ok(file) = File::create(&path) {
                let _ = array.write_npy(BufWriter::new(file));
                trace!("[DEBUG] Saved memory before enc_output to {}", path);
            }
        }

        Ok(RTDetrV2ModelOutput {
            last_hidden_state: decoder_output.0,
            intermediate_hidden_states: decoder_output.1,
            intermediate_logits: decoder_output.3,
            intermediate_reference_points: decoder_output.2,
            encoder_memory: Some(output_memory), // Save encoder output for validation
        })
    }
}

/// RT-DETR v2 for object detection
/// Python: RTDetrV2ForObjectDetection
///
/// Note: Detection heads (class_embed, bbox_embed) are now owned by the decoder
/// for iterative refinement during forward pass
pub struct RTDetrV2ForObjectDetection {
    pub model: RTDetrV2Model,
}

impl std::fmt::Debug for RTDetrV2ForObjectDetection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RTDetrV2ForObjectDetection")
            .field("model", &self.model)
            .finish()
    }
}

impl RTDetrV2ForObjectDetection {
    #[must_use = "constructor returns Result that should be handled"]
    pub fn new(vs: &nn::Path, config: RTDetrV2Config) -> Result<Self> {
        let mut model = RTDetrV2Model::new(vs, config.clone())?;
        // Detection heads are under model.decoder.{class_embed,bbox_embed} in HF
        let (class_embed, bbox_embed) =
            create_detection_heads(&(vs / "model" / "decoder"), &config);

        // Assign detection heads to decoder (Python: model.decoder.class_embed = class_embed)
        // The decoder now owns the detection heads and will use them for iterative refinement
        model.decoder.set_detection_heads(class_embed, bbox_embed);

        Ok(Self { model })
    }

    /// Load model from pretrained weights
    ///
    /// # Arguments
    /// * `weights_path` - Path to model.safetensors file
    /// * `config` - Model configuration
    /// * `device` - Device to load model onto (CPU/CUDA/Metal)
    ///
    /// # Returns
    /// * `Result<Self>` - Loaded model ready for inference
    ///
    /// # Example
    /// ```no_run
    /// use docling_pdf_ml::models::layout_predictor::pytorch_backend::model::{RTDetrV2Config, RTDetrV2ForObjectDetection};
    /// use tch::Device;
    /// use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = RTDetrV2Config::default();
    /// let model = RTDetrV2ForObjectDetection::from_pretrained(
    ///     Path::new("model.safetensors"),
    ///     config,
    ///     Device::Cpu,
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_pretrained(
        weights_path: &std::path::Path,
        config: RTDetrV2Config,
        device: tch::Device,
    ) -> Result<Self> {
        // Create VarStore on specified device
        let mut vs = tch::nn::VarStore::new(device);

        // Create model (this registers all variables in the VarStore)
        let model = Self::new(&vs.root(), config.clone())?;

        // Now load weights from file
        // This will populate the already-registered variables
        vs.load(weights_path)
            .map_err(|e| format!("Failed to load weights from {:?}: {}", weights_path, e))?;

        // Verify weights are complete
        super::weights::verify_weights(&vs, config.decoder_layers, config.num_feature_levels)?;

        Ok(model)
    }

    /// Forward pass for object detection
    ///
    /// # Arguments
    /// * `pixel_values` - Input images [batch_size, 3, height, width]
    ///
    /// # Returns
    /// RTDetrV2ObjectDetectionOutput with logits and bounding boxes
    #[must_use = "forward pass returns output that should be processed"]
    pub fn forward(&self, pixel_values: &Tensor) -> Result<RTDetrV2ObjectDetectionOutput> {
        // 1. Get base model outputs (decoder applies detection heads internally)
        let outputs = self.model.forward(pixel_values)?;

        // 2. Extract outputs from decoder
        // The decoder has already applied classification and bbox heads during forward pass
        // intermediate_logits: [batch, num_layers, num_queries, num_classes]
        // intermediate_reference_points: [batch, num_layers, num_queries, 4]

        let all_logits = outputs
            .intermediate_logits
            .as_ref()
            .expect("Decoder should return intermediate_logits when detection heads are set");

        let num_layers = all_logits.size()[1];

        // 3. Extract final layer predictions (last decoder layer)
        // Logits are already computed by decoder
        let logits = all_logits.select(1, num_layers - 1);

        // Bounding boxes: Use final reference points (already refined through decoder)
        // Reference points are in sigmoid space [0, 1] and represent (cx, cy, w, h)
        let pred_boxes = outputs
            .intermediate_reference_points
            .select(1, num_layers - 1);

        Ok(RTDetrV2ObjectDetectionOutput {
            logits,
            pred_boxes,
            last_hidden_state: outputs.last_hidden_state,
            intermediate_hidden_states: outputs.intermediate_hidden_states,
            intermediate_logits: outputs.intermediate_logits,
            intermediate_reference_points: outputs.intermediate_reference_points,
        })
    }
}

/// Helper function to create detection heads (class_embed and bbox_embed)
/// Used by RTDetrV2ForObjectDetection
pub fn create_detection_heads(
    vs: &nn::Path,
    config: &RTDetrV2Config,
) -> (Vec<nn::Linear>, Vec<RTDetrV2MLPPredictionHead>) {
    let mut class_embed = Vec::new();
    let mut bbox_embed = Vec::new();

    for i in 0..config.decoder_layers {
        // Classification head: d_model -> num_labels
        let class_head = nn::linear(
            &(vs / "class_embed" / i),
            config.d_model,
            config.num_labels,
            Default::default(),
        );
        class_embed.push(class_head);

        // Bounding box head: 3-layer MLP (d_model -> d_model -> 4)
        let bbox_head = RTDetrV2MLPPredictionHead::new(
            &(vs / "bbox_embed" / i),
            config.d_model,
            config.d_model,
            4,
            3, // num_layers
        );
        bbox_embed.push(bbox_head);
    }

    (class_embed, bbox_embed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tch::Device;

    #[test]
    fn test_config_default() {
        let config = RTDetrV2Config::default();
        assert_eq!(config.num_queries, 300);
        assert_eq!(config.decoder_layers, 6);
        assert_eq!(config.d_model, 256);
    }

    #[test]
    fn test_create_detection_heads() {
        let device = Device::Cpu;
        let vs = nn::VarStore::new(device);
        let root = vs.root();

        let config = RTDetrV2Config::default();
        let (class_embed, bbox_embed) = create_detection_heads(&root, &config);

        // Should have one head per decoder layer
        assert_eq!(class_embed.len(), config.decoder_layers as usize);
        assert_eq!(bbox_embed.len(), config.decoder_layers as usize);
    }
}
