// Encoder building blocks for RT-DETR v2
// Ported from transformers/models/rt_detr_v2/modeling_rt_detr_v2.py:875-1042

// Debug logging macro - disabled by default for performance
// To enable: Uncomment the macro below
// macro_rules! debug_log { ($($arg:tt)*) => { log::warn!($($arg)*) }; }
macro_rules! debug_log {
    ($($arg:tt)*) => {
        ()
    };
}

// Profiling macro - disabled by default for performance
// To enable: Set environment variable PROFILE_ENCODER=1
macro_rules! profile_log { ($($arg:tt)*) => {
    if std::env::var("PROFILE_ENCODER").is_ok() {
        log::warn!($($arg)*)
    }
}; }

// FPN detailed profiling macro - disabled by default
// To enable: Set environment variable PROFILE_FPN=1 (also requires PROFILE_ENCODER=1)
macro_rules! profile_fpn { ($($arg:tt)*) => {
    if std::env::var("PROFILE_FPN").is_ok() && std::env::var("PROFILE_ENCODER").is_ok() {
        log::warn!($($arg)*)
    }
}; }

// CSPRepLayer detailed profiling macro - disabled by default
// To enable: Set environment variable PROFILE_CSREPLAYER=1 (also requires PROFILE_ENCODER=1 and PROFILE_FPN=1)
macro_rules! profile_csreplayer { ($($arg:tt)*) => {
    if std::env::var("PROFILE_CSREPLAYER").is_ok() && std::env::var("PROFILE_FPN").is_ok() && std::env::var("PROFILE_ENCODER").is_ok() {
        log::warn!($($arg)*)
    }
}; }

use super::transformer::{Activation, RTDetrV2MultiheadAttention};
use log::trace;
use std::time::Instant;
use tch::{nn, Kind, Tensor};

/// Helper function to save tensor as plain numpy array (not TorchScript)
fn save_tensor_as_numpy(tensor: &Tensor, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use ndarray::Array3;

    // Convert tensor to Vec<f32> using tch's efficient conversion
    let size = tensor.size();
    if size.len() != 3 {
        return Err(format!("Expected 3D tensor, got {:?}", size).into());
    }

    let (batch, seq_len, hidden_dim) = (size[0], size[1], size[2]);

    // Convert to CPU and ensure contiguous layout
    let tensor_cpu = tensor.to_kind(Kind::Float).to(tch::Device::Cpu);
    let tensor_flat = tensor_cpu.flatten(0, -1);

    // Use tch's try_into for efficient conversion
    let data: Vec<f32> = Vec::try_from(&tensor_flat)
        .map_err(|e| format!("Failed to convert tensor to Vec: {:?}", e))?;

    // Convert to ndarray
    let array = Array3::from_shape_vec(
        (batch as usize, seq_len as usize, hidden_dim as usize),
        data,
    )?;

    // Save as .npy
    ndarray_npy::write_npy(path, &array)?;

    Ok(())
}

/// Helper function to save 4D tensor as numpy array (for FPN/PAN blocks)
fn save_tensor_4d_as_numpy(tensor: &Tensor, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use ndarray::Array4;

    // Convert tensor to Vec<f32> using tch's efficient conversion
    let size = tensor.size();
    if size.len() != 4 {
        return Err(format!("Expected 4D tensor, got {:?}", size).into());
    }

    let (batch, channels, height, width) = (size[0], size[1], size[2], size[3]);

    // Convert to CPU and ensure contiguous layout
    let tensor_cpu = tensor.to_kind(Kind::Float).to(tch::Device::Cpu);
    let tensor_flat = tensor_cpu.flatten(0, -1);

    // Use tch's try_into for efficient conversion
    let data: Vec<f32> = Vec::try_from(&tensor_flat)
        .map_err(|e| format!("Failed to convert tensor to Vec: {:?}", e))?;

    // Convert to ndarray
    let array = Array4::from_shape_vec(
        (
            batch as usize,
            channels as usize,
            height as usize,
            width as usize,
        ),
        data,
    )?;

    // Save as .npy
    ndarray_npy::write_npy(path, &array)?;

    Ok(())
}

/// Convolutional layer with normalization and optional activation
/// Python: RTDetrV2ConvNormLayer (lines 875-893)
#[derive(Debug)]
pub struct RTDetrV2ConvNormLayer {
    pub conv: nn::Conv2D,
    pub norm: nn::BatchNorm,
    pub activation: Option<Activation>,
}

impl RTDetrV2ConvNormLayer {
    pub fn new(
        vs: &nn::Path,
        in_channels: i64,
        out_channels: i64,
        kernel_size: i64,
        stride: i64,
        padding: Option<i64>,
        activation: Option<Activation>,
    ) -> Self {
        let padding = padding.unwrap_or((kernel_size - 1) / 2);

        // BATCH NORM FUSION (N=500): Changed bias=false to bias=true
        // The fused batch norm bias is now loaded into conv.bias
        // Original (N=499): bias=false (separate batch norm layer)
        // New (N=500+): bias=true (batch norm fused into conv)
        let conv_config = nn::ConvConfig {
            stride,
            padding,
            bias: true, // Changed from false to true for batch norm fusion
            ..Default::default()
        };

        let conv = nn::conv2d(
            vs / "conv",
            in_channels,
            out_channels,
            kernel_size,
            conv_config,
        );

        // Batch norm layer still created for weight loading compatibility
        // but NOT used in forward() (batch norm is fused into conv weights)
        let norm = nn::batch_norm2d(vs / "norm", out_channels, Default::default());

        Self {
            conv,
            norm,
            activation,
        }
    }

    pub fn forward(&self, hidden_state: &Tensor) -> Tensor {
        // === BATCH NORM FUSION (N=500) ===
        // Batch norm is now FUSED into convolution weights at model load time
        // This eliminates the batch norm operation which consumes 64.1% of ConvNorm time (N=499 profiling)
        //
        // The fusion is performed in weights.rs::load_weights_into():
        //   W_fused = γ * W / sqrt(σ² + ε)
        //   b_fused = γ * (b - μ) / sqrt(σ² + ε) + β
        //
        // The self.norm field still exists for weight loading compatibility,
        // but we SKIP it in forward() since its effect is already in conv weights.
        //
        // Expected performance: 63.8% ConvNorm speedup → 0.4% pipeline speedup

        // Profile ConvNormLayer internals (conv, activation only - batch norm is fused)
        let start_conv = if std::env::var("PROFILE_CONVNORM").is_ok()
            && std::env::var("PROFILE_REPVGG").is_ok()
            && std::env::var("PROFILE_CSREPLAYER").is_ok()
            && std::env::var("PROFILE_FPN").is_ok()
            && std::env::var("PROFILE_ENCODER").is_ok()
        {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Apply convolution (with fused batch norm)
        let hidden_state = hidden_state.apply(&self.conv);

        // DEBUG: Verify fusion is working by checking output stats
        if std::env::var("DEBUG_FUSION").is_ok() {
            static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count < 3 {
                let mean = hidden_state.mean(Kind::Float);
                let std = hidden_state.std(false);
                log::debug!(
                    "🔍 ConvNorm #{} after conv (with fused BN): mean={:.6}, std={:.6}",
                    count,
                    f64::try_from(mean).unwrap(),
                    f64::try_from(std).unwrap()
                );
            }
        }

        if let Some(start) = start_conv {
            profile_csreplayer!(
                "          ConvNorm conv (fused): {:.3} ms",
                start.elapsed().as_secs_f64() * 1000.0
            );
        }

        // BATCH NORM SKIPPED - already fused into conv weights above
        // Old code (N=499 and earlier):
        //   let hidden_state = hidden_state.apply_t(&self.norm, false); // eval mode (frozen BN)
        //
        // New code (N=500+): SKIP batch norm operation entirely

        let start_activation = if std::env::var("PROFILE_CONVNORM").is_ok()
            && std::env::var("PROFILE_REPVGG").is_ok()
            && std::env::var("PROFILE_CSREPLAYER").is_ok()
            && std::env::var("PROFILE_FPN").is_ok()
            && std::env::var("PROFILE_ENCODER").is_ok()
        {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Apply activation (if present)
        let output = if let Some(activation) = &self.activation {
            activation.apply(&hidden_state)
        } else {
            hidden_state
        };

        if let Some(start) = start_activation {
            profile_csreplayer!(
                "          ConvNorm activation: {:.3} ms",
                start.elapsed().as_secs_f64() * 1000.0
            );
        }

        output
    }

    /// Forward pass with intermediate debug outputs
    pub fn forward_debug(&self, hidden_state: &Tensor, name: &str) -> Tensor {
        debug_log!("[{}] ConvNormLayer forward", name);
        debug_log!("[{}]   Input shape: {:?}", name, hidden_state.size());

        // Apply conv (with fused batch norm - N=626)
        let after_conv = hidden_state.apply(&self.conv);
        debug_log!(
            "[{}]   After conv (with fused BN): shape={:?}, mean={:.6}, max={:.6}",
            name,
            after_conv.size(),
            after_conv.mean(tch::Kind::Float).double_value(&[]),
            after_conv.abs().max().double_value(&[])
        );

        // Save after conv (with fused BN)
        if let Err(e) =
            save_tensor_4d_as_numpy(&after_conv, &format!("debug_rust_{}_after_conv.npy", name))
        {
            debug_log!("[WARNING] Failed to save after_conv: {}", e);
        }

        // BATCH NORM SKIPPED - already fused into conv weights (N=626 fix)
        // Old code (N=625 and earlier): Applied BN twice (once fused, once here) causing 57% error!
        //   let after_bn = after_conv.apply_t(&self.norm, false);
        //
        // New code (N=626+): Skip BN here since it's already in conv weights
        let after_bn = after_conv.shallow_clone();

        // Save after batchnorm (same as after_conv since BN is fused)
        // Keeping this for compatibility with debug scripts
        if let Err(e) =
            save_tensor_4d_as_numpy(&after_bn, &format!("debug_rust_{}_after_bn.npy", name))
        {
            debug_log!("[WARNING] Failed to save after_bn: {}", e);
        }

        // Apply activation
        let output = if let Some(activation) = &self.activation {
            let activated = activation.apply(&after_bn);
            debug_log!(
                "[{}]   After activation: shape={:?}, mean={:.6}, max={:.6}",
                name,
                activated.size(),
                activated.mean(tch::Kind::Float).double_value(&[]),
                activated.abs().max().double_value(&[])
            );

            // Save after activation
            if let Err(e) = save_tensor_4d_as_numpy(
                &activated,
                &format!("debug_rust_{}_after_activation.npy", name),
            ) {
                debug_log!("[WARNING] Failed to save after_activation: {}", e);
            }

            activated
        } else {
            after_bn
        };

        output
    }
}

/// RepVGG architecture block from "RepVGG: Making VGG-style ConvNets Great Again"
/// Python: RTDetrV2RepVggBlock (lines 979-995)
#[derive(Debug)]
pub struct RTDetrV2RepVggBlock {
    pub conv1: RTDetrV2ConvNormLayer,
    pub conv2: RTDetrV2ConvNormLayer,
    pub activation: Option<Activation>,
}

impl RTDetrV2RepVggBlock {
    pub fn new(
        vs: &nn::Path,
        hidden_channels: i64,
        hidden_expansion: f64,
        activation: Option<Activation>,
    ) -> Self {
        let expanded_channels = (hidden_channels as f64 * hidden_expansion) as i64;

        let conv1 = RTDetrV2ConvNormLayer::new(
            &(vs / "conv1"),
            expanded_channels,
            expanded_channels,
            3,       // kernel_size
            1,       // stride
            Some(1), // padding
            None,    // no activation (applied at end)
        );

        let conv2 = RTDetrV2ConvNormLayer::new(
            &(vs / "conv2"),
            expanded_channels,
            expanded_channels,
            1,       // kernel_size
            1,       // stride
            Some(0), // padding
            None,    // no activation (applied at end)
        );

        Self {
            conv1,
            conv2,
            activation,
        }
    }

    pub fn forward(&self, x: &Tensor) -> Tensor {
        // Profile RepVGG block internals (conv1, conv2, add, activation)
        let start_conv1 = if std::env::var("PROFILE_REPVGG").is_ok()
            && std::env::var("PROFILE_CSREPLAYER").is_ok()
            && std::env::var("PROFILE_FPN").is_ok()
            && std::env::var("PROFILE_ENCODER").is_ok()
        {
            Some(std::time::Instant::now())
        } else {
            None
        };

        let conv1_output = self.conv1.forward(x);

        if let Some(start) = start_conv1 {
            profile_csreplayer!(
                "        RepVGG conv1 (3×3): {:.3} ms",
                start.elapsed().as_secs_f64() * 1000.0
            );
        }

        let start_conv2 = if std::env::var("PROFILE_REPVGG").is_ok()
            && std::env::var("PROFILE_CSREPLAYER").is_ok()
            && std::env::var("PROFILE_FPN").is_ok()
            && std::env::var("PROFILE_ENCODER").is_ok()
        {
            Some(std::time::Instant::now())
        } else {
            None
        };

        let conv2_output = self.conv2.forward(x);

        if let Some(start) = start_conv2 {
            profile_csreplayer!(
                "        RepVGG conv2 (1×1): {:.3} ms",
                start.elapsed().as_secs_f64() * 1000.0
            );
        }

        let start_add = if std::env::var("PROFILE_REPVGG").is_ok()
            && std::env::var("PROFILE_CSREPLAYER").is_ok()
            && std::env::var("PROFILE_FPN").is_ok()
            && std::env::var("PROFILE_ENCODER").is_ok()
        {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Add outputs of both convolutions
        let y = conv1_output + conv2_output;

        if let Some(start) = start_add {
            profile_csreplayer!(
                "        RepVGG add: {:.3} ms",
                start.elapsed().as_secs_f64() * 1000.0
            );
        }

        let start_activation = if std::env::var("PROFILE_REPVGG").is_ok()
            && std::env::var("PROFILE_CSREPLAYER").is_ok()
            && std::env::var("PROFILE_FPN").is_ok()
            && std::env::var("PROFILE_ENCODER").is_ok()
        {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Apply activation if present
        let output = if let Some(activation) = &self.activation {
            activation.apply(&y)
        } else {
            y
        };

        if let Some(start) = start_activation {
            profile_csreplayer!(
                "        RepVGG activation: {:.3} ms",
                start.elapsed().as_secs_f64() * 1000.0
            );
        }

        output
    }
}

/// Cross Stage Partial (CSP) network layer with RepVGG blocks
/// Python: RTDetrV2CSPRepLayer (lines 998-1024)
#[derive(Debug)]
pub struct RTDetrV2CSPRepLayer {
    pub conv1: RTDetrV2ConvNormLayer,
    pub conv2: RTDetrV2ConvNormLayer,
    pub bottlenecks: Vec<RTDetrV2RepVggBlock>,
    pub conv3: Option<RTDetrV2ConvNormLayer>,
}

impl RTDetrV2CSPRepLayer {
    pub fn new(
        vs: &nn::Path,
        encoder_hidden_dim: i64,
        hidden_expansion: f64,
        activation: Option<Activation>,
    ) -> Self {
        let in_channels = encoder_hidden_dim * 2;
        let out_channels = encoder_hidden_dim;
        let num_blocks = 3;

        let hidden_channels = (out_channels as f64 * hidden_expansion) as i64;

        let conv1 = RTDetrV2ConvNormLayer::new(
            &(vs / "conv1"),
            in_channels,
            hidden_channels,
            1,    // kernel_size
            1,    // stride
            None, // default padding
            activation,
        );

        let conv2 = RTDetrV2ConvNormLayer::new(
            &(vs / "conv2"),
            in_channels,
            hidden_channels,
            1,    // kernel_size
            1,    // stride
            None, // default padding
            activation,
        );

        // Create bottleneck blocks
        let bottlenecks: Vec<RTDetrV2RepVggBlock> = (0..num_blocks)
            .map(|i| {
                RTDetrV2RepVggBlock::new(
                    &(vs / "bottlenecks" / i.to_string()),
                    out_channels,
                    hidden_expansion,
                    activation,
                )
            })
            .collect();

        // Optional conv3 if hidden_channels != out_channels
        let conv3 = if hidden_channels != out_channels {
            Some(RTDetrV2ConvNormLayer::new(
                &(vs / "conv3"),
                hidden_channels,
                out_channels,
                1,    // kernel_size
                1,    // stride
                None, // default padding
                activation,
            ))
        } else {
            None
        };

        Self {
            conv1,
            conv2,
            bottlenecks,
            conv3,
        }
    }

    pub fn forward(&self, hidden_state: &Tensor) -> Tensor {
        // First branch: conv1 → bottlenecks
        let t0 = Instant::now();
        let mut hidden_state_1 = self.conv1.forward(hidden_state);
        profile_csreplayer!(
            "      [CSPRepLayer] conv1: {:.3} ms",
            t0.elapsed().as_secs_f64() * 1000.0
        );

        // Bottleneck loop (3× RepVGG blocks)
        let t1 = Instant::now();
        for (i, bottleneck) in self.bottlenecks.iter().enumerate() {
            let t_bottleneck = Instant::now();
            hidden_state_1 = bottleneck.forward(&hidden_state_1);
            profile_csreplayer!(
                "      [CSPRepLayer] bottleneck[{}]: {:.3} ms",
                i,
                t_bottleneck.elapsed().as_secs_f64() * 1000.0
            );
        }
        let bottleneck_total = t1.elapsed().as_secs_f64() * 1000.0;
        profile_csreplayer!(
            "      [CSPRepLayer] bottleneck_loop_total: {:.3} ms",
            bottleneck_total
        );

        // Second branch: conv2
        let t2 = Instant::now();
        let hidden_state_2 = self.conv2.forward(hidden_state);
        profile_csreplayer!(
            "      [CSPRepLayer] conv2: {:.3} ms",
            t2.elapsed().as_secs_f64() * 1000.0
        );

        // Add branches and apply optional conv3
        let t3 = Instant::now();
        let output = hidden_state_1 + hidden_state_2;
        profile_csreplayer!(
            "      [CSPRepLayer] add: {:.3} ms",
            t3.elapsed().as_secs_f64() * 1000.0
        );

        if let Some(conv3) = &self.conv3 {
            let t4 = Instant::now();
            let result = conv3.forward(&output);
            profile_csreplayer!(
                "      [CSPRepLayer] conv3: {:.3} ms",
                t4.elapsed().as_secs_f64() * 1000.0
            );
            result
        } else {
            output
        }
    }
}

/// Configuration for encoder layer
#[derive(Debug)]
pub struct EncoderLayerConfig {
    pub encoder_hidden_dim: i64,
    pub num_attention_heads: i64,
    pub encoder_ffn_dim: i64,
    pub dropout: f64,
    pub activation_dropout: f64,
    pub encoder_activation_function: Activation,
    pub normalize_before: bool,
    pub layer_norm_eps: f64,
}

/// Single encoder layer with self-attention and feed-forward network
/// Python: RTDetrV2EncoderLayer (lines 896-976)
#[derive(Debug)]
pub struct RTDetrV2EncoderLayer {
    pub normalize_before: bool,
    pub self_attn: RTDetrV2MultiheadAttention,
    pub self_attn_layer_norm: nn::LayerNorm,
    pub dropout: f64,
    pub activation_fn: Activation,
    pub activation_dropout: f64,
    pub fc1: nn::Linear,
    pub fc2: nn::Linear,
    pub final_layer_norm: nn::LayerNorm,
}

impl RTDetrV2EncoderLayer {
    pub fn new(vs: &nn::Path, config: EncoderLayerConfig) -> Self {
        let encoder_hidden_dim = config.encoder_hidden_dim;
        let num_attention_heads = config.num_attention_heads;
        let encoder_ffn_dim = config.encoder_ffn_dim;
        let dropout = config.dropout;
        let activation_dropout = config.activation_dropout;
        let encoder_activation_function = config.encoder_activation_function;
        let normalize_before = config.normalize_before;
        let layer_norm_eps = config.layer_norm_eps;
        let self_attn = RTDetrV2MultiheadAttention::new(
            &(vs / "self_attn"),
            encoder_hidden_dim,
            num_attention_heads,
            dropout,
            true, // bias
        );

        let layer_norm_config = nn::LayerNormConfig {
            eps: layer_norm_eps,
            ..Default::default()
        };

        let self_attn_layer_norm = nn::layer_norm(
            vs / "self_attn_layer_norm",
            vec![encoder_hidden_dim],
            layer_norm_config,
        );

        let fc1 = nn::linear(
            vs / "fc1",
            encoder_hidden_dim,
            encoder_ffn_dim,
            Default::default(),
        );
        let fc2 = nn::linear(
            vs / "fc2",
            encoder_ffn_dim,
            encoder_hidden_dim,
            Default::default(),
        );

        let final_layer_norm = nn::layer_norm(
            vs / "final_layer_norm",
            vec![encoder_hidden_dim],
            layer_norm_config,
        );

        Self {
            normalize_before,
            self_attn,
            self_attn_layer_norm,
            dropout,
            activation_fn: encoder_activation_function,
            activation_dropout,
            fc1,
            fc2,
            final_layer_norm,
        }
    }

    pub fn forward(
        &self,
        hidden_states: &Tensor,
        attention_mask: Option<&Tensor>,
        position_embeddings: Option<&Tensor>,
        output_attentions: bool,
        train: bool,
    ) -> (Tensor, Option<Tensor>) {
        debug_log!("[ENCODER_LAYER] Starting encoder layer forward");
        debug_log!(
            "[ENCODER_LAYER]   hidden_states: {:?}",
            hidden_states.size()
        );
        debug_log!(
            "[ENCODER_LAYER]   normalize_before: {}",
            self.normalize_before
        );

        // DEBUG: Save encoder layer 0 input
        if let Err(e) = save_tensor_as_numpy(hidden_states, "rust_encoder_layer_0_input.npy") {
            debug_log!("  [DEBUG] Failed to save encoder_layer_0_input: {}", e);
        } else {
            debug_log!("  [DEBUG] Saved encoder_layer_0_input to rust_encoder_layer_0_input.npy");
        }

        let residual = hidden_states.shallow_clone();

        // Pre-norm if normalize_before
        debug_log!("[ENCODER_LAYER]   Applying pre-norm...");
        let hidden_states = if self.normalize_before {
            hidden_states.apply(&self.self_attn_layer_norm)
        } else {
            hidden_states.shallow_clone()
        };
        debug_log!("[ENCODER_LAYER]   Pre-norm complete");

        // Self-attention
        debug_log!("[ENCODER_LAYER]   Applying self-attention...");
        let (mut hidden_states, attn_weights) = self.self_attn.forward(
            &hidden_states,
            attention_mask,
            position_embeddings,
            output_attentions,
            train,
            None, // debug_name - not debugging encoder
        );
        debug_log!("[ENCODER_LAYER]   Self-attention complete");

        // DEBUG: Save attention output (before residual/norm)
        if let Err(e) = save_tensor_as_numpy(&hidden_states, "rust_encoder_layer_0_attn_output.npy")
        {
            debug_log!(
                "  [DEBUG] Failed to save encoder_layer_0_attn_output: {}",
                e
            );
        } else {
            debug_log!("  [DEBUG] Saved encoder_layer_0_attn_output to rust_encoder_layer_0_attn_output.npy");
        }

        // Dropout + residual
        debug_log!("[ENCODER_LAYER]   Applying dropout...");
        hidden_states = if train {
            hidden_states.dropout(self.dropout, train)
        } else {
            hidden_states
        };
        debug_log!("[ENCODER_LAYER]   Adding residual...");
        hidden_states = residual + hidden_states;
        debug_log!("[ENCODER_LAYER]   Residual added");

        // Post-norm if not normalize_before
        debug_log!("[ENCODER_LAYER]   Applying post-norm (if needed)...");
        let hidden_states = if !self.normalize_before {
            hidden_states.apply(&self.self_attn_layer_norm)
        } else {
            hidden_states
        };
        debug_log!("[ENCODER_LAYER]   Post-norm complete");

        // Pre-norm for FFN if normalize_before
        debug_log!("[ENCODER_LAYER]   Applying FFN pre-norm...");
        let hidden_states = if self.normalize_before {
            hidden_states.apply(&self.final_layer_norm)
        } else {
            hidden_states
        };
        debug_log!("[ENCODER_LAYER]   FFN pre-norm complete");
        let residual = hidden_states.shallow_clone();

        // Feed-forward network
        debug_log!("[ENCODER_LAYER]   Applying fc1...");
        let mut hidden_states = hidden_states.apply(&self.fc1);
        debug_log!(
            "[ENCODER_LAYER]   fc1 complete, shape: {:?}",
            hidden_states.size()
        );
        debug_log!(
            "[ENCODER_LAYER]   fc1 min/max = {:.6} / {:.6}",
            hidden_states.min().double_value(&[]),
            hidden_states.max().double_value(&[])
        );
        debug_log!("[ENCODER_LAYER]   fc1 device: {:?}", hidden_states.device());
        debug_log!("[ENCODER_LAYER]   fc1 dtype: {:?}", hidden_states.kind());
        debug_log!(
            "[ENCODER_LAYER]   Applying activation (activation_fn: {:?})...",
            self.activation_fn
        );
        hidden_states = self.activation_fn.apply(&hidden_states);
        debug_log!("[ENCODER_LAYER]   Activation complete");
        debug_log!("[ENCODER_LAYER]   Applying activation dropout...");
        hidden_states = if train {
            hidden_states.dropout(self.activation_dropout, train)
        } else {
            hidden_states
        };
        debug_log!("[ENCODER_LAYER]   Activation dropout complete");

        debug_log!("[ENCODER_LAYER]   Applying fc2...");
        hidden_states = hidden_states.apply(&self.fc2);
        debug_log!("[ENCODER_LAYER]   fc2 complete");
        hidden_states = if train {
            hidden_states.dropout(self.dropout, train)
        } else {
            hidden_states
        };

        hidden_states = residual + hidden_states;

        // Post-norm if not normalize_before
        let mut hidden_states = if !self.normalize_before {
            hidden_states.apply(&self.final_layer_norm)
        } else {
            hidden_states
        };

        // Clamp values during training to prevent overflow
        if train {
            let max_value = match hidden_states.kind() {
                Kind::Float => f32::MAX as f64 - 1000.0,
                Kind::Double => f64::MAX - 1000.0,
                _ => 1e30,
            };
            hidden_states = hidden_states.clamp(-max_value, max_value);
        }

        // DEBUG: Save encoder layer 0 final output
        if let Err(e) = save_tensor_as_numpy(&hidden_states, "rust_encoder_layer_0_output.npy") {
            debug_log!("  [DEBUG] Failed to save encoder_layer_0_output: {}", e);
        } else {
            debug_log!("  [DEBUG] Saved encoder_layer_0_output to rust_encoder_layer_0_output.npy");
        }

        (hidden_states, attn_weights)
    }
}

/// Stack of encoder layers
/// Python: RTDetrV2Encoder (lines 1027-1042)
#[derive(Debug)]
pub struct RTDetrV2Encoder {
    pub layers: Vec<RTDetrV2EncoderLayer>,
}

impl RTDetrV2Encoder {
    pub fn new(vs: &nn::Path, num_encoder_layers: i64, config: EncoderLayerConfig) -> Self {
        let layers: Vec<RTDetrV2EncoderLayer> = (0..num_encoder_layers)
            .map(|i| RTDetrV2EncoderLayer::new(&(vs / "layers" / i.to_string()), config.clone()))
            .collect();

        Self { layers }
    }

    pub fn forward(
        &self,
        src: &Tensor,
        src_mask: Option<&Tensor>,
        pos_embed: Option<&Tensor>,
        output_attentions: bool,
        train: bool,
    ) -> (Tensor, Vec<Option<Tensor>>) {
        let mut hidden_states = src.shallow_clone();
        let mut all_attentions = Vec::new();

        for layer in &self.layers {
            let (new_hidden_states, attn_weights) = layer.forward(
                &hidden_states,
                src_mask,
                pos_embed,
                output_attentions,
                train,
            );
            hidden_states = new_hidden_states;
            all_attentions.push(attn_weights);
        }

        (hidden_states, all_attentions)
    }
}

/// Encoder stage outputs for systematic validation
#[derive(Debug)]
pub struct EncoderStageOutputs {
    pub fpn_block_outputs: Vec<Tensor>,
    pub pan_block_outputs: Vec<Tensor>,
    pub final_outputs: Vec<Tensor>,
}

/// Hybrid encoder with FPN and PAN
/// Python: RTDetrV2HybridEncoder (lines 1045-1236)
#[derive(Debug)]
pub struct RTDetrV2HybridEncoder {
    pub encoder: Vec<RTDetrV2Encoder>,
    pub lateral_convs: Vec<RTDetrV2ConvNormLayer>,
    pub fpn_blocks: Vec<RTDetrV2CSPRepLayer>,
    pub downsample_convs: Vec<RTDetrV2ConvNormLayer>,
    pub pan_blocks: Vec<RTDetrV2CSPRepLayer>,
    pub encoder_hidden_dim: i64,
    pub encode_proj_layers: Vec<i64>,
    pub num_fpn_stages: i64,
    pub num_pan_stages: i64,
    pub positional_encoding_temperature: f64,
    pub eval_size: Option<(i64, i64)>,
}

impl RTDetrV2HybridEncoder {
    #[allow(
        clippy::too_many_arguments,
        reason = "RT-DETR encoder requires many configuration params"
    )]
    pub fn new(
        vs: &nn::Path,
        encoder_hidden_dim: i64,
        encoder_layers: i64,
        encode_proj_layers: Vec<i64>,
        num_fpn_stages: i64,
        num_pan_stages: i64,
        activation: Activation,
        encoder_config: EncoderLayerConfig,
        positional_encoding_temperature: f64,
        eval_size: Option<(i64, i64)>,
    ) -> Self {
        // Encoder transformers (one per projection layer)
        let encoder: Vec<RTDetrV2Encoder> = encode_proj_layers
            .iter()
            .enumerate()
            .map(|(i, _)| {
                RTDetrV2Encoder::new(
                    &(vs / "encoder" / i.to_string()),
                    encoder_layers,
                    encoder_config.clone(),
                )
            })
            .collect();

        // Top-down FPN (Feature Pyramid Network)
        let lateral_convs: Vec<RTDetrV2ConvNormLayer> = (0..num_fpn_stages)
            .map(|i| {
                RTDetrV2ConvNormLayer::new(
                    &(vs / "lateral_convs" / i.to_string()),
                    encoder_hidden_dim,
                    encoder_hidden_dim,
                    1,    // kernel_size
                    1,    // stride
                    None, // padding (calculated)
                    Some(activation),
                )
            })
            .collect();

        let fpn_blocks: Vec<RTDetrV2CSPRepLayer> = (0..num_fpn_stages)
            .map(|i| {
                RTDetrV2CSPRepLayer::new(
                    &(vs / "fpn_blocks" / i.to_string()),
                    encoder_hidden_dim,
                    1.0, // hidden_expansion
                    Some(activation),
                )
            })
            .collect();

        // Bottom-up PAN (Path Aggregation Network)
        let downsample_convs: Vec<RTDetrV2ConvNormLayer> = (0..num_pan_stages)
            .map(|i| {
                RTDetrV2ConvNormLayer::new(
                    &(vs / "downsample_convs" / i.to_string()),
                    encoder_hidden_dim,
                    encoder_hidden_dim,
                    3,    // kernel_size
                    2,    // stride (downsample by 2x)
                    None, // padding (calculated)
                    Some(activation),
                )
            })
            .collect();

        let pan_blocks: Vec<RTDetrV2CSPRepLayer> = (0..num_pan_stages)
            .map(|i| {
                RTDetrV2CSPRepLayer::new(
                    &(vs / "pan_blocks" / i.to_string()),
                    encoder_hidden_dim,
                    1.0, // hidden_expansion
                    Some(activation),
                )
            })
            .collect();

        Self {
            encoder,
            lateral_convs,
            fpn_blocks,
            downsample_convs,
            pan_blocks,
            encoder_hidden_dim,
            encode_proj_layers,
            num_fpn_stages,
            num_pan_stages,
            positional_encoding_temperature,
            eval_size,
        }
    }

    /// Build 2D sinusoidal position embeddings
    /// Python: build_2d_sincos_position_embedding (lines 1104-1120)
    pub fn build_2d_sincos_position_embedding(
        width: i64,
        height: i64,
        embed_dim: i64,
        temperature: f64,
        device: tch::Device,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        if embed_dim % 4 != 0 {
            return Err(
                "Embed dimension must be divisible by 4 for 2D sin-cos position embedding".into(),
            );
        }

        let pos_dim = embed_dim / 4;

        // Create coordinate grids
        let grid_w = Tensor::arange(width, (Kind::Float, device));
        let grid_h = Tensor::arange(height, (Kind::Float, device));

        // meshgrid with indexing="ij" (Cartesian indexing)
        let grid_w = grid_w.unsqueeze(1).expand([width, height], false);
        let grid_h = grid_h.unsqueeze(0).expand([width, height], false);

        // Compute omega = 1.0 / (temperature ** (arange(pos_dim) / pos_dim))
        // Formula: omega = temperature^(-i/pos_dim) = exp(-i/pos_dim * ln(temperature))
        let omega = Tensor::arange(pos_dim, (Kind::Float, device)) / pos_dim as f64;
        let omega = (omega * (-temperature.ln())).exp(); // exp(-omega * ln(temp)) = temp^(-omega)

        // Flatten grids and compute outer products
        let out_w = grid_w
            .flatten(0, 1)
            .unsqueeze(-1)
            .matmul(&omega.unsqueeze(0));
        let out_h = grid_h
            .flatten(0, 1)
            .unsqueeze(-1)
            .matmul(&omega.unsqueeze(0));

        // Concatenate sin and cos embeddings
        let pos_embed = Tensor::cat(&[out_w.sin(), out_w.cos(), out_h.sin(), out_h.cos()], 1);

        // Add batch dimension: [1, width*height, embed_dim]
        Ok(pos_embed.unsqueeze(0))
    }

    pub fn forward(
        &self,
        hidden_states: &[Tensor],
        output_attentions: bool,
        train: bool,
    ) -> Result<Vec<Tensor>, Box<dyn std::error::Error>> {
        debug_log!("[ENCODER] Starting encoder forward");
        debug_log!("[ENCODER] Input features: {}", hidden_states.len());
        for (i, t) in hidden_states.iter().enumerate() {
            debug_log!("[ENCODER]   Feature {}: {:?}", i, t.size());
        }

        let mut hidden_states: Vec<Tensor> =
            hidden_states.iter().map(|t| t.shallow_clone()).collect();

        // Encoder pass (if encoder_layers > 0)
        debug_log!(
            "[ENCODER] encode_proj_layers: {:?}",
            self.encode_proj_layers
        );
        debug_log!("[ENCODER] encoder layers count: {}", self.encoder.len());
        let self_attn_start = Instant::now();
        if !self.encode_proj_layers.is_empty() {
            for (i, &enc_ind) in self.encode_proj_layers.iter().enumerate() {
                debug_log!(
                    "[ENCODER] Processing encoder layer {} (feature index {})",
                    i,
                    enc_ind
                );
                let enc_ind_usize = enc_ind as usize;
                if enc_ind_usize >= hidden_states.len() {
                    return Err(format!(
                        "encode_proj_layers index {} out of bounds (hidden_states len: {})",
                        enc_ind,
                        hidden_states.len()
                    )
                    .into());
                }

                let feature_map = &hidden_states[enc_ind_usize];
                let shape = feature_map.size();
                let height = shape[2];
                let width = shape[3];
                debug_log!("[ENCODER]   Feature shape: {:?}", shape);

                // Debug encoder input before flattening (only for layer 0)
                if i == 0 {
                    debug_log!("[ENCODER_INPUT] Feature map before flattening:");
                    debug_log!("[ENCODER_INPUT]   Shape: {:?}", feature_map.size());
                    debug_log!("[ENCODER_INPUT]   Sample [0, :5, 0, 0]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
                        feature_map.double_value(&[0, 0, 0, 0]),
                        feature_map.double_value(&[0, 1, 0, 0]),
                        feature_map.double_value(&[0, 2, 0, 0]),
                        feature_map.double_value(&[0, 3, 0, 0]),
                        feature_map.double_value(&[0, 4, 0, 0]),
                    );
                    // Python expected: [-0.05451153, -0.19066043, 0.2057578, 0.03153802, 0.12763587]
                }

                // Flatten [batch, channel, height, width] to [batch, height*width, channel]
                debug_log!("[ENCODER]   Flattening...");
                let src_flatten = feature_map.flatten(2, 3).permute([0, 2, 1]);
                debug_log!("[ENCODER]   Flattened shape: {:?}", src_flatten.size());

                // Build position embeddings (training or eval without fixed size)
                debug_log!("[ENCODER]   Building position embeddings...");
                let pos_embed = if train || self.eval_size.is_none() {
                    Some(Self::build_2d_sincos_position_embedding(
                        width,
                        height,
                        self.encoder_hidden_dim,
                        self.positional_encoding_temperature,
                        feature_map.device(),
                    )?)
                } else {
                    None
                };
                debug_log!("[ENCODER]   Position embeddings built");

                // Apply encoder layer
                debug_log!("[ENCODER]   Applying encoder layer...");
                let (encoded, _attentions) = self.encoder[i].forward(
                    &src_flatten,
                    None,
                    pos_embed.as_ref(),
                    output_attentions,
                    train,
                );
                debug_log!(
                    "[ENCODER]   Encoder layer complete, shape: {:?}",
                    encoded.size()
                );

                // Debug output for encoder layer 0 comparison
                if i == 0 {
                    debug_log!("[ENCODER_LAYER_0] Encoder output (flattened):");
                    debug_log!("[ENCODER_LAYER_0]   Shape: {:?}", encoded.size());
                    debug_log!("[ENCODER_LAYER_0]   Sample [0, 0, :5]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
                        encoded.double_value(&[0, 0, 0]),
                        encoded.double_value(&[0, 0, 1]),
                        encoded.double_value(&[0, 0, 2]),
                        encoded.double_value(&[0, 0, 3]),
                        encoded.double_value(&[0, 0, 4]),
                    );
                    // Python expected: [1.0327351, 1.6274145, -0.43288943, -0.32748982, -0.30143353]

                    // Save self-attention encoder output for comparison
                    let save_path = "debug_rust_self_attention_encoder.npy";
                    if let Err(e) = save_tensor_as_numpy(&encoded, save_path) {
                        debug_log!(
                            "[WARNING] Failed to save self-attention encoder output: {}",
                            e
                        );
                    } else {
                        debug_log!(
                            "[DEBUG] Saved self-attention encoder output to {}",
                            save_path
                        );
                    }
                }

                // Reshape back to [batch, channel, height, width]
                let batch_size = shape[0];
                debug_log!("[ENCODER]   Reshaping back...");
                let encoded = encoded
                    .permute([0, 2, 1])
                    .reshape([batch_size, self.encoder_hidden_dim, height, width])
                    .contiguous();
                debug_log!("[ENCODER]   Reshaped to: {:?}", encoded.size());

                // Debug reshaped output for encoder layer 0
                if i == 0 {
                    debug_log!("[ENCODER_LAYER_0] Reshaped encoder output (4D):");
                    debug_log!("[ENCODER_LAYER_0]   Shape: {:?}", encoded.size());
                    debug_log!("[ENCODER_LAYER_0]   Sample [0, :5, 0, 0]: [{:.6}, {:.6}, {:.6}, {:.6}, {:.6}]",
                        encoded.double_value(&[0, 0, 0, 0]),
                        encoded.double_value(&[0, 1, 0, 0]),
                        encoded.double_value(&[0, 2, 0, 0]),
                        encoded.double_value(&[0, 3, 0, 0]),
                        encoded.double_value(&[0, 4, 0, 0]),
                    );

                    // Save reshaped output for comparison
                    let save_path = "debug_rust_encoder_output_2_after_reshape.npy";
                    if let Err(e) = save_tensor_4d_as_numpy(&encoded, save_path) {
                        debug_log!("[WARNING] Failed to save reshaped encoder output: {}", e);
                    } else {
                        debug_log!("[DEBUG] Saved reshaped encoder output to {}", save_path);
                    }
                }

                hidden_states[enc_ind_usize] = encoded;
                debug_log!("[ENCODER]   Encoder layer {} complete", i);
            }
        }
        let self_attn_time = self_attn_start.elapsed();
        profile_log!(
            "[PROFILE]   Self-attention encoder: {:.2} ms",
            self_attn_time.as_secs_f64() * 1000.0
        );
        debug_log!("[ENCODER] Encoder pass complete");

        // Debug: Check hidden_states before FPN
        debug_log!("[FPN_INIT] Hidden states before FPN:");
        for (i, t) in hidden_states.iter().enumerate() {
            debug_log!("[FPN_INIT]   hidden_states[{}]: {:?}", i, t.size());
        }

        // Top-down FPN (Feature Pyramid Network)
        let fpn_start = Instant::now();
        let last_hidden_state = hidden_states[hidden_states.len() - 1].shallow_clone();
        debug_log!(
            "[FPN_INIT] Initializing fpn_feature_maps with hidden_states[{}]: {:?}",
            hidden_states.len() - 1,
            last_hidden_state.size()
        );
        let mut fpn_feature_maps = vec![last_hidden_state];

        for (idx, (lateral_conv, fpn_block)) in self
            .lateral_convs
            .iter()
            .zip(self.fpn_blocks.iter())
            .enumerate()
        {
            let fpn_block_start = Instant::now();
            let backbone_idx = (self.num_fpn_stages - idx as i64 - 1) as usize;
            let backbone_feature_map = &hidden_states[backbone_idx];
            let mut top_fpn_feature_map = fpn_feature_maps.last().unwrap().shallow_clone();

            // Debug: Save input to lateral conv
            if idx == 0 {
                debug_log!("[LATERAL_CONV_0] Input to lateral conv:");
                debug_log!("[LATERAL_CONV_0]   Shape: {:?}", top_fpn_feature_map.size());
                debug_log!(
                    "[LATERAL_CONV_0]   Mean: {:.6}",
                    top_fpn_feature_map.mean(tch::Kind::Float).double_value(&[])
                );
                debug_log!(
                    "[LATERAL_CONV_0]   Max: {:.6}",
                    top_fpn_feature_map.abs().max().double_value(&[])
                );

                if let Err(e) = save_tensor_4d_as_numpy(
                    &top_fpn_feature_map,
                    "debug_rust_lateral_conv_0_input.npy",
                ) {
                    debug_log!("[WARNING] Failed to save lateral_conv_0 input: {}", e);
                }
            }

            // Apply lateral convolution
            let lateral_start = Instant::now();
            top_fpn_feature_map = if idx == 0 {
                // Use debug version for first lateral conv
                lateral_conv.forward_debug(&top_fpn_feature_map, "lateral_conv_0")
            } else {
                lateral_conv.forward(&top_fpn_feature_map)
            };
            *fpn_feature_maps.last_mut().unwrap() = top_fpn_feature_map.shallow_clone();
            let lateral_time = lateral_start.elapsed();
            profile_fpn!(
                "[PROFILE]       Lateral conv: {:.2} ms",
                lateral_time.as_secs_f64() * 1000.0
            );

            // Upsample and fuse with backbone feature
            let upsample_start = Instant::now();
            let upsampled = top_fpn_feature_map.upsample_nearest2d(
                [
                    backbone_feature_map.size()[2],
                    backbone_feature_map.size()[3],
                ],
                None,
                None,
            );
            let upsample_time = upsample_start.elapsed();
            profile_fpn!(
                "[PROFILE]       Upsample: {:.2} ms",
                upsample_time.as_secs_f64() * 1000.0
            );

            // Debug FPN block 0 intermediate outputs
            if idx == 0 {
                debug_log!("[FPN_BLOCK_0] Extracting intermediate outputs...");

                // Save lateral conv output
                if let Err(e) = save_tensor_4d_as_numpy(
                    &top_fpn_feature_map,
                    "debug_rust_fpn_block0_lateral_output.npy",
                ) {
                    debug_log!("[WARNING] Failed to save lateral output: {}", e);
                }

                // Save upsampled
                if let Err(e) =
                    save_tensor_4d_as_numpy(&upsampled, "debug_rust_fpn_block0_upsampled.npy")
                {
                    debug_log!("[WARNING] Failed to save upsampled: {}", e);
                }

                // Save backbone feature (40x40)
                if let Err(e) = save_tensor_4d_as_numpy(
                    backbone_feature_map,
                    "debug_rust_fpn_block0_backbone_40x40.npy",
                ) {
                    debug_log!("[WARNING] Failed to save backbone feature: {}", e);
                }
            }

            let concat_start = Instant::now();
            let fused = Tensor::cat(&[upsampled, backbone_feature_map.shallow_clone()], 1);
            let concat_time = concat_start.elapsed();
            profile_fpn!(
                "[PROFILE]       Concat: {:.2} ms",
                concat_time.as_secs_f64() * 1000.0
            );

            // Debug FPN block 0 fused input
            if idx == 0 {
                if let Err(e) = save_tensor_4d_as_numpy(&fused, "debug_rust_fpn_block0_fused.npy") {
                    debug_log!("[WARNING] Failed to save fused: {}", e);
                }
            }

            let bottleneck_start = Instant::now();
            let new_fpn_feature = if idx == 0 {
                // Manual forward pass with debug output for FPN block 0
                debug_log!("[FPN_BLOCK_0] Running manual forward pass...");

                // Branch 1: conv1 → bottlenecks
                let mut branch1 = fpn_block.conv1.forward(&fused);
                if let Err(e) =
                    save_tensor_4d_as_numpy(&branch1, "debug_rust_fpn_block0_conv1_output.npy")
                {
                    debug_log!("[WARNING] Failed to save conv1 output: {}", e);
                }

                // Bottleneck 0
                branch1 = fpn_block.bottlenecks[0].forward(&branch1);
                if let Err(e) = save_tensor_4d_as_numpy(
                    &branch1,
                    "debug_rust_fpn_block0_bottleneck0_output.npy",
                ) {
                    debug_log!("[WARNING] Failed to save bottleneck0 output: {}", e);
                }

                // Bottleneck 1
                branch1 = fpn_block.bottlenecks[1].forward(&branch1);
                if let Err(e) = save_tensor_4d_as_numpy(
                    &branch1,
                    "debug_rust_fpn_block0_bottleneck1_output.npy",
                ) {
                    debug_log!("[WARNING] Failed to save bottleneck1 output: {}", e);
                }

                // Bottleneck 2
                branch1 = fpn_block.bottlenecks[2].forward(&branch1);
                if let Err(e) = save_tensor_4d_as_numpy(
                    &branch1,
                    "debug_rust_fpn_block0_bottleneck2_output.npy",
                ) {
                    debug_log!("[WARNING] Failed to save bottleneck2 output: {}", e);
                }

                // Branch 2: conv2
                let branch2 = fpn_block.conv2.forward(&fused);
                if let Err(e) =
                    save_tensor_4d_as_numpy(&branch2, "debug_rust_fpn_block0_conv2_output.npy")
                {
                    debug_log!("[WARNING] Failed to save conv2 output: {}", e);
                }

                // Add branches
                let added = &branch1 + &branch2;
                if let Err(e) = save_tensor_4d_as_numpy(&added, "debug_rust_fpn_block0_added.npy") {
                    debug_log!("[WARNING] Failed to save added output: {}", e);
                }

                // Optional conv3
                if let Some(conv3) = &fpn_block.conv3 {
                    let output = conv3.forward(&added);
                    if let Err(e) =
                        save_tensor_4d_as_numpy(&output, "debug_rust_fpn_block0_conv3_output.npy")
                    {
                        debug_log!("[WARNING] Failed to save conv3 output: {}", e);
                    }
                    output
                } else {
                    debug_log!("[FPN_BLOCK_0] Conv3 is None, using added output");
                    added
                }
            } else {
                fpn_block.forward(&fused)
            };
            let bottleneck_time = bottleneck_start.elapsed();
            profile_fpn!(
                "[PROFILE]       Bottleneck (CSPRepLayer): {:.2} ms",
                bottleneck_time.as_secs_f64() * 1000.0
            );

            // Debug: Save FPN block output for comparison (before pushing)
            debug_log!("[DEBUG] FPN block {}: {:?}", idx, new_fpn_feature.size());

            // Save FPN block outputs for validation (if env var set)
            if let Ok(debug_page) = std::env::var("DEBUG_SAVE_FPN_PAN") {
                let save_dir =
                    format!("baseline_data/{}/layout/rust_encoder_internals", debug_page);
                std::fs::create_dir_all(&save_dir).ok();
                let path = format!("{}/fpn_block_{}.npy", save_dir, idx);
                if let Err(e) = save_tensor_4d_as_numpy(&new_fpn_feature, &path) {
                    trace!("[DEBUG] Failed to save FPN block {}: {:?}", idx, e);
                } else {
                    trace!("[DEBUG] Saved FPN block {} to {}", idx, path);
                }
            }

            fpn_feature_maps.push(new_fpn_feature);
            let fpn_block_time = fpn_block_start.elapsed();
            profile_fpn!(
                "[PROFILE]     FPN block {}: {:.2} ms",
                idx,
                fpn_block_time.as_secs_f64() * 1000.0
            );
        }

        // Reverse to get top-to-bottom order
        fpn_feature_maps.reverse();
        let fpn_time = fpn_start.elapsed();
        profile_log!(
            "[PROFILE]   FPN (Feature Pyramid Network): {:.2} ms",
            fpn_time.as_secs_f64() * 1000.0
        );

        // Bottom-up PAN (Path Aggregation Network)
        let pan_start = Instant::now();
        let mut pan_feature_maps = vec![fpn_feature_maps[0].shallow_clone()];

        for (idx, (downsample_conv, pan_block)) in self
            .downsample_convs
            .iter()
            .zip(self.pan_blocks.iter())
            .enumerate()
        {
            let top_pan_feature = pan_feature_maps.last().unwrap();
            let fpn_feature = &fpn_feature_maps[idx + 1];

            // Downsample top PAN feature
            let downsampled = downsample_conv.forward(top_pan_feature);

            // Fuse with FPN feature
            let fused = Tensor::cat(&[downsampled, fpn_feature.shallow_clone()], 1);
            let new_pan_feature = pan_block.forward(&fused);

            // Debug: Save PAN block output for comparison (before pushing)
            debug_log!("[DEBUG] PAN block {}: {:?}", idx, new_pan_feature.size());

            // Save PAN block outputs for validation (if env var set)
            if let Ok(debug_page) = std::env::var("DEBUG_SAVE_FPN_PAN") {
                let save_dir =
                    format!("baseline_data/{}/layout/rust_encoder_internals", debug_page);
                std::fs::create_dir_all(&save_dir).ok();
                let path = format!("{}/pan_block_{}.npy", save_dir, idx);
                if let Err(e) = save_tensor_4d_as_numpy(&new_pan_feature, &path) {
                    trace!("[DEBUG] Failed to save PAN block {}: {:?}", idx, e);
                } else {
                    trace!("[DEBUG] Saved PAN block {} to {}", idx, path);
                }
            }

            pan_feature_maps.push(new_pan_feature);
        }
        let pan_time = pan_start.elapsed();
        profile_log!(
            "[PROFILE]   PAN (Path Aggregation Network): {:.2} ms",
            pan_time.as_secs_f64() * 1000.0
        );

        // Debug: Save final encoder outputs for comparison
        debug_log!(
            "[DEBUG] Saving final encoder outputs ({} feature maps)...",
            pan_feature_maps.len()
        );
        for (i, feat) in pan_feature_maps.iter().enumerate() {
            debug_log!("[DEBUG]   Encoder output {}: {:?}", i, feat.size());

            // Save each output for comparison with Python
            let save_path = format!("debug_rust_encoder_output_{}.npy", i);
            // Convert [B, C, H, W] to Python format
            // Python saves as [B, C, H, W] so we keep the same format
            if feat.size().len() == 4 {
                // Create a 4D array for saving
                use ndarray::Array4;
                let size = feat.size();
                let (b, c, h, w) = (size[0], size[1], size[2], size[3]);

                let feat_cpu = feat.to_kind(tch::Kind::Float).to(tch::Device::Cpu);
                let feat_flat = feat_cpu.flatten(0, -1);
                let data: Vec<f32> = Vec::try_from(&feat_flat)
                    .map_err(|e| format!("Failed to convert tensor: {:?}", e))?;

                let array =
                    Array4::from_shape_vec((b as usize, c as usize, h as usize, w as usize), data)?;

                ndarray_npy::write_npy(&save_path, &array)?;
                debug_log!("[DEBUG]   Saved to {}", save_path);
            }
        }

        Ok(pan_feature_maps)
    }

    /// Forward pass that also returns intermediate FPN/PAN stage outputs for validation
    /// This is used for systematic validation testing (Phase 2)
    pub fn forward_with_stages(
        &self,
        hidden_states: &[Tensor],
        output_attentions: bool,
        train: bool,
    ) -> Result<EncoderStageOutputs, Box<dyn std::error::Error>> {
        // Run same logic as forward(), but capture FPN/PAN block outputs

        let mut hidden_states: Vec<Tensor> =
            hidden_states.iter().map(|t| t.shallow_clone()).collect();

        // Encoder pass (same as forward())
        if !self.encode_proj_layers.is_empty() {
            for (i, &enc_ind) in self.encode_proj_layers.iter().enumerate() {
                let enc_ind_usize = enc_ind as usize;
                if enc_ind_usize >= hidden_states.len() {
                    return Err(format!(
                        "encode_proj_layers index {} out of bounds (hidden_states len: {})",
                        enc_ind,
                        hidden_states.len()
                    )
                    .into());
                }

                let feature_map = &hidden_states[enc_ind_usize];
                let shape = feature_map.size();
                let height = shape[2];
                let width = shape[3];

                let pos_embed = Self::build_2d_sincos_position_embedding(
                    width,
                    height,
                    self.encoder_hidden_dim,
                    self.positional_encoding_temperature,
                    feature_map.device(),
                )?;

                let src_flatten = feature_map.flatten(2, -1).permute([0, 2, 1]);

                let (encoded, _attentions) = self.encoder[i].forward(
                    &src_flatten,
                    None,
                    Some(&pos_embed),
                    output_attentions,
                    train,
                );

                let batch_size = shape[0];
                let encoded = encoded
                    .permute([0, 2, 1])
                    .reshape([batch_size, self.encoder_hidden_dim, height, width])
                    .contiguous();

                hidden_states[enc_ind_usize] = encoded;
            }
        }

        // Top-down FPN - CAPTURE OUTPUTS
        // N=511: Reduced shallow_clone() overhead (5-10% expected speedup)
        // - Line 1367: Keep clone (need ownership for vec initialization)
        // - Line 1377: REMOVED - use reference for lateral_conv.forward()
        // - Line 1380: REMOVED - move instead of clone-then-replace
        // - Line 1388: REMOVED - Tensor::cat accepts mixed references
        // - Line 1391: Keep clone (need to capture for output AND push to vec)
        //
        // N=511: Vec pre-allocation (2-5% expected speedup)
        // - Pre-allocate fpn_feature_maps and fpn_block_outputs to avoid reallocation
        let last_hidden_state = hidden_states[hidden_states.len() - 1].shallow_clone();
        let mut fpn_feature_maps = Vec::with_capacity((self.num_fpn_stages + 1) as usize);
        fpn_feature_maps.push(last_hidden_state);
        let mut fpn_block_outputs = Vec::with_capacity(self.num_fpn_stages as usize); // CAPTURE

        for (idx, (lateral_conv, fpn_block)) in self
            .lateral_convs
            .iter()
            .zip(self.fpn_blocks.iter())
            .enumerate()
        {
            let backbone_idx = (self.num_fpn_stages - idx as i64 - 1) as usize;
            let backbone_feature_map = &hidden_states[backbone_idx];

            // OPTIMIZATION (N=511): Use reference instead of shallow_clone (line 1377)
            let top_fpn_feature_map = fpn_feature_maps.last().unwrap();
            let top_fpn_feature_map = lateral_conv.forward(top_fpn_feature_map);

            // OPTIMIZATION (N=511): Move instead of clone-then-replace (line 1380)
            *fpn_feature_maps.last_mut().unwrap() = top_fpn_feature_map;

            let top_fpn_feature_map = fpn_feature_maps.last().unwrap();
            let upsampled = top_fpn_feature_map.upsample_nearest2d(
                [
                    backbone_feature_map.size()[2],
                    backbone_feature_map.size()[3],
                ],
                None,
                None,
            );

            // OPTIMIZATION (N=511): Use reference for cat instead of shallow_clone (line 1388)
            let fused = Tensor::cat(&[&upsampled, backbone_feature_map], 1);
            let new_fpn_feature = fpn_block.forward(&fused);

            fpn_block_outputs.push(new_fpn_feature.shallow_clone()); // CAPTURE - clone needed
            fpn_feature_maps.push(new_fpn_feature);
        }

        fpn_feature_maps.reverse();

        // Bottom-up PAN - CAPTURE OUTPUTS
        // N=511: Reduced shallow_clone() overhead
        // - Line 1398: Keep clone (need ownership for vec initialization)
        // - Line 1409: REMOVED - Tensor::cat accepts mixed references
        // - Line 1412: Keep clone (need to capture for output AND push to vec)
        //
        // N=511: Vec pre-allocation (2-5% expected speedup)
        // - Pre-allocate pan_feature_maps and pan_block_outputs to avoid reallocation
        let mut pan_feature_maps = Vec::with_capacity((self.num_fpn_stages + 1) as usize);
        pan_feature_maps.push(fpn_feature_maps[0].shallow_clone());
        let mut pan_block_outputs = Vec::with_capacity(self.num_fpn_stages as usize); // CAPTURE

        for (idx, (downsample_conv, pan_block)) in self
            .downsample_convs
            .iter()
            .zip(self.pan_blocks.iter())
            .enumerate()
        {
            let top_pan_feature = pan_feature_maps.last().unwrap();
            let fpn_feature = &fpn_feature_maps[idx + 1];

            let downsampled = downsample_conv.forward(top_pan_feature);
            // OPTIMIZATION (N=511): Use reference for cat instead of shallow_clone (line 1409)
            let fused = Tensor::cat(&[&downsampled, fpn_feature], 1);
            let new_pan_feature = pan_block.forward(&fused);

            pan_block_outputs.push(new_pan_feature.shallow_clone()); // CAPTURE - clone needed
            pan_feature_maps.push(new_pan_feature);
        }

        Ok(EncoderStageOutputs {
            fpn_block_outputs,
            pan_block_outputs,
            final_outputs: pan_feature_maps,
        })
    }
}

// Clone implementation for EncoderLayerConfig (needed for encoder construction)
impl Clone for EncoderLayerConfig {
    fn clone(&self) -> Self {
        Self {
            encoder_hidden_dim: self.encoder_hidden_dim,
            num_attention_heads: self.num_attention_heads,
            encoder_ffn_dim: self.encoder_ffn_dim,
            dropout: self.dropout,
            activation_dropout: self.activation_dropout,
            encoder_activation_function: self.encoder_activation_function, // Copy trait
            normalize_before: self.normalize_before,
            layer_norm_eps: self.layer_norm_eps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tch::{Device, Kind};

    #[test]
    fn test_conv_norm_layer() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let in_channels = 64;
        let out_channels = 128;
        let kernel_size = 3;
        let stride = 1;

        let layer = RTDetrV2ConvNormLayer::new(
            &root,
            in_channels,
            out_channels,
            kernel_size,
            stride,
            None,
            Some(Activation::ReLU),
        );

        let batch_size = 2;
        let height = 28;
        let width = 28;

        let input = Tensor::randn(
            [batch_size, in_channels, height, width],
            (Kind::Float, Device::Cpu),
        );
        let output = layer.forward(&input);

        assert_eq!(output.size(), vec![batch_size, out_channels, height, width]);
    }

    #[test]
    fn test_repvgg_block() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let hidden_channels = 256;
        let hidden_expansion = 1.0;

        let block = RTDetrV2RepVggBlock::new(
            &root,
            hidden_channels,
            hidden_expansion,
            Some(Activation::SiLU),
        );

        let batch_size = 2;
        let height = 14;
        let width = 14;

        let input = Tensor::randn(
            [batch_size, hidden_channels, height, width],
            (Kind::Float, Device::Cpu),
        );
        let output = block.forward(&input);

        assert_eq!(
            output.size(),
            vec![batch_size, hidden_channels, height, width]
        );
    }

    #[test]
    fn test_csp_rep_layer() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let encoder_hidden_dim = 256;
        let hidden_expansion = 1.0;

        let layer = RTDetrV2CSPRepLayer::new(
            &root,
            encoder_hidden_dim,
            hidden_expansion,
            Some(Activation::SiLU),
        );

        let batch_size = 2;
        let in_channels = encoder_hidden_dim * 2;
        let height = 14;
        let width = 14;

        let input = Tensor::randn(
            [batch_size, in_channels, height, width],
            (Kind::Float, Device::Cpu),
        );
        let output = layer.forward(&input);

        // Output should have encoder_hidden_dim channels
        assert_eq!(
            output.size(),
            vec![batch_size, encoder_hidden_dim, height, width]
        );
    }

    #[test]
    fn test_encoder_layer() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let config = EncoderLayerConfig {
            encoder_hidden_dim: 256,
            num_attention_heads: 8,
            encoder_ffn_dim: 1024,
            dropout: 0.0, // No dropout for deterministic test
            activation_dropout: 0.0,
            encoder_activation_function: Activation::GELU,
            normalize_before: false,
            layer_norm_eps: 1e-5,
        };

        let encoder_hidden_dim = config.encoder_hidden_dim;
        let encoder_layer = RTDetrV2EncoderLayer::new(&root, config);

        let batch_size = 2;
        let seq_len = 100;

        let hidden_states = Tensor::randn(
            [batch_size, seq_len, encoder_hidden_dim],
            (Kind::Float, Device::Cpu),
        );

        let (output, attn_weights) = encoder_layer.forward(
            &hidden_states,
            None,
            None,
            false,
            false, // eval mode
        );

        assert_eq!(output.size(), vec![batch_size, seq_len, encoder_hidden_dim]);
        assert!(attn_weights.is_none());
    }

    #[test]
    fn test_encoder_layer_with_position_embeddings() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let config = EncoderLayerConfig {
            encoder_hidden_dim: 256,
            num_attention_heads: 8,
            encoder_ffn_dim: 1024,
            dropout: 0.0,
            activation_dropout: 0.0,
            encoder_activation_function: Activation::GELU,
            normalize_before: true, // Test pre-norm variant
            layer_norm_eps: 1e-5,
        };

        let encoder_hidden_dim = config.encoder_hidden_dim;
        let num_attention_heads = config.num_attention_heads;
        let encoder_layer = RTDetrV2EncoderLayer::new(&root, config);

        let batch_size = 2;
        let seq_len = 100;

        let hidden_states = Tensor::randn(
            [batch_size, seq_len, encoder_hidden_dim],
            (Kind::Float, Device::Cpu),
        );
        let position_embeddings = Tensor::randn(
            [batch_size, seq_len, encoder_hidden_dim],
            (Kind::Float, Device::Cpu),
        );

        let (output, attn_weights) = encoder_layer.forward(
            &hidden_states,
            None,
            Some(&position_embeddings),
            true, // output attentions
            false,
        );

        assert_eq!(output.size(), vec![batch_size, seq_len, encoder_hidden_dim]);
        assert!(attn_weights.is_some());
        if let Some(weights) = attn_weights {
            assert_eq!(
                weights.size(),
                vec![batch_size, num_attention_heads, seq_len, seq_len]
            );
        }
    }

    #[test]
    fn test_encoder() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let num_encoder_layers = 3;
        let config = EncoderLayerConfig {
            encoder_hidden_dim: 256,
            num_attention_heads: 8,
            encoder_ffn_dim: 1024,
            dropout: 0.0,
            activation_dropout: 0.0,
            encoder_activation_function: Activation::GELU,
            normalize_before: false,
            layer_norm_eps: 1e-5,
        };

        let encoder_hidden_dim = config.encoder_hidden_dim;
        let encoder = RTDetrV2Encoder::new(&root, num_encoder_layers, config);

        let batch_size = 2;
        let seq_len = 100;

        let src = Tensor::randn(
            [batch_size, seq_len, encoder_hidden_dim],
            (Kind::Float, Device::Cpu),
        );

        let (output, all_attentions) = encoder.forward(
            &src, None, None, false, // don't output attentions
            false, // eval mode
        );

        assert_eq!(output.size(), vec![batch_size, seq_len, encoder_hidden_dim]);
        assert_eq!(all_attentions.len(), num_encoder_layers as usize);
        assert!(all_attentions.iter().all(|attn| attn.is_none()));
    }

    #[test]
    fn test_encoder_with_attentions() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let num_encoder_layers = 2;
        let config = EncoderLayerConfig {
            encoder_hidden_dim: 128,
            num_attention_heads: 4,
            encoder_ffn_dim: 512,
            dropout: 0.0,
            activation_dropout: 0.0,
            encoder_activation_function: Activation::SiLU,
            normalize_before: true,
            layer_norm_eps: 1e-5,
        };

        let encoder_hidden_dim = config.encoder_hidden_dim;
        let num_attention_heads = config.num_attention_heads;
        let encoder = RTDetrV2Encoder::new(&root, num_encoder_layers, config);

        let batch_size = 1;
        let seq_len = 50;

        let src = Tensor::randn(
            [batch_size, seq_len, encoder_hidden_dim],
            (Kind::Float, Device::Cpu),
        );

        let (output, all_attentions) = encoder.forward(
            &src, None, None, true, // output attentions
            false,
        );

        assert_eq!(output.size(), vec![batch_size, seq_len, encoder_hidden_dim]);
        assert_eq!(all_attentions.len(), num_encoder_layers as usize);

        // Check all attention weights are present and have correct shape
        for attn_weights in &all_attentions {
            assert!(attn_weights.is_some());
            if let Some(weights) = attn_weights {
                assert_eq!(
                    weights.size(),
                    vec![batch_size, num_attention_heads, seq_len, seq_len]
                );
            }
        }
    }

    #[test]
    fn test_position_embedding() {
        let width = 32;
        let height = 24;
        let embed_dim = 256;
        let temperature = 10000.0;
        let device = Device::Cpu;

        let pos_embed = RTDetrV2HybridEncoder::build_2d_sincos_position_embedding(
            width,
            height,
            embed_dim,
            temperature,
            device,
        )
        .unwrap();

        assert_eq!(pos_embed.size(), vec![1, width * height, embed_dim]);

        let min_val = pos_embed.min().double_value(&[]);
        let max_val = pos_embed.max().double_value(&[]);
        assert!((-1.1..=1.1).contains(&min_val));
        assert!((-1.1..=1.1).contains(&max_val));
    }

    #[test]
    fn test_position_embedding_invalid_dim() {
        let width = 32;
        let height = 24;
        let embed_dim = 255;
        let temperature = 10000.0;
        let device = Device::Cpu;

        let result = RTDetrV2HybridEncoder::build_2d_sincos_position_embedding(
            width,
            height,
            embed_dim,
            temperature,
            device,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_hybrid_encoder() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let encoder_hidden_dim = 256;
        let encoder_layers = 1;
        let encode_proj_layers = vec![2];
        let num_fpn_stages = 2;
        let num_pan_stages = 2;
        let activation = Activation::SiLU;
        let positional_encoding_temperature = 10000.0;

        let encoder_config = EncoderLayerConfig {
            encoder_hidden_dim,
            num_attention_heads: 8,
            encoder_ffn_dim: 1024,
            dropout: 0.0,
            activation_dropout: 0.0,
            encoder_activation_function: Activation::GELU,
            normalize_before: false,
            layer_norm_eps: 1e-5,
        };

        let hybrid_encoder = RTDetrV2HybridEncoder::new(
            &root,
            encoder_hidden_dim,
            encoder_layers,
            encode_proj_layers,
            num_fpn_stages,
            num_pan_stages,
            activation,
            encoder_config,
            positional_encoding_temperature,
            None,
        );

        let batch_size = 1;
        let feature_map_1 = Tensor::randn(
            [batch_size, encoder_hidden_dim, 64, 64],
            (Kind::Float, Device::Cpu),
        );
        let feature_map_2 = Tensor::randn(
            [batch_size, encoder_hidden_dim, 32, 32],
            (Kind::Float, Device::Cpu),
        );
        let feature_map_3 = Tensor::randn(
            [batch_size, encoder_hidden_dim, 16, 16],
            (Kind::Float, Device::Cpu),
        );
        let hidden_states = vec![feature_map_1, feature_map_2, feature_map_3];

        let output = hybrid_encoder
            .forward(&hidden_states, false, false)
            .unwrap();

        assert_eq!(output.len(), (num_pan_stages + 1) as usize);

        for (i, feature_map) in output.iter().enumerate() {
            let expected_spatial_size = 64 / (2_i64.pow(i as u32));
            assert_eq!(
                feature_map.size(),
                vec![
                    batch_size,
                    encoder_hidden_dim,
                    expected_spatial_size,
                    expected_spatial_size
                ]
            );
        }
    }

    #[test]
    fn test_hybrid_encoder_no_encoder_layers() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let encoder_hidden_dim = 256;
        let encoder_layers = 0;
        let encode_proj_layers = vec![];
        let num_fpn_stages = 2;
        let num_pan_stages = 2;
        let activation = Activation::SiLU;
        let positional_encoding_temperature = 10000.0;

        let encoder_config = EncoderLayerConfig {
            encoder_hidden_dim,
            num_attention_heads: 8,
            encoder_ffn_dim: 1024,
            dropout: 0.0,
            activation_dropout: 0.0,
            encoder_activation_function: Activation::GELU,
            normalize_before: false,
            layer_norm_eps: 1e-5,
        };

        let hybrid_encoder = RTDetrV2HybridEncoder::new(
            &root,
            encoder_hidden_dim,
            encoder_layers,
            encode_proj_layers,
            num_fpn_stages,
            num_pan_stages,
            activation,
            encoder_config,
            positional_encoding_temperature,
            None,
        );

        let batch_size = 1;
        let feature_map_1 = Tensor::randn(
            [batch_size, encoder_hidden_dim, 64, 64],
            (Kind::Float, Device::Cpu),
        );
        let feature_map_2 = Tensor::randn(
            [batch_size, encoder_hidden_dim, 32, 32],
            (Kind::Float, Device::Cpu),
        );
        let feature_map_3 = Tensor::randn(
            [batch_size, encoder_hidden_dim, 16, 16],
            (Kind::Float, Device::Cpu),
        );
        let hidden_states = vec![feature_map_1, feature_map_2, feature_map_3];

        let output = hybrid_encoder
            .forward(&hidden_states, false, false)
            .unwrap();

        assert_eq!(output.len(), (num_pan_stages + 1) as usize);
    }
}
