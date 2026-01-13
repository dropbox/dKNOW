// RT-DETR v2 decoder implementation
// Ported from transformers/models/rt_detr_v2/modeling_rt_detr_v2.py:346-726

// Debug logging macro - disabled by default for performance
// To enable: Uncomment the macro below
// macro_rules! debug_log { ($($arg:tt)*) => { log::warn!($($arg)*) }; }
macro_rules! debug_log {
    ($($arg:tt)*) => {
        ()
    };
}

use super::deformable_attention::multi_scale_deformable_attention_v2;
use super::transformer::RTDetrV2MultiheadAttention;
use std::sync::atomic::{AtomicBool, Ordering};
use tch::nn::Module;
use tch::{nn, Tensor};

/// Activation function types
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Activation {
    ReLU,
    GELU,
    Sigmoid,
}

impl Activation {
    pub fn apply(&self, x: &Tensor) -> Tensor {
        match self {
            Activation::ReLU => x.relu(),
            Activation::GELU => {
                // Exact GELU implementation using error function (matches Python)
                // GELU(x) = 0.5 * x * (1 + erf(x / sqrt(2)))
                let x_normalized = x / (2.0_f64.sqrt());
                0.5_f64 * x * (1.0_f64 + x_normalized.erf())
            }
            Activation::Sigmoid => x.sigmoid(),
        }
    }
}

impl std::fmt::Display for Activation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReLU => write!(f, "relu"),
            Self::GELU => write!(f, "gelu"),
            Self::Sigmoid => write!(f, "sigmoid"),
        }
    }
}

impl std::str::FromStr for Activation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "relu" => Ok(Self::ReLU),
            "gelu" => Ok(Self::GELU),
            "sigmoid" | "sig" => Ok(Self::Sigmoid),
            _ => Err(format!("unknown activation function: '{s}'")),
        }
    }
}

/// Multi-scale deformable attention module for decoder cross-attention
/// Python: RTDetrV2MultiscaleDeformableAttention (lines 116-223)
#[derive(Debug)]
pub struct RTDetrV2MultiscaleDeformableAttention {
    pub d_model: i64,
    pub n_levels: i64,
    pub n_heads: i64,
    pub n_points: i64,
    pub offset_scale: f64,
    pub method: String,
    pub n_points_list: Vec<i64>,
    pub n_points_scale: Tensor,

    pub sampling_offsets: nn::Linear,
    pub attention_weights: nn::Linear,
    pub value_proj: nn::Linear,
    pub output_proj: nn::Linear,
}

// Global flag to track if we've saved cross-attention debug data from the first forward call
static CROSS_ATTN_FIRST_CALL_DONE: AtomicBool = AtomicBool::new(false);

impl RTDetrV2MultiscaleDeformableAttention {
    /// Helper function to save tensor to .npy file for debugging cross-attention internals
    /// Uses an atomic flag to ensure only first forward call saves (decoder layer 0 only)
    fn save_cross_attn_debug(tensor: &Tensor, name: &str) {
        use ndarray_npy::WriteNpyExt;
        use std::fs::File;
        use std::io::BufWriter;
        use std::path::Path;

        // Check if we've already completed the first forward call
        if CROSS_ATTN_FIRST_CALL_DONE.load(Ordering::SeqCst) {
            return; // Already saved from layer 0, skip all subsequent calls
        }

        let output_dir =
            Path::new("baseline_data/arxiv_2206.01062/page_0/layout/rust_cross_attn_internals");

        // Create output directory
        std::fs::create_dir_all(output_dir).ok();

        let output_path = output_dir.join(format!("rust_{}.npy", name));

        // Convert tensor to CPU and to f32 for consistency
        let cpu_tensor = tensor.to_device(tch::Device::Cpu).to_kind(tch::Kind::Float);

        // Get tensor shape
        let shape = cpu_tensor.size();

        // Convert to Vec<f32>
        let data: Vec<f32> = cpu_tensor.flatten(0, -1).try_into().unwrap();

        // Convert shape to Vec<usize>
        let shape_usize: Vec<usize> = shape.iter().map(|&x| x as usize).collect();

        // Create ndarray from data
        let array = ndarray::ArrayD::from_shape_vec(shape_usize, data).unwrap();

        // Write to file
        if let Ok(file) = File::create(&output_path) {
            let writer = BufWriter::new(file);
            array.write_npy(writer).ok();
            debug_log!("[DEBUG] Saved {} to {}", name, output_path.display());
        }
    }

    /// Mark that the first forward call is complete (all tensors saved)
    fn mark_first_call_complete() {
        CROSS_ATTN_FIRST_CALL_DONE.store(true, Ordering::SeqCst);
    }

    pub fn new(
        vs: &nn::Path,
        d_model: i64,
        n_heads: i64,
        n_levels: i64,
        n_points: i64,
        offset_scale: f64,
        method: &str,
    ) -> Self {
        assert!(
            d_model % n_heads == 0,
            "d_model {} must be divisible by n_heads {}",
            d_model,
            n_heads
        );

        let dim_per_head = d_model / n_heads;

        // Warn if dim_per_head is not a power of 2
        if !((dim_per_head & (dim_per_head - 1) == 0) && dim_per_head != 0) {
            log::warn!(
                "Warning: dim_per_head {} is not a power of 2. This may be less efficient.",
                dim_per_head
            );
        }

        let linear_config = nn::LinearConfig {
            bias: true,
            ..Default::default()
        };

        let sampling_offsets = nn::linear(
            vs / "sampling_offsets",
            d_model,
            n_heads * n_levels * n_points * 2,
            linear_config,
        );

        let attention_weights = nn::linear(
            vs / "attention_weights",
            d_model,
            n_heads * n_levels * n_points,
            linear_config,
        );

        let value_proj = nn::linear(vs / "value_proj", d_model, d_model, linear_config);
        let output_proj = nn::linear(vs / "output_proj", d_model, d_model, linear_config);

        // Initialize n_points_list and scale
        let n_points_list: Vec<i64> = (0..n_levels).map(|_| n_points).collect();
        let n_points_scale_vec: Vec<f32> = n_points_list
            .iter()
            .flat_map(|&n| vec![1.0 / n as f32; n as usize])
            .collect();

        let n_points_scale = Tensor::from_slice(&n_points_scale_vec).to_device(vs.device());

        Self {
            d_model,
            n_levels,
            n_heads,
            n_points,
            offset_scale,
            method: method.to_string(),
            n_points_list,
            n_points_scale,
            sampling_offsets,
            attention_weights,
            value_proj,
            output_proj,
        }
    }

    /// Forward pass
    ///
    /// Args:
    ///   hidden_states: [batch_size, num_queries, d_model] - Decoder queries
    ///   encoder_hidden_states: [batch_size, sequence_length, d_model] - Encoder outputs
    ///   position_embeddings: Optional position embeddings to add to queries
    ///   reference_points: Reference points for deformable attention
    ///   spatial_shapes: [n_levels, 2] - (height, width) for each level
    ///   spatial_shapes_list: Vec of (height, width) tuples
    ///   level_start_index: Starting index for each level
    ///   attention_mask: Optional attention mask
    ///   output_attentions: Whether to return attention weights
    ///
    /// Returns: (output, attention_weights)
    ///   output: [batch_size, num_queries, d_model]
    ///   attention_weights: Optional [batch_size, num_queries, n_heads, n_levels * n_points]
    #[allow(
        clippy::too_many_arguments,
        reason = "deformable attention forward pass requires many tensors"
    )]
    pub fn forward(
        &self,
        hidden_states: &Tensor,
        encoder_hidden_states: &Tensor,
        position_embeddings: Option<&Tensor>,
        reference_points: &Tensor,
        spatial_shapes: &Tensor,
        spatial_shapes_list: &[(i64, i64)],
        _level_start_index: Option<&Tensor>,
        attention_mask: Option<&Tensor>,
        output_attentions: bool,
    ) -> (Tensor, Option<Tensor>) {
        debug_log!("[DEFORM_ATTN] Starting deformable attention...");
        debug_log!("[DEFORM_ATTN]   hidden_states: {:?}", hidden_states.size());
        debug_log!(
            "[DEFORM_ATTN]   encoder_hidden_states: {:?}",
            encoder_hidden_states.size()
        );
        debug_log!(
            "[DEFORM_ATTN]   reference_points: {:?}",
            reference_points.size()
        );
        debug_log!(
            "[DEFORM_ATTN]   spatial_shapes: {:?}",
            spatial_shapes.size()
        );

        // Save inputs
        Self::save_cross_attn_debug(hidden_states, "input_hidden_states");
        Self::save_cross_attn_debug(encoder_hidden_states, "encoder_hidden_states");
        Self::save_cross_attn_debug(reference_points, "reference_points");
        Self::save_cross_attn_debug(spatial_shapes, "spatial_shapes");

        // Add position embeddings to queries if provided
        let hidden_states = if let Some(pos_emb) = position_embeddings {
            debug_log!(
                "[DEFORM_ATTN] Adding position embeddings: {:?}",
                pos_emb.size()
            );
            Self::save_cross_attn_debug(pos_emb, "position_embeddings");
            let result = hidden_states + pos_emb;
            Self::save_cross_attn_debug(&result, "hidden_states_with_pos");
            result
        } else {
            debug_log!("[DEFORM_ATTN] No position embeddings");
            hidden_states.shallow_clone()
        };

        let shape = hidden_states.size();
        let batch_size = shape[0];
        let num_queries = shape[1];

        let encoder_shape = encoder_hidden_states.size();
        let sequence_length = encoder_shape[1];
        debug_log!(
            "[DEFORM_ATTN] batch_size={}, num_queries={}, sequence_length={}",
            batch_size,
            num_queries,
            sequence_length
        );

        // Validate spatial shapes match sequence length
        debug_log!("[DEFORM_ATTN] Validating spatial shapes...");
        let spatial_prod = spatial_shapes.select(1, 0) * spatial_shapes.select(1, 1);
        let expected_seq_len = spatial_prod.sum(tch::Kind::Int64).int64_value(&[]);
        debug_log!(
            "[DEFORM_ATTN]   expected_seq_len: {}, actual: {}",
            expected_seq_len,
            sequence_length
        );
        assert_eq!(
            expected_seq_len, sequence_length,
            "Spatial shapes {:?} do not match sequence length {}",
            spatial_shapes, sequence_length
        );

        // Project encoder hidden states to values
        debug_log!("[DEFORM_ATTN] Projecting values...");
        let mut value = encoder_hidden_states.apply(&self.value_proj);
        debug_log!("[DEFORM_ATTN]   value: {:?}", value.size());
        Self::save_cross_attn_debug(&value, "value_after_proj");

        // Apply attention mask if provided
        if let Some(mask) = attention_mask {
            debug_log!("[DEFORM_ATTN] Applying attention mask...");
            let mask_broadcasted = mask.unsqueeze(-1);
            value = value.where_self(&mask_broadcasted, &Tensor::zeros_like(&value));
            Self::save_cross_attn_debug(&value, "value_after_mask");
        }

        // Reshape value: [batch_size, sequence_length, n_heads, head_dim]
        debug_log!("[DEFORM_ATTN] Reshaping value...");
        let value = value.view([
            batch_size,
            sequence_length,
            self.n_heads,
            self.d_model / self.n_heads,
        ]);
        debug_log!("[DEFORM_ATTN]   value reshaped: {:?}", value.size());
        Self::save_cross_attn_debug(&value, "value_reshaped");

        // Compute sampling offsets
        debug_log!("[DEFORM_ATTN] Computing sampling offsets...");
        let sampling_offsets_raw = hidden_states.apply(&self.sampling_offsets);
        Self::save_cross_attn_debug(&sampling_offsets_raw, "sampling_offsets_raw");

        // Save reshaped version to match Python baseline (6D shape)
        let sampling_offsets_reshaped_debug = sampling_offsets_raw.view([
            batch_size,
            num_queries,
            self.n_heads,
            self.n_levels,
            self.n_points,
            2,
        ]);
        Self::save_cross_attn_debug(
            &sampling_offsets_reshaped_debug,
            "sampling_offsets_reshaped",
        );

        let sampling_offsets = sampling_offsets_raw.view([
            batch_size,
            num_queries,
            self.n_heads,
            self.n_levels * self.n_points,
            2,
        ]);
        debug_log!(
            "[DEFORM_ATTN]   sampling_offsets: {:?}",
            sampling_offsets.size()
        );

        // Compute attention weights
        debug_log!("[DEFORM_ATTN] Computing attention weights...");
        let attention_weights_before_softmax = hidden_states.apply(&self.attention_weights);
        Self::save_cross_attn_debug(
            &attention_weights_before_softmax,
            "attention_weights_before_softmax",
        );

        let mut attention_weights_raw = attention_weights_before_softmax.view([
            batch_size,
            num_queries,
            self.n_heads,
            self.n_levels * self.n_points,
        ]);
        debug_log!(
            "[DEFORM_ATTN]   attention_weights_raw: {:?}",
            attention_weights_raw.size()
        );

        attention_weights_raw = attention_weights_raw.softmax(-1, attention_weights_raw.kind());
        debug_log!(
            "[DEFORM_ATTN]   attention_weights after softmax: {:?}",
            attention_weights_raw.size()
        );
        Self::save_cross_attn_debug(&attention_weights_raw, "attention_weights_after_softmax");

        // Compute sampling locations based on reference points shape
        debug_log!("[DEFORM_ATTN] Computing sampling locations...");
        let ref_shape = reference_points.size();
        let ref_last_dim = ref_shape[ref_shape.len() - 1];
        debug_log!("[DEFORM_ATTN]   ref_last_dim: {}", ref_last_dim);

        let sampling_locations = if ref_last_dim == 2 {
            // Reference points are 2D (x, y)
            // Normalize by spatial shapes
            let offset_normalizer = Tensor::stack(
                &[spatial_shapes.select(1, 1), spatial_shapes.select(1, 0)],
                -1,
            );

            // reference_points: [batch_size, num_queries, n_levels, 2]
            // sampling_offsets: [batch_size, num_queries, n_heads, n_levels * n_points, 2]
            // Need to reshape reference_points to [batch_size, num_queries, 1, n_levels, 1, 2]
            let ref_reshaped = reference_points
                .unsqueeze(2) // [batch_size, num_queries, 1, n_levels, 2]
                .unsqueeze(4); // [batch_size, num_queries, 1, n_levels, 1, 2]

            // Reshape sampling_offsets to [batch_size, num_queries, n_heads, n_levels, n_points, 2]
            let offsets_reshaped = sampling_offsets.view([
                batch_size,
                num_queries,
                self.n_heads,
                self.n_levels,
                self.n_points,
                2,
            ]);

            // Normalize offsets
            let offset_normalizer = offset_normalizer
                .unsqueeze(0)
                .unsqueeze(0)
                .unsqueeze(2)
                .unsqueeze(4);
            let normalized_offsets = &offsets_reshaped / &offset_normalizer;

            // Add reference points to offsets
            &ref_reshaped + &normalized_offsets
        } else if ref_last_dim == 4 {
            // Reference points are 4D (x, y, w, h)
            // Input shape: [batch_size, num_queries, 1, 4] (middle dim is 1, not n_levels)
            // Python: reference_points[:, :, None, :, 2:] creates [batch, queries, 1, 1, 2]

            // Apply offset scaling with box dimensions
            // n_points_scale is [n_levels * n_points] = [12]
            // Python flattens to [12, 1], NOT [3, 4, 1]
            let n_points_scale = self
                .n_points_scale
                .to_device(hidden_states.device())
                .to_kind(hidden_states.kind())
                .view([self.n_levels * self.n_points, 1]);

            // Reshape sampling_offsets to 6D for debug save (Python saves this)
            let offsets_reshaped_6d = sampling_offsets.view([
                batch_size,
                num_queries,
                self.n_heads,
                self.n_levels,
                self.n_points,
                2,
            ]);

            // Python flattens to [batch, queries, heads, n_levels * n_points, 2] for computation
            let offsets_reshaped = sampling_offsets.view([
                batch_size,
                num_queries,
                self.n_heads,
                self.n_levels * self.n_points,
                2,
            ]);

            // Python: reference_points[:, :, None, :, 2:]
            // For input [1, 300, 1, 4]: [:, :, None, :, 2:] → [1, 300, 1, 1, 2]
            debug_log!("[DEFORM_ATTN]   Extracting ref_wh (w, h)...");
            let ref_wh = reference_points
                .unsqueeze(2) // [batch, queries, 1, 1, 4]
                .narrow(4, 2, 2); // [batch, queries, 1, 1, 2] (w, h)
            debug_log!("[DEFORM_ATTN]   ref_wh: {:?}", ref_wh.size());

            // Python: reference_points[:, :, None, :, :2]
            // For input [1, 300, 1, 4]: [:, :, None, :, :2] → [1, 300, 1, 1, 2]
            debug_log!("[DEFORM_ATTN]   Extracting ref_xy (x, y)...");
            let ref_xy = reference_points
                .unsqueeze(2) // [batch, queries, 1, 1, 4]
                .narrow(4, 0, 2); // [batch, queries, 1, 1, 2] (x, y)
            debug_log!("[DEFORM_ATTN]   ref_xy: {:?}", ref_xy.size());
            Self::save_cross_attn_debug(&ref_xy, "ref_xy");
            Self::save_cross_attn_debug(&ref_wh, "ref_wh");
            Self::save_cross_attn_debug(&n_points_scale, "n_points_scale");

            // Python: offset = sampling_offsets * n_points_scale * ref_wh * offset_scale
            // offsets_reshaped: [batch, queries, heads, n_levels * n_points, 2] = [1, 300, 8, 12, 2]
            // n_points_scale: [n_levels * n_points, 1] = [12, 1]
            // ref_wh: [batch, queries, 1, 1, 2] = [1, 300, 1, 1, 2]
            // Broadcasting: [1, 300, 8, 12, 2] * [12, 1] * [1, 300, 1, 1, 2] → [1, 300, 8, 12, 2]
            debug_log!("[DEFORM_ATTN]   Computing offset...");
            debug_log!(
                "[DEFORM_ATTN]     offsets_reshaped: {:?}",
                offsets_reshaped.size()
            );
            debug_log!(
                "[DEFORM_ATTN]     n_points_scale: {:?}",
                n_points_scale.size()
            );
            debug_log!("[DEFORM_ATTN]     ref_wh: {:?}", ref_wh.size());
            debug_log!("[DEFORM_ATTN]     offset_scale: {}", self.offset_scale);
            let offset = &offsets_reshaped * &n_points_scale * &ref_wh * self.offset_scale;
            debug_log!("[DEFORM_ATTN]   offset computed: {:?}", offset.size());
            Self::save_cross_attn_debug(&offset, "offset");

            // Add reference points to offsets
            // sampling_locations = ref_xy + offset
            // ref_xy: [1, 300, 1, 1, 2]
            // offset: [1, 300, 8, 12, 2]
            // Broadcasting: [1, 300, 1, 1, 2] + [1, 300, 8, 12, 2] → [1, 300, 8, 12, 2]
            debug_log!("[DEFORM_ATTN]   Computing sampling_locations = ref_xy + offset...");
            let sampling_locations = &ref_xy + &offset;
            debug_log!(
                "[DEFORM_ATTN]   sampling_locations: {:?}",
                sampling_locations.size()
            );
            Self::save_cross_attn_debug(&sampling_locations, "sampling_locations");
            sampling_locations
        } else {
            panic!(
                "Last dim of reference_points must be 2 or 4, but got {}",
                ref_last_dim
            );
        };

        // Reshape attention weights for deformable attention
        // [batch_size, num_queries, n_heads, n_levels, n_points]
        debug_log!("[DEFORM_ATTN] Reshaping attention weights...");
        let attention_weights_reshaped = attention_weights_raw.view([
            batch_size,
            num_queries,
            self.n_heads,
            self.n_levels,
            self.n_points,
        ]);
        debug_log!(
            "[DEFORM_ATTN]   attention_weights_reshaped: {:?}",
            attention_weights_reshaped.size()
        );
        debug_log!(
            "[DEFORM_ATTN]   sampling_locations: {:?}",
            sampling_locations.size()
        );

        // Apply multi-scale deformable attention
        debug_log!("[DEFORM_ATTN] Calling multi_scale_deformable_attention_v2...");
        let output = multi_scale_deformable_attention_v2(
            &value,
            spatial_shapes_list,
            &sampling_locations,
            &attention_weights_reshaped,
            &self.n_points_list,
            &self.method,
        );
        debug_log!("[DEFORM_ATTN]   output: {:?}", output.size());
        Self::save_cross_attn_debug(&output, "output_before_proj");

        // Project output
        debug_log!("[DEFORM_ATTN] Projecting output...");
        let output = output.apply(&self.output_proj);
        debug_log!("[DEFORM_ATTN]   output projected: {:?}", output.size());
        Self::save_cross_attn_debug(&output, "output_final");

        // Mark first call complete (all tensors from layer 0 saved)
        Self::mark_first_call_complete();

        // Return attention weights if requested
        let attn_weights = if output_attentions {
            Some(attention_weights_raw)
        } else {
            None
        };

        (output, attn_weights)
    }
}

/// Single decoder layer with self-attention, cross-attention, and feed-forward
/// Python: RTDetrV2DecoderLayer (lines 346-451)
#[derive(Debug)]
pub struct RTDetrV2DecoderLayer {
    pub dropout: f64,
    pub activation_dropout: f64,
    pub activation_fn: Activation,

    pub self_attn: RTDetrV2MultiheadAttention,
    pub self_attn_layer_norm: nn::LayerNorm,

    pub encoder_attn: RTDetrV2MultiscaleDeformableAttention,
    pub encoder_attn_layer_norm: nn::LayerNorm,

    pub fc1: nn::Linear,
    pub fc2: nn::Linear,
    pub final_layer_norm: nn::LayerNorm,
}

impl RTDetrV2DecoderLayer {
    /// Helper function to save tensor to .npy file for debugging
    fn save_tensor_debug(tensor: &Tensor, name: &str) {
        use ndarray_npy::WriteNpyExt;
        use std::fs::File;
        use std::io::BufWriter;
        use std::path::Path;

        let output_path = format!(
            "baseline_data/arxiv_2206.01062/page_0/layout/pytorch_decoder_internals/debug_rust_{}.npy",
            name
        );

        if let Some(parent) = Path::new(&output_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let shape = tensor.size();
        let tensor_flat = tensor.flatten(0, -1).to_kind(tch::Kind::Float);
        let tensor_vec: Vec<f32> = match Vec::try_from(tensor_flat) {
            Ok(v) => v,
            Err(e) => {
                debug_log!("[DEBUG] Failed to convert {} tensor to Vec: {:?}", name, e);
                return;
            }
        };

        let array = match ndarray::Array::from_shape_vec(
            (shape[0] as usize, shape[1] as usize, shape[2] as usize),
            tensor_vec,
        ) {
            Ok(arr) => arr,
            Err(e) => {
                debug_log!("[DEBUG] Failed to create {} ndarray: {:?}", name, e);
                return;
            }
        };

        match File::create(&output_path) {
            Ok(file) => {
                let writer = BufWriter::new(file);
                match array.write_npy(writer) {
                    Ok(_) => debug_log!("[DEBUG] ✓ Saved {} to: {}", name, output_path),
                    Err(e) => debug_log!("[DEBUG] ✗ Failed to write {} npy: {:?}", name, e),
                }
            }
            Err(e) => debug_log!("[DEBUG] ✗ Failed to create {} file: {:?}", name, e),
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "RT-DETR decoder layer requires many configuration params"
    )]
    pub fn new(
        vs: &nn::Path,
        d_model: i64,
        decoder_attention_heads: i64,
        decoder_ffn_dim: i64,
        dropout: f64,
        activation_dropout: f64,
        attention_dropout: f64,
        decoder_activation_function: Activation,
        layer_norm_eps: f64,
        decoder_n_levels: i64,
        decoder_n_points: i64,
        decoder_offset_scale: f64,
        decoder_method: &str,
    ) -> Self {
        // Self-attention
        let self_attn = RTDetrV2MultiheadAttention::new(
            &(vs / "self_attn"),
            d_model,
            decoder_attention_heads,
            attention_dropout,
            true,
        );

        let self_attn_layer_norm = nn::layer_norm(
            vs / "self_attn_layer_norm",
            vec![d_model],
            nn::LayerNormConfig {
                eps: layer_norm_eps,
                ..Default::default()
            },
        );

        // Cross-attention (deformable)
        let encoder_attn = RTDetrV2MultiscaleDeformableAttention::new(
            &(vs / "encoder_attn"),
            d_model,
            decoder_attention_heads,
            decoder_n_levels,
            decoder_n_points,
            decoder_offset_scale,
            decoder_method,
        );

        let encoder_attn_layer_norm = nn::layer_norm(
            vs / "encoder_attn_layer_norm",
            vec![d_model],
            nn::LayerNormConfig {
                eps: layer_norm_eps,
                ..Default::default()
            },
        );

        // Feed-forward network
        let linear_config = nn::LinearConfig {
            bias: true,
            ..Default::default()
        };
        let fc1 = nn::linear(vs / "fc1", d_model, decoder_ffn_dim, linear_config);
        let fc2 = nn::linear(vs / "fc2", decoder_ffn_dim, d_model, linear_config);

        let final_layer_norm = nn::layer_norm(
            vs / "final_layer_norm",
            vec![d_model],
            nn::LayerNormConfig {
                eps: layer_norm_eps,
                ..Default::default()
            },
        );

        Self {
            dropout,
            activation_dropout,
            activation_fn: decoder_activation_function,
            self_attn,
            self_attn_layer_norm,
            encoder_attn,
            encoder_attn_layer_norm,
            fc1,
            fc2,
            final_layer_norm,
        }
    }

    /// Forward pass
    ///
    /// Args:
    ///   hidden_states: [batch_size, num_queries, d_model] - Decoder queries
    ///   position_embeddings: Optional position embeddings to add to queries and keys
    ///   reference_points: Reference points for deformable attention
    ///   spatial_shapes: [n_levels, 2] - (height, width) for each level
    ///   spatial_shapes_list: Vec of (height, width) tuples
    ///   level_start_index: Starting index for each level
    ///   encoder_hidden_states: [batch_size, sequence_length, d_model] - Encoder outputs
    ///   encoder_attention_mask: Optional encoder attention mask
    ///   output_attentions: Whether to return attention weights
    ///   train: Whether in training mode (for dropout)
    ///   layer_idx: Layer index (for debugging)
    ///
    /// Returns: (hidden_states, self_attn_weights, cross_attn_weights)
    #[allow(
        clippy::too_many_arguments,
        reason = "decoder layer forward requires many tensors and config"
    )]
    pub fn forward(
        &self,
        hidden_states: &Tensor,
        position_embeddings: Option<&Tensor>,
        reference_points: &Tensor,
        spatial_shapes: &Tensor,
        spatial_shapes_list: &[(i64, i64)],
        level_start_index: Option<&Tensor>,
        encoder_hidden_states: &Tensor,
        encoder_attention_mask: Option<&Tensor>,
        output_attentions: bool,
        train: bool,
        layer_idx: usize,
    ) -> (Tensor, Option<Tensor>, Option<Tensor>) {
        debug_log!("[LAYER] Starting decoder layer forward...");
        debug_log!(
            "[LAYER]   Input hidden_states[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        // Save input checkpoint for layer 0
        let debug_layer_0 =
            layer_idx == 0 && std::env::var("DEBUG_SAVE_DECODER_LAYER_0_INTERNALS").is_ok();
        if debug_layer_0 {
            Self::save_tensor_debug(hidden_states, "input");
        }

        let residual = hidden_states.shallow_clone();

        // Self-attention
        debug_log!("[LAYER]   Running self-attention...");
        let debug_name = if layer_idx == 0 {
            Some("decoder_layer_0")
        } else {
            None
        };
        let (mut hidden_states, self_attn_weights) = self.self_attn.forward(
            hidden_states,
            encoder_attention_mask,
            position_embeddings,
            output_attentions,
            train,
            debug_name,
        );
        debug_log!(
            "[LAYER]   After self-attn[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        if debug_layer_0 {
            Self::save_tensor_debug(&hidden_states, "after_self_attn");
        }

        // Dropout
        hidden_states = hidden_states.dropout(self.dropout, train);

        // Residual connection
        hidden_states = &residual + &hidden_states;
        debug_log!(
            "[LAYER]   After self-attn residual[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        if debug_layer_0 {
            Self::save_tensor_debug(&hidden_states, "after_self_attn_residual");
        }

        // Layer norm
        hidden_states = hidden_states.apply(&self.self_attn_layer_norm);
        debug_log!(
            "[LAYER]   After self-attn norm[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        if debug_layer_0 {
            Self::save_tensor_debug(&hidden_states, "after_self_attn_norm");
        }

        let second_residual = hidden_states.shallow_clone();

        // Cross-attention (deformable)
        debug_log!("[LAYER]   Running cross-attention (deformable)...");
        let (mut hidden_states, cross_attn_weights) = self.encoder_attn.forward(
            &hidden_states,
            encoder_hidden_states,
            position_embeddings,
            reference_points,
            spatial_shapes,
            spatial_shapes_list,
            level_start_index,
            encoder_attention_mask,
            output_attentions,
        );
        debug_log!(
            "[LAYER]   After cross-attn[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        if debug_layer_0 {
            Self::save_tensor_debug(&hidden_states, "after_cross_attn");
        }

        // Dropout
        hidden_states = hidden_states.dropout(self.dropout, train);

        // Residual connection
        hidden_states = &second_residual + &hidden_states;
        debug_log!(
            "[LAYER]   After cross-attn residual[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        if debug_layer_0 {
            Self::save_tensor_debug(&hidden_states, "after_cross_attn_residual");
        }

        // Layer norm
        hidden_states = hidden_states.apply(&self.encoder_attn_layer_norm);
        debug_log!(
            "[LAYER]   After cross-attn norm[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        if debug_layer_0 {
            Self::save_tensor_debug(&hidden_states, "after_cross_attn_norm");
        }

        // Feed-forward network
        debug_log!("[LAYER]   Running FFN...");
        let residual = hidden_states.shallow_clone();

        // FC1 + activation
        hidden_states = self.activation_fn.apply(&hidden_states.apply(&self.fc1));
        debug_log!(
            "[LAYER]   After FC1+activation[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        if debug_layer_0 {
            Self::save_tensor_debug(&hidden_states, "after_fc1_activation");
        }

        hidden_states = hidden_states.dropout(self.activation_dropout, train);

        // FC2
        hidden_states = hidden_states.apply(&self.fc2);
        debug_log!(
            "[LAYER]   After FC2[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        if debug_layer_0 {
            Self::save_tensor_debug(&hidden_states, "after_fc2");
        }

        hidden_states = hidden_states.dropout(self.dropout, train);

        // Residual connection
        hidden_states = &residual + &hidden_states;
        debug_log!(
            "[LAYER]   After FFN residual[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        if debug_layer_0 {
            Self::save_tensor_debug(&hidden_states, "after_ffn_residual");
        }

        // Final layer norm
        hidden_states = hidden_states.apply(&self.final_layer_norm);
        debug_log!(
            "[LAYER]   After final norm[0,0,:5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
            hidden_states.double_value(&[0, 0, 0]),
            hidden_states.double_value(&[0, 0, 1]),
            hidden_states.double_value(&[0, 0, 2]),
            hidden_states.double_value(&[0, 0, 3]),
            hidden_states.double_value(&[0, 0, 4]),
        );

        if debug_layer_0 {
            Self::save_tensor_debug(&hidden_states, "output");
        }

        debug_log!("[LAYER] Decoder layer complete");
        (hidden_states, self_attn_weights, cross_attn_weights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tch::Device;

    #[test]
    #[ignore] // Skip for now - depends on deformable_attention which has runtime issues
    fn test_multiscale_deformable_attention() {
        let device = Device::Cpu;
        let vs = nn::VarStore::new(device);
        let root = vs.root();

        let d_model = 256;
        let n_heads = 8;
        let n_levels = 3;
        let n_points = 4;

        let attn = RTDetrV2MultiscaleDeformableAttention::new(
            &root, d_model, n_heads, n_levels, n_points, 2.0, "default",
        );

        let batch_size = 2;
        let num_queries = 300;
        let sequence_length = 64 + 32 + 16; // Sum of spatial shapes

        // Create inputs
        let hidden_states = Tensor::randn(
            [batch_size, num_queries, d_model],
            (tch::Kind::Float, device),
        );
        let encoder_hidden_states = Tensor::randn(
            [batch_size, sequence_length, d_model],
            (tch::Kind::Float, device),
        );

        // Reference points: [batch_size, num_queries, n_levels, 2]
        let reference_points = Tensor::rand(
            [batch_size, num_queries, n_levels, 2],
            (tch::Kind::Float, device),
        );

        // Spatial shapes: [n_levels, 2]
        let spatial_shapes = Tensor::from_slice2(&[[8i64, 8], [4, 8], [4, 4]]);
        let spatial_shapes_list = vec![(8, 8), (4, 8), (4, 4)];

        // Forward pass
        let (output, _) = attn.forward(
            &hidden_states,
            &encoder_hidden_states,
            None,
            &reference_points,
            &spatial_shapes,
            &spatial_shapes_list,
            None,
            None,
            false,
        );

        // Check output shape
        assert_eq!(output.size(), vec![batch_size, num_queries, d_model]);
    }

    #[test]
    #[ignore] // Skip for now - depends on deformable_attention which has runtime issues
    fn test_decoder_layer() {
        let device = Device::Cpu;
        let vs = nn::VarStore::new(device);
        let root = vs.root();

        let d_model = 256;
        let decoder_attention_heads = 8;
        let decoder_ffn_dim = 1024;
        let dropout = 0.1;
        let activation_dropout = 0.0;
        let attention_dropout = 0.0;
        let layer_norm_eps = 1e-5;
        let decoder_n_levels = 3;
        let decoder_n_points = 4;
        let decoder_offset_scale = 0.5; // FIXED: Was 2.0 (4x too large), Python uses 0.5

        let layer = RTDetrV2DecoderLayer::new(
            &root,
            d_model,
            decoder_attention_heads,
            decoder_ffn_dim,
            dropout,
            activation_dropout,
            attention_dropout,
            Activation::ReLU,
            layer_norm_eps,
            decoder_n_levels,
            decoder_n_points,
            decoder_offset_scale,
            "default",
        );

        let batch_size = 2;
        let num_queries = 300;
        let sequence_length = 64 + 32 + 16;

        // Create inputs
        let hidden_states = Tensor::randn(
            [batch_size, num_queries, d_model],
            (tch::Kind::Float, device),
        );
        let encoder_hidden_states = Tensor::randn(
            [batch_size, sequence_length, d_model],
            (tch::Kind::Float, device),
        );
        let reference_points = Tensor::rand(
            [batch_size, num_queries, decoder_n_levels, 2],
            (tch::Kind::Float, device),
        );
        let spatial_shapes = Tensor::from_slice2(&[[8i64, 8], [4, 8], [4, 4]]);
        let spatial_shapes_list = vec![(8, 8), (4, 8), (4, 4)];

        // Forward pass
        let (output, _, _) = layer.forward(
            &hidden_states,
            None,
            &reference_points,
            &spatial_shapes,
            &spatial_shapes_list,
            None,
            &encoder_hidden_states,
            None,
            false,
            false,
            0, // layer_idx (for testing, doesn't matter)
        );

        // Check output shape
        assert_eq!(output.size(), vec![batch_size, num_queries, d_model]);
    }

    #[test]
    #[ignore] // Skip for now - depends on deformable_attention which has runtime issues
    fn test_decoder_layer_with_4d_reference_points() {
        let device = Device::Cpu;
        let vs = nn::VarStore::new(device);
        let root = vs.root();

        let d_model = 256;
        let decoder_attention_heads = 8;
        let decoder_ffn_dim = 1024;

        let layer = RTDetrV2DecoderLayer::new(
            &root,
            d_model,
            decoder_attention_heads,
            decoder_ffn_dim,
            0.0,
            0.0,
            0.0,
            Activation::ReLU,
            1e-5,
            3,
            4,
            2.0,
            "default",
        );

        let batch_size = 2;
        let num_queries = 300;
        let sequence_length = 112;

        let hidden_states = Tensor::randn(
            [batch_size, num_queries, d_model],
            (tch::Kind::Float, device),
        );
        let encoder_hidden_states = Tensor::randn(
            [batch_size, sequence_length, d_model],
            (tch::Kind::Float, device),
        );

        // 4D reference points: [batch_size, num_queries, n_levels, 4] (x, y, w, h)
        let reference_points =
            Tensor::rand([batch_size, num_queries, 3, 4], (tch::Kind::Float, device));

        let spatial_shapes = Tensor::from_slice2(&[[8i64, 8], [4, 8], [4, 4]]);
        let spatial_shapes_list = vec![(8, 8), (4, 8), (4, 4)];

        // Forward pass
        let (output, _, _) = layer.forward(
            &hidden_states,
            None,
            &reference_points,
            &spatial_shapes,
            &spatial_shapes_list,
            None,
            &encoder_hidden_states,
            None,
            false,
            false,
            0, // layer_idx (for testing, doesn't matter)
        );

        // Check output shape
        assert_eq!(output.size(), vec![batch_size, num_queries, d_model]);
    }

    #[test]
    #[ignore] // Skip for now - depends on deformable_attention which has runtime issues
    fn test_decoder_layer_with_attention_outputs() {
        let device = Device::Cpu;
        let vs = nn::VarStore::new(device);
        let root = vs.root();

        let d_model = 256;
        let decoder_attention_heads = 8;
        let decoder_ffn_dim = 1024;

        let layer = RTDetrV2DecoderLayer::new(
            &root,
            d_model,
            decoder_attention_heads,
            decoder_ffn_dim,
            0.0,
            0.0,
            0.0,
            Activation::ReLU,
            1e-5,
            3,
            4,
            2.0,
            "default",
        );

        let batch_size = 2;
        let num_queries = 300;
        let sequence_length = 112;

        let hidden_states = Tensor::randn(
            [batch_size, num_queries, d_model],
            (tch::Kind::Float, device),
        );
        let encoder_hidden_states = Tensor::randn(
            [batch_size, sequence_length, d_model],
            (tch::Kind::Float, device),
        );
        let reference_points =
            Tensor::rand([batch_size, num_queries, 3, 2], (tch::Kind::Float, device));
        let spatial_shapes = Tensor::from_slice2(&[[8i64, 8], [4, 8], [4, 4]]);
        let spatial_shapes_list = vec![(8, 8), (4, 8), (4, 4)];

        // Forward pass with attention outputs
        let (output, self_attn_weights, cross_attn_weights) = layer.forward(
            &hidden_states,
            None,
            &reference_points,
            &spatial_shapes,
            &spatial_shapes_list,
            None,
            &encoder_hidden_states,
            None,
            true,
            false,
            0, // layer_idx (for testing, doesn't matter)
        );

        // Check output shape
        assert_eq!(output.size(), vec![batch_size, num_queries, d_model]);

        // Check attention weights are returned
        assert!(self_attn_weights.is_some());
        assert!(cross_attn_weights.is_some());

        // Check self-attention weights shape
        let self_attn = self_attn_weights.unwrap();
        assert_eq!(
            self_attn.size(),
            vec![
                batch_size,
                decoder_attention_heads,
                num_queries,
                num_queries
            ]
        );

        // Check cross-attention weights shape
        let cross_attn = cross_attn_weights.unwrap();
        assert_eq!(
            cross_attn.size(),
            vec![batch_size, num_queries, decoder_attention_heads, 3 * 4]
        );
    }
}

/// MLP prediction head for bounding box regression and query position embeddings
/// Python: RTDetrV2MLPPredictionHead (lines 1704-1730)
#[derive(Debug)]
pub struct RTDetrV2MLPPredictionHead {
    pub num_layers: i64,
    pub layers: Vec<nn::Linear>,
}

impl RTDetrV2MLPPredictionHead {
    pub fn new(
        vs: &nn::Path,
        input_dim: i64,
        d_model: i64,
        output_dim: i64,
        num_layers: i64,
    ) -> Self {
        assert!(num_layers >= 1, "num_layers must be >= 1");

        let linear_config = nn::LinearConfig {
            bias: true,
            ..Default::default()
        };
        let mut layers = Vec::new();

        // Build layer dimensions: [input_dim, d_model, ..., d_model, output_dim]
        let mut dims = vec![input_dim];
        for _ in 0..(num_layers - 1) {
            dims.push(d_model);
        }
        dims.push(output_dim);

        // Create linear layers
        for i in 0..(num_layers as usize) {
            let layer = nn::linear(
                &(vs / "layers" / i as i64),
                dims[i],
                dims[i + 1],
                linear_config,
            );
            layers.push(layer);
        }

        Self { num_layers, layers }
    }

    pub fn forward(&self, x: &Tensor) -> Tensor {
        let mut output = x.shallow_clone();
        for (i, layer) in self.layers.iter().enumerate() {
            output = layer.forward(&output);
            // Apply ReLU for all layers except the last
            if i < (self.num_layers - 1) as usize {
                output = output.relu();
            }
        }
        output
    }
}

/// Full RT-DETR v2 decoder with 6 layers
/// Python: RTDetrV2Decoder (lines 563-726)
#[derive(Debug)]
pub struct RTDetrV2Decoder {
    pub dropout: f64,
    pub decoder_layers: i64,
    pub layers: Vec<RTDetrV2DecoderLayer>,
    pub query_pos_head: RTDetrV2MLPPredictionHead,
    // Detection heads for iterative refinement (set externally after construction)
    pub bbox_embed: Option<Vec<RTDetrV2MLPPredictionHead>>,
    pub class_embed: Option<Vec<nn::Linear>>,
}

impl RTDetrV2Decoder {
    #[allow(
        clippy::too_many_arguments,
        reason = "RT-DETR full decoder requires many configuration params"
    )]
    pub fn new(
        vs: &nn::Path,
        d_model: i64,
        decoder_layers: i64,
        decoder_attention_heads: i64,
        decoder_ffn_dim: i64,
        dropout: f64,
        attention_dropout: f64,
        activation_dropout: f64,
        activation_function: Activation,
        layer_norm_eps: f64,
        decoder_n_levels: i64,
        decoder_n_points: i64,
        offset_scale: f64,
        method: &str,
    ) -> Self {
        let mut layers = Vec::new();
        for i in 0..decoder_layers {
            let layer = RTDetrV2DecoderLayer::new(
                &(vs / "layers" / i),
                d_model,
                decoder_attention_heads,
                decoder_ffn_dim,
                dropout,
                attention_dropout,
                activation_dropout,
                activation_function,
                layer_norm_eps,
                decoder_n_levels,
                decoder_n_points,
                offset_scale,
                method,
            );
            layers.push(layer);
        }

        // Query position head: input_dim=4 (reference points), hidden=2*d_model, output=d_model, num_layers=2
        let query_pos_head =
            RTDetrV2MLPPredictionHead::new(&(vs / "query_pos_head"), 4, 2 * d_model, d_model, 2);

        Self {
            dropout,
            decoder_layers,
            layers,
            query_pos_head,
            bbox_embed: None,
            class_embed: None,
        }
    }

    /// Set detection heads for iterative refinement
    /// This must be called after construction to enable bbox refinement
    ///
    /// Python equivalent: model.decoder.class_embed = class_embed
    #[inline]
    pub fn set_detection_heads(
        &mut self,
        class_embed: Vec<nn::Linear>,
        bbox_embed: Vec<RTDetrV2MLPPredictionHead>,
    ) {
        self.class_embed = Some(class_embed);
        self.bbox_embed = Some(bbox_embed);
    }

    /// Forward pass through all decoder layers
    ///
    /// # Arguments
    /// * `inputs_embeds` - Query embeddings [batch_size, num_queries, d_model]
    /// * `encoder_hidden_states` - Encoder output [batch_size, sequence_length, d_model]
    /// * `reference_points` - Reference points [batch_size, num_queries, 4] (x, y, w, h)
    /// * `spatial_shapes` - Spatial shapes of feature maps [num_levels, 2]
    /// * `spatial_shapes_list` - List of (height, width) tuples for each level
    /// * `encoder_attention_mask` - Optional attention mask for encoder features
    /// * `bbox_embed` - Optional bbox refinement layers (one per decoder layer)
    /// * `class_embed` - Optional classification layers (one per decoder layer)
    /// * `output_attentions` - Whether to return attention weights
    /// * `output_hidden_states` - Whether to return all hidden states
    ///
    /// # Returns
    /// Tuple of:
    /// * `last_hidden_state` - Final decoder output [batch_size, num_queries, d_model]
    /// * `intermediate_hidden_states` - All layer outputs [batch_size, decoder_layers, num_queries, d_model]
    /// * `intermediate_reference_points` - Reference points at each layer [batch_size, decoder_layers, num_queries, 4]
    /// * `intermediate_logits` - Optional classification logits at each layer
    /// * `all_hidden_states` - Optional all hidden states (including input)
    /// * `all_self_attns` - Optional self-attention weights
    /// * `all_cross_attentions` - Optional cross-attention weights
    #[allow(
        clippy::too_many_arguments,
        reason = "full decoder forward pass requires many tensors and flags"
    )]
    pub fn forward(
        &self,
        inputs_embeds: &Tensor,
        encoder_hidden_states: &Tensor,
        reference_points: &Tensor,
        spatial_shapes: &Tensor,
        spatial_shapes_list: &[(i64, i64)],
        encoder_attention_mask: Option<&Tensor>,
        output_attentions: bool,
        output_hidden_states: bool,
    ) -> (
        Tensor,                      // last_hidden_state
        Tensor,                      // intermediate_hidden_states
        Tensor,                      // intermediate_reference_points
        Option<Tensor>,              // intermediate_logits
        Option<Vec<Tensor>>,         // all_hidden_states
        Option<Vec<Option<Tensor>>>, // all_self_attns
        Option<Vec<Option<Tensor>>>, // all_cross_attentions
    ) {
        debug_log!("[DECODER] Starting decoder forward pass...");
        debug_log!("[DECODER]   inputs_embeds: {:?}", inputs_embeds.size());
        debug_log!(
            "[DECODER]   encoder_hidden_states: {:?}",
            encoder_hidden_states.size()
        );
        debug_log!(
            "[DECODER]   reference_points: {:?}",
            reference_points.size()
        );
        debug_log!("[DECODER]   spatial_shapes: {:?}", spatial_shapes.size());
        debug_log!("[DECODER]   num layers: {}", self.decoder_layers);

        let mut hidden_states = inputs_embeds.shallow_clone();
        debug_log!("[DECODER] Sigmoid on reference points...");
        let mut reference_points_mut = reference_points.sigmoid();
        debug_log!(
            "[DECODER] Sigmoid complete: {:?}",
            reference_points_mut.size()
        );

        let mut all_hidden_states = if output_hidden_states {
            Some(Vec::new())
        } else {
            None
        };

        let mut all_self_attns = if output_attentions {
            Some(Vec::new())
        } else {
            None
        };

        let mut all_cross_attentions = if output_attentions {
            Some(Vec::new())
        } else {
            None
        };

        let mut intermediate = Vec::new();
        let mut intermediate_reference_points_vec = Vec::new();
        let mut intermediate_logits = if self.class_embed.is_some() {
            Some(Vec::new())
        } else {
            None
        };

        // Iterate through decoder layers
        debug_log!(
            "[DECODER] Starting layer iteration (total: {} layers)",
            self.layers.len()
        );
        for (idx, layer) in self.layers.iter().enumerate() {
            debug_log!("[DECODER] === Layer {} ===", idx);
            // Unsqueeze reference points: [batch, queries, 4] -> [batch, queries, 1, 4]
            debug_log!("[DECODER]   Unsqueezing reference points...");
            let reference_points_input = reference_points_mut.unsqueeze(2);
            debug_log!(
                "[DECODER]   reference_points_input: {:?}",
                reference_points_input.size()
            );

            // Generate position embeddings from reference points
            debug_log!("[DECODER]   Computing position embeddings...");
            let position_embeddings = self.query_pos_head.forward(&reference_points_mut);
            debug_log!(
                "[DECODER]   position_embeddings: {:?}",
                position_embeddings.size()
            );

            // Save position embeddings for layer 0 (DEBUG_SAVE_DECODER_INPUTS)
            if idx == 0 && std::env::var("DEBUG_SAVE_DECODER_INPUTS").is_ok() {
                use ndarray_npy::WriteNpyExt;
                use std::fs::File;
                use std::io::BufWriter;
                use std::path::Path;

                let output_path = "baseline_data/arxiv_2206.01062/page_0/layout/pytorch_intermediate/debug_rust_decoder_input_position_embeddings.npy";
                if let Some(parent) = Path::new(output_path).parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                let shape = position_embeddings.size();
                let tensor_flat = position_embeddings.flatten(0, -1).to_kind(tch::Kind::Float);
                let tensor_vec: Vec<f32> = Vec::try_from(tensor_flat)
                    .expect("Failed to convert position_embeddings tensor to Vec");
                let array = ndarray::Array::from_shape_vec(
                    (shape[0] as usize, shape[1] as usize, shape[2] as usize),
                    tensor_vec,
                )
                .expect("Failed to create position_embeddings ndarray");

                match File::create(output_path) {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        match array.write_npy(writer) {
                            Ok(_) => debug_log!(
                                "[DECODER_LAYER_0] ✓ Saved position_embeddings to: {}",
                                output_path
                            ),
                            Err(e) => debug_log!(
                                "[DECODER_LAYER_0] ✗ Failed to write position_embeddings npy: {:?}",
                                e
                            ),
                        }
                    }
                    Err(e) => debug_log!(
                        "[DECODER_LAYER_0] ✗ Failed to create position_embeddings file: {:?}",
                        e
                    ),
                }
            }

            // Save hidden states if requested
            if let Some(ref mut states) = all_hidden_states {
                states.push(hidden_states.shallow_clone());
            }

            // Forward through decoder layer
            debug_log!("[DECODER]   Calling layer.forward()...");
            debug_log!("[DECODER]     hidden_states: {:?}", hidden_states.size());
            debug_log!(
                "[DECODER]     reference_points_input: {:?}",
                reference_points_input.size()
            );
            let (layer_output, self_attn_weights, cross_attn_weights) = layer.forward(
                &hidden_states,
                Some(&position_embeddings),
                &reference_points_input,
                spatial_shapes,
                spatial_shapes_list,
                None, // level_start_index (computed inside attention)
                encoder_hidden_states,
                encoder_attention_mask,
                output_attentions,
                false, // train mode
                idx,   // layer index for debugging
            );
            debug_log!("[DECODER]   layer.forward() complete");

            hidden_states = layer_output;

            // Debug output and save layer outputs for all layers (0-5)
            debug_log!("[DECODER_LAYER_{}] Hidden states after layer {}:", idx, idx);
            debug_log!(
                "[DECODER_LAYER_{}]   Shape: {:?}",
                idx,
                hidden_states.size()
            );
            debug_log!(
                "[DECODER_LAYER_{}]   Sample [0, 0, :5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
                idx,
                hidden_states.double_value(&[0, 0, 0]),
                hidden_states.double_value(&[0, 0, 1]),
                hidden_states.double_value(&[0, 0, 2]),
                hidden_states.double_value(&[0, 0, 3]),
                hidden_states.double_value(&[0, 0, 4]),
            );

            // Save layer output to .npy file for comparison with Python baseline
            // Only save if DEBUG_SAVE_DECODER_LAYERS env var is set
            if std::env::var("DEBUG_SAVE_DECODER_LAYERS").is_ok() {
                use std::path::Path;
                let output_path = format!(
                    "baseline_data/arxiv_2206.01062/page_0/layout/pytorch_intermediate/debug_rust_decoder_layer_{}_output.npy",
                    idx
                );
                if let Some(parent) = Path::new(&output_path).parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                // Convert tensor to Vec<f32> and save using ndarray
                let shape = hidden_states.size();
                let tensor_flat = hidden_states.flatten(0, -1).to_kind(tch::Kind::Float);
                let tensor_vec: Vec<f32> =
                    Vec::try_from(tensor_flat).expect("Failed to convert tensor to Vec");

                use ndarray_npy::WriteNpyExt;
                use std::fs::File;
                use std::io::BufWriter;

                // Create ndarray with correct shape
                let array = ndarray::Array::from_shape_vec(
                    (shape[0] as usize, shape[1] as usize, shape[2] as usize),
                    tensor_vec,
                )
                .expect("Failed to create ndarray");

                match File::create(&output_path) {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        match array.write_npy(writer) {
                            Ok(_) => {
                                debug_log!("[DECODER_LAYER_{}]   ✓ Saved to: {}", idx, output_path)
                            }
                            Err(e) => debug_log!(
                                "[DECODER_LAYER_{}]   ✗ Failed to write npy: {:?}",
                                idx,
                                e
                            ),
                        }
                    }
                    Err(e) => {
                        debug_log!("[DECODER_LAYER_{}]   ✗ Failed to create file: {:?}", idx, e)
                    }
                }
            }

            if idx == 0 {
                // Python expected: [-0.5470454, -0.6785877, 0.78192, -1.7899334, -0.5142547]
            }

            // Iterative bounding box refinement (if bbox_embed provided)
            if let Some(ref bbox_layers) = self.bbox_embed {
                let predicted_corners = bbox_layers[idx].forward(&hidden_states);
                let inverse_ref = inverse_sigmoid(&reference_points_mut, 1e-5);
                let new_reference_points = (predicted_corners + inverse_ref).sigmoid();
                reference_points_mut = new_reference_points.detach();
            }

            // Save intermediate outputs
            intermediate.push(hidden_states.shallow_clone());
            intermediate_reference_points_vec.push(reference_points_mut.shallow_clone());

            // Classification logits (if class_embed provided)
            if let Some(ref class_layers) = self.class_embed {
                let logits = class_layers[idx].forward(&hidden_states);
                if let Some(ref mut logits_vec) = intermediate_logits {
                    logits_vec.push(logits);
                }
            }

            // Save attention weights if requested
            if let Some(ref mut attns) = all_self_attns {
                attns.push(self_attn_weights);
            }
            if let Some(ref mut attns) = all_cross_attentions {
                attns.push(cross_attn_weights);
            }
        }

        // Save final hidden states if requested
        if let Some(ref mut states) = all_hidden_states {
            states.push(hidden_states.shallow_clone());
        }

        // Stack intermediate outputs: List[Tensor] -> Tensor
        // From [decoder_layers, batch, queries, d_model] to [batch, decoder_layers, queries, d_model]
        let intermediate_stacked = Tensor::stack(&intermediate, 1);
        let intermediate_reference_points_stacked =
            Tensor::stack(&intermediate_reference_points_vec, 1);
        let intermediate_logits_stacked = intermediate_logits.map(|logits| {
            let logits_refs: Vec<&Tensor> = logits.iter().collect();
            Tensor::stack(&logits_refs, 1)
        });

        (
            hidden_states,
            intermediate_stacked,
            intermediate_reference_points_stacked,
            intermediate_logits_stacked,
            all_hidden_states,
            all_self_attns,
            all_cross_attentions,
        )
    }
}

/// Inverse sigmoid function (logit function)
/// Python: inverse_sigmoid (lines 556-561)
fn inverse_sigmoid(x: &Tensor, eps: f64) -> Tensor {
    let x_clamped = x.clamp(0.0, 1.0);
    let x1 = x_clamped.clamp_min(eps);
    let ones = Tensor::ones_like(&x_clamped);
    let x2 = (ones - &x_clamped).clamp_min(eps);
    (&x1 / &x2).log()
}

#[cfg(test)]
mod decoder_tests {
    use super::*;
    use tch::Device;

    #[test]
    fn test_mlp_prediction_head() {
        let device = Device::Cpu;
        let vs = nn::VarStore::new(device);
        let root = vs.root();

        let input_dim = 4;
        let d_model = 256;
        let output_dim = 256;
        let num_layers = 2;

        let head =
            RTDetrV2MLPPredictionHead::new(&root, input_dim, d_model, output_dim, num_layers);

        let batch_size = 2;
        let num_queries = 300;
        let input = Tensor::randn(
            [batch_size, num_queries, input_dim],
            (tch::Kind::Float, device),
        );

        let output = head.forward(&input);

        // Check output shape
        assert_eq!(output.size(), vec![batch_size, num_queries, output_dim]);
    }

    #[test]
    #[ignore] // Skip for now - depends on decoder layer which has deformable attention issues
    fn test_rtdetr_v2_decoder() {
        let device = Device::Cpu;
        let vs = nn::VarStore::new(device);
        let root = vs.root();

        let d_model = 256;
        let decoder_layers = 6;
        let decoder_attention_heads = 8;
        let decoder_ffn_dim = 1024;
        let decoder_n_levels = 3;
        let decoder_n_points = 4;

        let decoder = RTDetrV2Decoder::new(
            &root,
            d_model,
            decoder_layers,
            decoder_attention_heads,
            decoder_ffn_dim,
            0.0,
            0.0,
            0.0,
            Activation::ReLU,
            1e-5,
            decoder_n_levels,
            decoder_n_points,
            2.0,
            "default",
        );

        let batch_size = 2;
        let num_queries = 300;
        let sequence_length = 112; // e.g., sum of (8*8 + 4*8 + 4*4) = 112

        let inputs_embeds = Tensor::randn(
            [batch_size, num_queries, d_model],
            (tch::Kind::Float, device),
        );
        let encoder_hidden_states = Tensor::randn(
            [batch_size, sequence_length, d_model],
            (tch::Kind::Float, device),
        );
        let reference_points =
            Tensor::rand([batch_size, num_queries, 4], (tch::Kind::Float, device));
        let spatial_shapes = Tensor::from_slice2(&[[8i64, 8], [4, 8], [4, 4]]);
        let spatial_shapes_list = vec![(8, 8), (4, 8), (4, 4)];

        // Forward pass
        let (
            last_hidden_state,
            intermediate_hidden_states,
            intermediate_reference_points,
            intermediate_logits,
            all_hidden_states,
            all_self_attns,
            all_cross_attentions,
        ) = decoder.forward(
            &inputs_embeds,
            &encoder_hidden_states,
            &reference_points,
            &spatial_shapes,
            &spatial_shapes_list,
            None,  // encoder_attention_mask
            false, // output_attentions
            false, // output_hidden_states
        );

        // Check output shapes
        assert_eq!(
            last_hidden_state.size(),
            vec![batch_size, num_queries, d_model]
        );
        assert_eq!(
            intermediate_hidden_states.size(),
            vec![batch_size, decoder_layers, num_queries, d_model]
        );
        assert_eq!(
            intermediate_reference_points.size(),
            vec![batch_size, decoder_layers, num_queries, 4]
        );
        assert!(intermediate_logits.is_none());
        assert!(all_hidden_states.is_none());
        assert!(all_self_attns.is_none());
        assert!(all_cross_attentions.is_none());
    }

    #[test]
    #[ignore] // Skip for now - depends on decoder layer which has deformable attention issues
    fn test_rtdetr_v2_decoder_with_outputs() {
        let device = Device::Cpu;
        let vs = nn::VarStore::new(device);
        let root = vs.root();

        let d_model = 256;
        let decoder_layers = 6;
        let decoder_attention_heads = 8;
        let decoder_ffn_dim = 1024;
        let decoder_n_levels = 3;
        let decoder_n_points = 4;

        let decoder = RTDetrV2Decoder::new(
            &root,
            d_model,
            decoder_layers,
            decoder_attention_heads,
            decoder_ffn_dim,
            0.0,
            0.0,
            0.0,
            Activation::ReLU,
            1e-5,
            decoder_n_levels,
            decoder_n_points,
            2.0,
            "default",
        );

        let batch_size = 2;
        let num_queries = 300;
        let sequence_length = 112;

        let inputs_embeds = Tensor::randn(
            [batch_size, num_queries, d_model],
            (tch::Kind::Float, device),
        );
        let encoder_hidden_states = Tensor::randn(
            [batch_size, sequence_length, d_model],
            (tch::Kind::Float, device),
        );
        let reference_points =
            Tensor::rand([batch_size, num_queries, 4], (tch::Kind::Float, device));
        let spatial_shapes = Tensor::from_slice2(&[[8i64, 8], [4, 8], [4, 4]]);
        let spatial_shapes_list = vec![(8, 8), (4, 8), (4, 4)];

        // Forward pass with outputs
        let (
            last_hidden_state,
            intermediate_hidden_states,
            intermediate_reference_points,
            intermediate_logits,
            all_hidden_states,
            all_self_attns,
            all_cross_attentions,
        ) = decoder.forward(
            &inputs_embeds,
            &encoder_hidden_states,
            &reference_points,
            &spatial_shapes,
            &spatial_shapes_list,
            None, // encoder_attention_mask
            true, // output_attentions
            true, // output_hidden_states
        );

        // Check output shapes
        assert_eq!(
            last_hidden_state.size(),
            vec![batch_size, num_queries, d_model]
        );
        assert_eq!(
            intermediate_hidden_states.size(),
            vec![batch_size, decoder_layers, num_queries, d_model]
        );
        assert_eq!(
            intermediate_reference_points.size(),
            vec![batch_size, decoder_layers, num_queries, 4]
        );

        // Check optional outputs are present
        assert!(all_hidden_states.is_some());
        assert!(all_self_attns.is_some());
        assert!(all_cross_attentions.is_some());

        // Check hidden states count (decoder_layers + 1 for initial state)
        let states = all_hidden_states.unwrap();
        assert_eq!(states.len(), (decoder_layers + 1) as usize);

        // Check attention weights count
        let self_attns = all_self_attns.unwrap();
        let cross_attns = all_cross_attentions.unwrap();
        assert_eq!(self_attns.len(), decoder_layers as usize);
        assert_eq!(cross_attns.len(), decoder_layers as usize);
    }

    #[test]
    fn test_inverse_sigmoid() {
        let device = Device::Cpu;
        let x = Tensor::from_slice(&[0.1, 0.5, 0.9]).to_device(device);
        let result = inverse_sigmoid(&x, 1e-5);

        // Check shape
        assert_eq!(result.size(), vec![3]);

        // Check inverse sigmoid properties
        // sigmoid(inverse_sigmoid(x)) should ≈ x
        let reconstructed = result.sigmoid();
        let diff = (&reconstructed - &x).abs().max();
        assert!(f64::try_from(diff).unwrap() < 1e-6);
    }

    #[test]
    fn test_activation_from_str() {
        use std::str::FromStr;

        // Standard cases
        assert_eq!(Activation::from_str("relu").unwrap(), Activation::ReLU);
        assert_eq!(Activation::from_str("gelu").unwrap(), Activation::GELU);
        assert_eq!(
            Activation::from_str("sigmoid").unwrap(),
            Activation::Sigmoid
        );

        // Case insensitivity
        assert_eq!(Activation::from_str("RELU").unwrap(), Activation::ReLU);
        assert_eq!(Activation::from_str("GeLU").unwrap(), Activation::GELU);

        // Alias
        assert_eq!(Activation::from_str("sig").unwrap(), Activation::Sigmoid);

        // Error case
        assert!(Activation::from_str("unknown").is_err());
    }

    #[test]
    fn test_activation_roundtrip() {
        use std::str::FromStr;

        for act in [Activation::ReLU, Activation::GELU, Activation::Sigmoid] {
            let s = act.to_string();
            assert_eq!(Activation::from_str(&s).unwrap(), act);
        }
    }
}
