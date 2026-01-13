// Basic transformer building blocks for RT-DETR v2
// Ported from transformers/models/rt_detr_v2/modeling_rt_detr_v2.py:225-344

use ndarray::{Array1, Array2, Array3};
use std::sync::atomic::{AtomicBool, Ordering};
use tch::{nn, Kind, Tensor};

// Global flag to only capture attention outputs once (from first encoder layer)
static ATTENTION_DEBUG_CAPTURED: AtomicBool = AtomicBool::new(false);

/// Helper function to save tensor as numpy .npy file for debugging
fn save_tensor_as_numpy(tensor: &Tensor, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Convert tensor to CPU and f32
    let tensor_cpu = tensor.to_kind(Kind::Float).to(tch::Device::Cpu);
    let shape = tensor_cpu.size();

    // Convert to Vec<f32>
    let tensor_flat = tensor_cpu.flatten(0, -1);
    let data: Vec<f32> = Vec::try_from(&tensor_flat)?;

    // Save based on dimensionality
    match shape.len() {
        1 => {
            let array = Array1::from_shape_vec((shape[0] as usize,), data)?;
            ndarray_npy::write_npy(path, &array)?;
        }
        2 => {
            let array = Array2::from_shape_vec((shape[0] as usize, shape[1] as usize), data)?;
            ndarray_npy::write_npy(path, &array)?;
        }
        3 => {
            let array = Array3::from_shape_vec(
                (shape[0] as usize, shape[1] as usize, shape[2] as usize),
                data,
            )?;
            ndarray_npy::write_npy(path, &array)?;
        }
        _ => {
            log::warn!(
                "Warning: Cannot save tensor with {} dimensions",
                shape.len()
            );
        }
    }

    Ok(())
}

/// Multi-headed attention from 'Attention Is All You Need' paper
/// Python: RTDetrV2MultiheadAttention (lines 225-343)
#[derive(Debug)]
pub struct RTDetrV2MultiheadAttention {
    pub embed_dim: i64,
    pub num_heads: i64,
    pub head_dim: i64,
    pub scaling: f64,
    pub dropout: f64,

    pub k_proj: nn::Linear,
    pub v_proj: nn::Linear,
    pub q_proj: nn::Linear,
    pub out_proj: nn::Linear,
}

impl RTDetrV2MultiheadAttention {
    pub fn new(vs: &nn::Path, embed_dim: i64, num_heads: i64, dropout: f64, bias: bool) -> Self {
        let head_dim = embed_dim / num_heads;

        assert!(
            head_dim * num_heads == embed_dim,
            "embed_dim {} must be divisible by num_heads {}",
            embed_dim,
            num_heads
        );

        let scaling = (head_dim as f64).powf(-0.5);

        let linear_config = nn::LinearConfig {
            bias,
            ..Default::default()
        };

        let k_proj = nn::linear(vs / "k_proj", embed_dim, embed_dim, linear_config);
        let v_proj = nn::linear(vs / "v_proj", embed_dim, embed_dim, linear_config);
        let q_proj = nn::linear(vs / "q_proj", embed_dim, embed_dim, linear_config);
        let out_proj = nn::linear(vs / "out_proj", embed_dim, embed_dim, linear_config);

        Self {
            embed_dim,
            num_heads,
            head_dim,
            scaling,
            dropout,
            k_proj,
            v_proj,
            q_proj,
            out_proj,
        }
    }

    /// Reshape tensor for multi-head attention
    /// [batch_size, seq_len, embed_dim] -> [batch_size, num_heads, seq_len, head_dim]
    fn reshape(&self, tensor: &Tensor, seq_len: i64, batch_size: i64) -> Tensor {
        tensor
            .view([batch_size, seq_len, self.num_heads, self.head_dim])
            .transpose(1, 2)
            .contiguous()
    }

    /// Add position embeddings to tensor
    fn with_pos_embed(&self, tensor: &Tensor, position_embeddings: Option<&Tensor>) -> Tensor {
        if let Some(pos_emb) = position_embeddings {
            tensor + pos_emb
        } else {
            tensor.shallow_clone()
        }
    }

    /// Forward pass
    ///
    /// Args:
    ///   hidden_states: [batch_size, seq_len, embed_dim]
    ///   attention_mask: Optional [batch_size, 1, target_len, source_len] or [seq_len, seq_len]
    ///   position_embeddings: Optional position embeddings to add to queries and keys
    ///   output_attentions: Whether to return attention weights
    ///
    /// Returns: (output, attention_weights_reshaped)
    ///   output: [batch_size, seq_len, embed_dim]
    ///   attention_weights_reshaped: Optional [batch_size, num_heads, target_len, source_len]
    pub fn forward(
        &self,
        hidden_states: &Tensor,
        attention_mask: Option<&Tensor>,
        position_embeddings: Option<&Tensor>,
        output_attentions: bool,
        train: bool,
        debug_name: Option<&str>,
    ) -> (Tensor, Option<Tensor>) {
        let shape = hidden_states.size();
        let batch_size = shape[0];
        let target_len = shape[1];

        // Determine if we should capture based on debug_name parameter
        let decoder_debug_requested =
            std::env::var("DEBUG_SAVE_DECODER_SELF_ATTN_INTERNALS").is_ok();
        let is_decoder_layer_0 = debug_name == Some("decoder_layer_0");
        let should_capture_encoder =
            !decoder_debug_requested && !ATTENTION_DEBUG_CAPTURED.swap(true, Ordering::SeqCst);
        let should_capture_decoder = decoder_debug_requested && is_decoder_layer_0;
        let should_capture = should_capture_encoder || should_capture_decoder;

        // Debug: Save input (with different prefix for decoder vs encoder)
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ =
                save_tensor_as_numpy(hidden_states, &format!("{}input_hidden_states.npy", prefix));
        }

        // Add position embeddings to the hidden states before projecting to queries and keys
        let (hidden_states_for_qk, hidden_states_original) = if position_embeddings.is_some() {
            let hidden_states_original = hidden_states.shallow_clone();
            let hidden_states_with_pos = self.with_pos_embed(hidden_states, position_embeddings);

            // Debug: Save position embeddings and combined state
            if should_capture {
                let prefix = if should_capture_decoder {
                    "rust_decoder_self_attn_"
                } else {
                    "rust_attn_"
                };
                #[allow(
                    clippy::unnecessary_unwrap,
                    reason = "unwrap safe - guarded by is_some check above"
                )]
                let _ = save_tensor_as_numpy(
                    position_embeddings.unwrap(),
                    &format!("{}position_embeddings.npy", prefix),
                );
                let _ = save_tensor_as_numpy(
                    &hidden_states_with_pos,
                    &format!("{}hidden_states_with_pos.npy", prefix),
                );
            }

            (hidden_states_with_pos, hidden_states_original)
        } else {
            (hidden_states.shallow_clone(), hidden_states.shallow_clone())
        };

        // Get queries, keys, and values
        // Query: project then scale
        let query_states_raw = hidden_states_for_qk.apply(&self.q_proj);
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ = save_tensor_as_numpy(
                &query_states_raw,
                &format!("{}query_after_proj.npy", prefix),
            );
        }

        let query_states = &query_states_raw * self.scaling;
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ =
                save_tensor_as_numpy(&query_states, &format!("{}query_after_scaling.npy", prefix));
        }

        // Key and Value projections
        let key_states_raw = hidden_states_for_qk.apply(&self.k_proj);
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ = save_tensor_as_numpy(&key_states_raw, &format!("{}key_after_proj.npy", prefix));
        }

        let value_states_raw = hidden_states_original.apply(&self.v_proj);
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ = save_tensor_as_numpy(
                &value_states_raw,
                &format!("{}value_after_proj.npy", prefix),
            );
        }

        let key_states = self.reshape(&key_states_raw, -1, batch_size);
        let value_states = self.reshape(&value_states_raw, -1, batch_size);

        // Reshape query, key, value for attention
        let proj_shape = [batch_size * self.num_heads, -1, self.head_dim];
        let query_states = self
            .reshape(&query_states, target_len, batch_size)
            .view(proj_shape.as_slice());
        let key_states = key_states.view(proj_shape.as_slice());
        let value_states = value_states.view(proj_shape.as_slice());

        // Debug: Save reshaped states
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ = save_tensor_as_numpy(&query_states, &format!("{}query_reshaped.npy", prefix));
            let _ = save_tensor_as_numpy(&key_states, &format!("{}key_reshaped.npy", prefix));
            let _ = save_tensor_as_numpy(&value_states, &format!("{}value_reshaped.npy", prefix));
        }

        let source_len = key_states.size()[1];

        // Compute attention weights: Q @ K^T
        let attn_weights = query_states.bmm(&key_states.transpose(1, 2));

        // Debug: Save attention scores (before softmax)
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ = save_tensor_as_numpy(&attn_weights, &format!("{}attn_scores.npy", prefix));
        }

        // Validate attention weights shape
        let expected_attn_shape = [batch_size * self.num_heads, target_len, source_len];
        assert_eq!(
            attn_weights.size(),
            expected_attn_shape,
            "Attention weights should be {:?}, but is {:?}",
            expected_attn_shape,
            attn_weights.size()
        );

        // Apply attention mask if provided
        let attn_weights = if let Some(mask) = attention_mask {
            // Expand mask if needed: [seq_len, seq_len] -> [batch_size, 1, target_len, source_len]
            let mask = if mask.size().len() == 2 {
                mask.unsqueeze(0)
                    .unsqueeze(0)
                    .expand([batch_size, 1, target_len, source_len].as_slice(), false)
            } else {
                mask.shallow_clone()
            };

            // Validate mask shape
            assert_eq!(
                mask.size(),
                vec![batch_size, 1, target_len, source_len],
                "Attention mask should be [batch_size, 1, target_len, source_len]"
            );

            // Convert bool mask to float mask with -inf for masked positions
            let mask = if mask.kind() == Kind::Bool {
                let zeros = Tensor::zeros_like(&mask).to_kind(attn_weights.kind());
                zeros.masked_fill(&mask, f64::NEG_INFINITY)
            } else {
                mask
            };

            // Add mask to attention weights
            let attn_weights =
                attn_weights.view([batch_size, self.num_heads, target_len, source_len]);
            let attn_weights = attn_weights + mask;
            attn_weights.view([batch_size * self.num_heads, target_len, source_len])
        } else {
            attn_weights
        };

        // Apply softmax
        let attn_weights = attn_weights.softmax(-1, Kind::Float);

        // Debug: Save attention weights (after softmax)
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ = save_tensor_as_numpy(
                &attn_weights,
                &format!("{}attn_weights_after_softmax.npy", prefix),
            );
        }

        // Prepare attention weights for output if needed
        let attn_weights_reshaped = if output_attentions {
            Some(attn_weights.view([batch_size, self.num_heads, target_len, source_len]))
        } else {
            None
        };

        // Apply dropout
        let attn_probs = if train {
            attn_weights.dropout(self.dropout, train)
        } else {
            attn_weights.shallow_clone()
        };

        // Debug: Save attention probs (after dropout)
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ = save_tensor_as_numpy(
                &attn_probs,
                &format!("{}attn_probs_after_dropout.npy", prefix),
            );
        }

        // Compute attention output: attn_probs @ V
        let attn_output = attn_probs.bmm(&value_states);

        // Debug: Save attention output (before reshape)
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ = save_tensor_as_numpy(
                &attn_output,
                &format!("{}attn_output_before_reshape.npy", prefix),
            );
        }

        // Validate output shape
        assert_eq!(
            attn_output.size(),
            vec![batch_size * self.num_heads, target_len, self.head_dim],
            "attn_output should be [batch * num_heads, target_len, head_dim]"
        );

        // Reshape output back to [batch_size, target_len, embed_dim]
        let attn_output = attn_output
            .view([batch_size, self.num_heads, target_len, self.head_dim])
            .transpose(1, 2)
            .reshape([batch_size, target_len, self.embed_dim]);

        // Debug: Save attention output (before output projection)
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ = save_tensor_as_numpy(
                &attn_output,
                &format!("{}attn_output_before_out_proj.npy", prefix),
            );
        }

        // Apply output projection
        let attn_output = attn_output.apply(&self.out_proj);

        // Debug: Save final attention output
        if should_capture {
            let prefix = if should_capture_decoder {
                "rust_decoder_self_attn_"
            } else {
                "rust_attn_"
            };
            let _ = save_tensor_as_numpy(&attn_output, &format!("{}attn_output_final.npy", prefix));
        }

        (attn_output, attn_weights_reshaped)
    }
}

/// Activation functions for transformer layers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    ReLU,
    GELU,
    SiLU,
}

impl Activation {
    pub fn apply(&self, tensor: &Tensor) -> Tensor {
        match self {
            Activation::ReLU => tensor.relu(),
            Activation::GELU => {
                // Exact GELU implementation using error function (matches Python)
                // GELU(x) = 0.5 * x * (1 + erf(x / sqrt(2)))
                let x = tensor;
                let x_normalized = x / (2.0_f64.sqrt());
                0.5_f64 * x * (1.0_f64 + x_normalized.erf())
            }
            Activation::SiLU => tensor.silu(),
        }
    }
}

impl std::fmt::Display for Activation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Activation::ReLU => write!(f, "relu"),
            Activation::GELU => write!(f, "gelu"),
            Activation::SiLU => write!(f, "silu"),
        }
    }
}

impl std::str::FromStr for Activation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "relu" => Ok(Activation::ReLU),
            "gelu" => Ok(Activation::GELU),
            "silu" | "swish" => Ok(Activation::SiLU),
            _ => Err(format!("unknown activation function: '{s}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tch::Device;

    #[test]
    fn test_multihead_attention_shapes() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let embed_dim = 256;
        let num_heads = 8;
        let dropout = 0.1;
        let bias = true;

        let attention = RTDetrV2MultiheadAttention::new(&root, embed_dim, num_heads, dropout, bias);

        let batch_size = 2;
        let seq_len = 100;

        let hidden_states =
            Tensor::randn([batch_size, seq_len, embed_dim], (Kind::Float, Device::Cpu));

        let (output, attn_weights) =
            attention.forward(&hidden_states, None, None, false, false, None);

        assert_eq!(output.size(), vec![batch_size, seq_len, embed_dim]);
        assert!(attn_weights.is_none());
    }

    #[test]
    fn test_multihead_attention_with_position_embeddings() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let embed_dim = 256;
        let num_heads = 8;
        let dropout = 0.0; // No dropout for deterministic test

        let attention = RTDetrV2MultiheadAttention::new(&root, embed_dim, num_heads, dropout, true);

        let batch_size = 2;
        let seq_len = 100;

        let hidden_states =
            Tensor::randn([batch_size, seq_len, embed_dim], (Kind::Float, Device::Cpu));
        let position_embeddings =
            Tensor::randn([batch_size, seq_len, embed_dim], (Kind::Float, Device::Cpu));

        let (output, attn_weights) = attention.forward(
            &hidden_states,
            None,
            Some(&position_embeddings),
            true,
            false,
            None,
        );

        assert_eq!(output.size(), vec![batch_size, seq_len, embed_dim]);
        assert!(attn_weights.is_some());
        if let Some(weights) = attn_weights {
            assert_eq!(
                weights.size(),
                vec![batch_size, num_heads, seq_len, seq_len]
            );
        }
    }

    #[test]
    fn test_multihead_attention_with_mask() {
        let vs = nn::VarStore::new(Device::Cpu);
        let root = vs.root();

        let embed_dim = 128;
        let num_heads = 4;
        let dropout = 0.0;

        let attention = RTDetrV2MultiheadAttention::new(&root, embed_dim, num_heads, dropout, true);

        let batch_size = 2;
        let seq_len = 10;

        let hidden_states =
            Tensor::randn([batch_size, seq_len, embed_dim], (Kind::Float, Device::Cpu));

        // Create a causal mask (upper triangular)
        let mask = Tensor::ones([seq_len, seq_len], (Kind::Bool, Device::Cpu)).triu(1);

        let (output, _) = attention.forward(&hidden_states, Some(&mask), None, false, false, None);

        assert_eq!(output.size(), vec![batch_size, seq_len, embed_dim]);
    }

    #[test]
    fn test_activation_functions() {
        let tensor = Tensor::randn([2, 3, 4], (Kind::Float, Device::Cpu));

        // Test ReLU
        let relu_out = Activation::ReLU.apply(&tensor);
        assert_eq!(relu_out.size(), vec![2, 3, 4]);

        // Test GELU
        let gelu_out = Activation::GELU.apply(&tensor);
        assert_eq!(gelu_out.size(), vec![2, 3, 4]);

        // Test SiLU
        let silu_out = Activation::SiLU.apply(&tensor);
        assert_eq!(silu_out.size(), vec![2, 3, 4]);
    }

    #[test]
    fn test_activation_display() {
        assert_eq!(Activation::ReLU.to_string(), "relu");
        assert_eq!(Activation::GELU.to_string(), "gelu");
        assert_eq!(Activation::SiLU.to_string(), "silu");
    }

    #[test]
    fn test_activation_from_str() {
        use std::str::FromStr;

        // Standard cases
        assert_eq!(Activation::from_str("relu").unwrap(), Activation::ReLU);
        assert_eq!(Activation::from_str("gelu").unwrap(), Activation::GELU);
        assert_eq!(Activation::from_str("silu").unwrap(), Activation::SiLU);

        // Case insensitivity
        assert_eq!(Activation::from_str("RELU").unwrap(), Activation::ReLU);
        assert_eq!(Activation::from_str("SiLU").unwrap(), Activation::SiLU);

        // Alias
        assert_eq!(Activation::from_str("swish").unwrap(), Activation::SiLU);

        // Error case
        assert!(Activation::from_str("unknown").is_err());
    }

    #[test]
    fn test_activation_roundtrip() {
        use std::str::FromStr;

        for act in [Activation::ReLU, Activation::GELU, Activation::SiLU] {
            let s = act.to_string();
            assert_eq!(Activation::from_str(&s).unwrap(), act);
        }
    }
}
