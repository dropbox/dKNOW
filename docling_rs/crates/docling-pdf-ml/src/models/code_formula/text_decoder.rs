// Text decoder for Idefics3 (Llama-based causal language model)
// Based on HuggingFace transformers/models/llama/modeling_llama.py

use crate::models::code_formula::config::TextConfig;
use tch::{nn, nn::Module, Tensor};

/// Root Mean Square Layer Normalization
///
/// RMSNorm normalizes using root mean square instead of mean and variance.
/// More efficient than LayerNorm and performs similarly.
///
/// Formula: output = input / rms(input) * weight
/// where rms(input) = sqrt(mean(input^2) + eps)
pub struct RMSNorm {
    weight: Tensor,
    eps: f64,
}

impl std::fmt::Debug for RMSNorm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RMSNorm")
            .field("eps", &self.eps)
            .field("weight", &"<Tensor>")
            .finish()
    }
}

impl RMSNorm {
    pub fn new(vs: &nn::Path, hidden_size: i64, eps: f64) -> Self {
        let weight = vs.var("weight", &[hidden_size], nn::Init::Const(1.0));
        Self { weight, eps }
    }

    /// Apply RMSNorm
    ///
    /// Input: hidden_states [..., hidden_size]
    /// Output: normalized [..., hidden_size]
    pub fn forward(&self, hidden_states: &Tensor) -> Tensor {
        let input_dtype = hidden_states.kind();

        // Convert to float32 for numerical stability
        let hidden_states_f32 = hidden_states.to_kind(tch::Kind::Float);

        // Compute variance: mean(x^2, dim=-1, keepdim=True)
        let variance = hidden_states_f32.pow_tensor_scalar(2).mean_dim(
            &[-1i64][..],
            true, // keepdim
            tch::Kind::Float,
        );

        // Normalize: x / sqrt(variance + eps)
        let normalized = &hidden_states_f32 * (variance + self.eps).rsqrt();

        // Apply weight and convert back to original dtype
        (&self.weight * normalized).to_kind(input_dtype)
    }
}

/// Rotary Position Embeddings (RoPE)
///
/// RoPE encodes position information by rotating query and key embeddings.
/// This allows the model to be aware of token positions without learned embeddings.
///
/// Reference: <https://arxiv.org/abs/2104.09864>
pub struct RotaryEmbedding {
    inv_freq: Tensor,
    dim: i64,
}

impl std::fmt::Debug for RotaryEmbedding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RotaryEmbedding")
            .field("dim", &self.dim)
            .field("inv_freq", &"<Tensor>")
            .finish()
    }
}

impl RotaryEmbedding {
    pub fn new(vs: &nn::Path, dim: i64, _max_position_embeddings: i64, base: f64) -> Self {
        // Compute inverse frequencies: 1 / (base^(2i/dim)) for i in 0..dim/2
        // Formula: inv_freq[i] = 1 / (base^(2i/dim)) for i = 0, 1, 2, ..., dim/2-1
        let device = vs.device();

        // Create indices [0, 1, 2, ..., dim/2-1]
        let half_dim = dim / 2;
        let indices = Tensor::arange(half_dim, (tch::Kind::Float, device));

        // Compute exponents: 2i/dim
        let exponents = (indices * 2.0) / (dim as f64);

        // Compute inv_freq: base^(-2i/dim) = exp(-2i/dim * ln(base))
        let inv_freq = (exponents.neg() * base.ln()).exp();

        // Note: inv_freq is computed, not loaded from weights
        // Do NOT register with VarStore (SafeTensors doesn't have this buffer)

        Self { inv_freq, dim }
    }

    /// Compute cos and sin embeddings for given sequence length
    ///
    /// Returns: (cos, sin) tensors of shape [1, seq_len, dim]
    pub fn forward(&self, seq_len: i64, device: tch::Device) -> (Tensor, Tensor) {
        // Create position indices: [0, 1, 2, ..., seq_len-1]
        let position_ids = Tensor::arange(seq_len, (tch::Kind::Float, device));

        // Compute freqs: position_ids @ inv_freq^T
        // [seq_len, 1] @ [1, dim/2] → [seq_len, dim/2]
        let freqs = position_ids
            .unsqueeze(1)
            .matmul(&self.inv_freq.to_device(device).unsqueeze(0));

        // Concatenate freqs with itself to get full dimension
        // [seq_len, dim/2] → [seq_len, dim]
        let emb = Tensor::cat(&[&freqs, &freqs], -1);

        // Compute cos and sin
        let cos = emb.cos().unsqueeze(0); // [1, seq_len, dim]
        let sin = emb.sin().unsqueeze(0); // [1, seq_len, dim]

        (cos, sin)
    }
}

/// Apply rotary position embeddings to query and key tensors
///
/// Input: q, k [batch, num_heads, seq_len, head_dim]
/// Output: q_embed, k_embed with RoPE applied
fn apply_rotary_pos_emb(q: &Tensor, k: &Tensor, cos: &Tensor, sin: &Tensor) -> (Tensor, Tensor) {
    // Unsqueeze cos/sin to broadcast: [1, seq_len, dim] → [1, 1, seq_len, dim]
    let cos = cos.unsqueeze(1);
    let sin = sin.unsqueeze(1);

    // Apply rotation: (q * cos) + (rotate_half(q) * sin)
    let q_embed = (q * &cos) + (rotate_half(q) * &sin);
    let k_embed = (k * &cos) + (rotate_half(k) * &sin);

    (q_embed, k_embed)
}

/// Rotate half the hidden dimensions
///
/// Helper for RoPE. Splits tensor in half along last dim and rotates.
/// Input: [..., dim] → Output: [... -x2, x1]
#[inline]
fn rotate_half(x: &Tensor) -> Tensor {
    let dim = x.size()[x.size().len() - 1];
    let half_dim = dim / 2;

    // Split into two halves
    let x1 = x.slice(-1, 0, half_dim, 1);
    let x2 = x.slice(-1, half_dim, dim, 1);

    // Concatenate [-x2, x1]
    Tensor::cat(&[&(-x2), &x1], -1)
}

/// Repeat key/value tensors for grouped-query attention
///
/// Input: [batch, num_kv_heads, seq_len, head_dim]
/// Output: [batch, num_q_heads, seq_len, head_dim]
///
/// This expands KV heads to match Q heads when num_kv_heads < num_q_heads.
fn repeat_kv(hidden_states: &Tensor, n_rep: i64) -> Tensor {
    if n_rep == 1 {
        return hidden_states.shallow_clone();
    }

    let size = hidden_states.size();
    let batch = size[0];
    let num_kv_heads = size[1];
    let seq_len = size[2];
    let head_dim = size[3];

    // Reshape: [B, H_kv, S, D] → [B, H_kv, 1, S, D]
    let expanded = hidden_states
        .view([batch, num_kv_heads, 1, seq_len, head_dim])
        .expand([batch, num_kv_heads, n_rep, seq_len, head_dim], false);

    // Reshape back: [B, H_kv, n_rep, S, D] → [B, H_kv*n_rep, S, D]
    expanded.reshape([batch, num_kv_heads * n_rep, seq_len, head_dim])
}

/// Grouped-Query Attention for Llama
///
/// Uses fewer KV heads than Q heads for efficiency (e.g., 9 Q heads, 3 KV heads).
/// KV heads are repeated to match Q heads during attention computation.
pub struct GroupedQueryAttention {
    q_proj: nn::Linear,
    k_proj: nn::Linear,
    v_proj: nn::Linear,
    o_proj: nn::Linear,
    num_q_heads: i64,
    num_kv_heads: i64,
    num_kv_groups: i64,
    head_dim: i64,
    scale: f64,
    dropout: f64,
}

impl std::fmt::Debug for GroupedQueryAttention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedQueryAttention")
            .field("num_q_heads", &self.num_q_heads)
            .field("num_kv_heads", &self.num_kv_heads)
            .field("num_kv_groups", &self.num_kv_groups)
            .field("head_dim", &self.head_dim)
            .field("scale", &self.scale)
            .field("dropout", &self.dropout)
            .field("q_proj", &"<Linear>")
            .field("k_proj", &"<Linear>")
            .field("v_proj", &"<Linear>")
            .field("o_proj", &"<Linear>")
            .finish()
    }
}

impl GroupedQueryAttention {
    pub fn new(vs: &nn::Path, config: &TextConfig) -> Self {
        let hidden_size = config.hidden_size as i64;
        let num_q_heads = config.num_attention_heads as i64;
        let num_kv_heads = config.num_key_value_heads as i64;
        let head_dim = config.head_dim as i64;
        let attention_bias = config.attention_bias;
        let dropout = config.attention_dropout;

        let num_kv_groups = num_q_heads / num_kv_heads;
        let scale = (head_dim as f64).powf(-0.5);

        // Q projection: hidden_size → num_q_heads * head_dim
        let q_proj = nn::linear(
            vs / "q_proj",
            hidden_size,
            num_q_heads * head_dim,
            nn::LinearConfig {
                bias: attention_bias,
                ..Default::default()
            },
        );

        // K, V projections: hidden_size → num_kv_heads * head_dim (fewer heads)
        let k_proj = nn::linear(
            vs / "k_proj",
            hidden_size,
            num_kv_heads * head_dim,
            nn::LinearConfig {
                bias: attention_bias,
                ..Default::default()
            },
        );
        let v_proj = nn::linear(
            vs / "v_proj",
            hidden_size,
            num_kv_heads * head_dim,
            nn::LinearConfig {
                bias: attention_bias,
                ..Default::default()
            },
        );

        // Output projection: num_q_heads * head_dim → hidden_size
        let o_proj = nn::linear(
            vs / "o_proj",
            num_q_heads * head_dim,
            hidden_size,
            nn::LinearConfig {
                bias: attention_bias,
                ..Default::default()
            },
        );

        Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            num_q_heads,
            num_kv_heads,
            num_kv_groups,
            head_dim,
            scale,
            dropout,
        }
    }

    /// Forward pass: compute grouped-query attention with RoPE
    ///
    /// Input:
    /// - hidden_states: [batch, seq_len, hidden_size]
    /// - cos, sin: RoPE embeddings [1, seq_len, head_dim]
    /// - attention_mask: Optional causal mask [batch, 1, seq_len, seq_len]
    /// - train: Whether in training mode (for dropout)
    ///
    /// Output: [batch, seq_len, hidden_size]
    pub fn forward(
        &self,
        hidden_states: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        let size = hidden_states.size();
        let batch_size = size[0];
        let seq_len = size[1];

        // Project Q, K, V
        let queries = hidden_states.apply(&self.q_proj); // [B, S, num_q_heads * head_dim]
        let keys = hidden_states.apply(&self.k_proj); // [B, S, num_kv_heads * head_dim]
        let values = hidden_states.apply(&self.v_proj); // [B, S, num_kv_heads * head_dim]

        // Reshape for multi-head attention
        // [B, S, H*D] → [B, S, H, D] → [B, H, S, D]
        let queries = queries
            .view([batch_size, seq_len, self.num_q_heads, self.head_dim])
            .transpose(1, 2);
        let keys = keys
            .view([batch_size, seq_len, self.num_kv_heads, self.head_dim])
            .transpose(1, 2);
        let values = values
            .view([batch_size, seq_len, self.num_kv_heads, self.head_dim])
            .transpose(1, 2);

        // Apply RoPE to Q and K
        let (queries, keys) = apply_rotary_pos_emb(&queries, &keys, cos, sin);

        // Repeat KV heads to match Q heads (grouped-query attention)
        let keys = repeat_kv(&keys, self.num_kv_groups);
        let values = repeat_kv(&values, self.num_kv_groups);

        // Compute attention scores: Q @ K^T * scale
        // [B, H_q, S, D] @ [B, H_q, D, S] → [B, H_q, S, S]
        let mut attn_weights = queries.matmul(&keys.transpose(-2, -1)) * self.scale;

        // Apply attention mask if provided (causal mask for autoregressive generation)
        if let Some(mask) = attention_mask {
            attn_weights += mask;
        }

        // Softmax over last dimension
        let attn_weights = attn_weights.softmax(-1, tch::Kind::Float);

        // Apply dropout if training
        let attn_weights = if train && self.dropout > 0.0 {
            attn_weights.dropout(self.dropout, train)
        } else {
            attn_weights
        };

        // Apply attention to values: attn_weights @ V
        // [B, H_q, S, S] @ [B, H_q, S, D] → [B, H_q, S, D]
        let attn_output = attn_weights.matmul(&values);

        // Reshape back: [B, H_q, S, D] → [B, S, H_q, D] → [B, S, H_q*D]
        let attn_output = attn_output.transpose(1, 2).contiguous().view([
            batch_size,
            seq_len,
            self.num_q_heads * self.head_dim,
        ]);

        // Output projection
        let output = attn_output.apply(&self.o_proj);

        Ok(output)
    }

    // Accessor methods for debugging
    #[inline]
    #[must_use = "returns the query projection layer reference"]
    pub fn q_proj(&self) -> &nn::Linear {
        &self.q_proj
    }

    #[inline]
    #[must_use = "returns the key projection layer reference"]
    pub fn k_proj(&self) -> &nn::Linear {
        &self.k_proj
    }

    #[inline]
    #[must_use = "returns the value projection layer reference"]
    pub fn v_proj(&self) -> &nn::Linear {
        &self.v_proj
    }

    #[inline]
    #[must_use = "returns the output projection layer reference"]
    pub fn o_proj(&self) -> &nn::Linear {
        &self.o_proj
    }

    #[inline]
    #[must_use = "returns the number of query heads"]
    pub const fn num_q_heads(&self) -> i64 {
        self.num_q_heads
    }

    #[inline]
    #[must_use = "returns the number of key-value heads"]
    pub const fn num_kv_heads(&self) -> i64 {
        self.num_kv_heads
    }

    #[inline]
    #[must_use = "returns the attention head dimension"]
    pub const fn head_dim(&self) -> i64 {
        self.head_dim
    }
}

/// SwiGLU MLP (Swish-Gated Linear Unit)
///
/// Llama uses SwiGLU activation instead of standard GELU:
/// SwiGLU(x) = Swish(gate(x)) * up(x)
///
/// Where Swish(x) = x * sigmoid(x) = x * silu(x)
pub struct SwiGLUMLP {
    gate_proj: nn::Linear,
    up_proj: nn::Linear,
    down_proj: nn::Linear,
}

impl std::fmt::Debug for SwiGLUMLP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwiGLUMLP")
            .field("gate_proj", &"<Linear>")
            .field("up_proj", &"<Linear>")
            .field("down_proj", &"<Linear>")
            .finish()
    }
}

impl SwiGLUMLP {
    pub fn new(vs: &nn::Path, config: &TextConfig) -> Self {
        let hidden_size = config.hidden_size as i64;
        let intermediate_size = config.intermediate_size as i64;
        let mlp_bias = config.mlp_bias;

        let gate_proj = nn::linear(
            vs / "gate_proj",
            hidden_size,
            intermediate_size,
            nn::LinearConfig {
                bias: mlp_bias,
                ..Default::default()
            },
        );
        let up_proj = nn::linear(
            vs / "up_proj",
            hidden_size,
            intermediate_size,
            nn::LinearConfig {
                bias: mlp_bias,
                ..Default::default()
            },
        );
        let down_proj = nn::linear(
            vs / "down_proj",
            intermediate_size,
            hidden_size,
            nn::LinearConfig {
                bias: mlp_bias,
                ..Default::default()
            },
        );

        Self {
            gate_proj,
            up_proj,
            down_proj,
        }
    }

    /// Forward pass: SwiGLU activation
    ///
    /// Formula: down(silu(gate(x)) * up(x))
    /// where silu(x) = x * sigmoid(x)
    ///
    /// Input: [batch, seq_len, hidden_size]
    /// Output: [batch, seq_len, hidden_size]
    pub fn forward(&self, hidden_states: &Tensor) -> Tensor {
        let gate_output = hidden_states.apply(&self.gate_proj).silu();
        let up_output = hidden_states.apply(&self.up_proj);
        (gate_output * up_output).apply(&self.down_proj)
    }
}

/// Llama Decoder Layer
///
/// Transformer decoder layer with:
/// - RMSNorm (not LayerNorm)
/// - Grouped-Query Attention with RoPE
/// - SwiGLU MLP
/// - Residual connections
pub struct TextDecoderLayer {
    self_attn: GroupedQueryAttention,
    mlp: SwiGLUMLP,
    input_layernorm: RMSNorm,
    post_attention_layernorm: RMSNorm,
}

impl std::fmt::Debug for TextDecoderLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextDecoderLayer")
            .field("self_attn", &self.self_attn)
            .field("mlp", &self.mlp)
            .field("input_layernorm", &self.input_layernorm)
            .field("post_attention_layernorm", &self.post_attention_layernorm)
            .finish()
    }
}

impl TextDecoderLayer {
    pub fn new(vs: &nn::Path, config: &TextConfig) -> Self {
        let hidden_size = config.hidden_size as i64;
        let rms_norm_eps = config.rms_norm_eps;

        let self_attn = GroupedQueryAttention::new(&(vs / "self_attn"), config);
        let mlp = SwiGLUMLP::new(&(vs / "mlp"), config);
        let input_layernorm = RMSNorm::new(&(vs / "input_layernorm"), hidden_size, rms_norm_eps);
        let post_attention_layernorm = RMSNorm::new(
            &(vs / "post_attention_layernorm"),
            hidden_size,
            rms_norm_eps,
        );

        Self {
            self_attn,
            mlp,
            input_layernorm,
            post_attention_layernorm,
        }
    }

    /// Forward pass: attention + MLP with residuals
    ///
    /// Input:
    /// - hidden_states: [batch, seq_len, hidden_size]
    /// - cos, sin: RoPE embeddings
    /// - attention_mask: Optional causal mask
    /// - train: Whether in training mode
    ///
    /// Output: [batch, seq_len, hidden_size]
    pub fn forward(
        &self,
        hidden_states: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        attention_mask: Option<&Tensor>,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        // Self-attention block with residual
        let residual = hidden_states.shallow_clone();
        let hidden_states = self.input_layernorm.forward(hidden_states);
        let hidden_states =
            self.self_attn
                .forward(&hidden_states, cos, sin, attention_mask, train)?;
        let hidden_states = residual + hidden_states;

        // MLP block with residual
        let residual = hidden_states.shallow_clone();
        let hidden_states = self.post_attention_layernorm.forward(&hidden_states);
        let hidden_states = self.mlp.forward(&hidden_states);
        let hidden_states = residual + hidden_states;

        Ok(hidden_states)
    }

    // Accessor methods for debugging
    #[inline]
    #[must_use = "returns the self-attention layer reference"]
    pub fn self_attn(&self) -> &GroupedQueryAttention {
        &self.self_attn
    }

    #[inline]
    #[must_use = "returns the input layer normalization reference"]
    pub fn input_layernorm(&self) -> &RMSNorm {
        &self.input_layernorm
    }
}

/// Text Decoder (Llama-based)
///
/// Stack of decoder layers with:
/// - Token embeddings
/// - 30 decoder layers
/// - Final RMSNorm
///
/// Note: lm_head (vocabulary projection) is at Idefics3Model level, not here
pub struct TextDecoder {
    pub embed_tokens: nn::Embedding, // Public for vision-language generation
    layers: Vec<TextDecoderLayer>,
    norm: RMSNorm,
    rotary_emb: RotaryEmbedding,
}

impl std::fmt::Debug for TextDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextDecoder")
            .field("num_layers", &self.layers.len())
            .field("norm", &self.norm)
            .field("rotary_emb", &self.rotary_emb)
            .field("embed_tokens", &"<Embedding>")
            .finish()
    }
}

impl TextDecoder {
    pub fn new(vs: &nn::Path, config: &TextConfig) -> Self {
        let vocab_size = config.vocab_size as i64;
        let hidden_size = config.hidden_size as i64;
        let num_layers = config.num_hidden_layers;
        let head_dim = config.head_dim as i64;
        let max_position_embeddings = config.max_position_embeddings as i64;
        let rope_theta = config.rope_theta;
        let rms_norm_eps = config.rms_norm_eps;
        let tie_word_embeddings = config.tie_word_embeddings;

        // Token embeddings
        let embed_tokens = nn::embedding(
            vs / "embed_tokens",
            vocab_size,
            hidden_size,
            Default::default(),
        );

        // Decoder layers
        let layers = (0..num_layers)
            .map(|i| TextDecoderLayer::new(&(vs / "layers" / i.to_string()), config))
            .collect();

        // Final normalization
        let norm = RMSNorm::new(&(vs / "norm"), hidden_size, rms_norm_eps);

        // RoPE embeddings
        let rotary_emb = RotaryEmbedding::new(
            &(vs / "rotary_emb"),
            head_dim,
            max_position_embeddings,
            rope_theta,
        );

        Self {
            embed_tokens,
            layers,
            norm,
            rotary_emb,
        }
    }

    /// Forward pass: token IDs → hidden states
    ///
    /// Input: input_ids [batch, seq_len]
    /// Output: hidden_states [batch, seq_len, hidden_size]
    pub fn forward(
        &self,
        input_ids: &Tensor,
        attention_mask: Option<&Tensor>,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        let seq_len = input_ids.size()[1];

        // Embed tokens
        let hidden_states = self.embed_tokens.forward(input_ids);

        // Use forward_with_embeddings for the rest
        self.forward_with_embeddings(&hidden_states, attention_mask, train)
    }

    /// Forward pass: embeddings → hidden states
    ///
    /// This method accepts pre-computed embeddings instead of token IDs.
    /// Useful for vision-language models where image features replace text embeddings.
    ///
    /// Input: embeddings [batch, seq_len, hidden_size]
    /// Output: hidden_states [batch, seq_len, hidden_size]
    pub fn forward_with_embeddings(
        &self,
        embeddings: &Tensor,
        attention_mask: Option<&Tensor>,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        let seq_len = embeddings.size()[1];

        let mut hidden_states = embeddings.shallow_clone();

        // Compute RoPE embeddings for sequence length
        let (cos, sin) = self.rotary_emb.forward(seq_len, embeddings.device());

        // Pass through decoder layers
        for layer in &self.layers {
            hidden_states = layer.forward(&hidden_states, &cos, &sin, attention_mask, train)?;
        }

        // Final normalization
        hidden_states = self.norm.forward(&hidden_states);

        Ok(hidden_states)
    }

    /// Forward pass with layer-by-layer output extraction
    ///
    /// Same as forward_with_embeddings but returns hidden states after each layer
    /// for debugging purposes.
    ///
    /// Returns: Vec of (layer_idx, hidden_states) for each of the 30 layers + final norm
    pub fn forward_with_layer_outputs(
        &self,
        embeddings: &Tensor,
        attention_mask: Option<&Tensor>,
        train: bool,
    ) -> Result<Vec<(usize, Tensor)>, Box<dyn std::error::Error>> {
        let seq_len = embeddings.size()[1];
        let mut hidden_states = embeddings.shallow_clone();
        let mut layer_outputs = Vec::new();

        // Compute RoPE embeddings for sequence length
        let (cos, sin) = self.rotary_emb.forward(seq_len, embeddings.device());

        // Pass through decoder layers
        for (layer_idx, layer) in self.layers.iter().enumerate() {
            hidden_states = layer.forward(&hidden_states, &cos, &sin, attention_mask, train)?;
            layer_outputs.push((layer_idx, hidden_states.shallow_clone()));
        }

        // Final normalization
        hidden_states = self.norm.forward(&hidden_states);
        layer_outputs.push((99, hidden_states.shallow_clone())); // Use 99 to indicate final norm

        Ok(layer_outputs)
    }

    // Accessor methods for debugging
    #[inline]
    #[must_use = "returns the decoder layers reference"]
    pub fn layers(&self) -> &[TextDecoderLayer] {
        &self.layers
    }

    pub fn get_rope_embeddings(&self, position_ids: &Tensor, seq_len: usize) -> (Tensor, Tensor) {
        self.rotary_emb
            .forward(seq_len as i64, position_ids.device())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> TextConfig {
        TextConfig {
            vocab_size: 1000,
            hidden_size: 576,
            num_attention_heads: 9,
            num_key_value_heads: 3,
            head_dim: 64,
            num_hidden_layers: 2, // Only 2 layers for testing (not 30)
            intermediate_size: 1536,
            hidden_act: "silu".to_string(),
            max_position_embeddings: 512,
            rms_norm_eps: 1e-6,
            rope_theta: 10000.0,
            attention_bias: false,
            attention_dropout: 0.0,
            mlp_bias: false,
            tie_word_embeddings: false,
            bos_token_id: 1,
            eos_token_id: 2,
            initializer_range: 0.02,
            model_type: "llama".to_string(),
            torch_dtype: "float32".to_string(),
            use_cache: true,
            pretraining_tp: 1,
            rope_scaling: None,
            architectures: vec!["LlamaForCausalLM".to_string()],
        }
    }

    #[test]
    fn test_rms_norm() {
        let config = create_test_config();
        let vs = nn::VarStore::new(tch::Device::Cpu);
        let norm = RMSNorm::new(&vs.root(), config.hidden_size as i64, config.rms_norm_eps);

        let input = Tensor::randn([2, 10, 576], (tch::Kind::Float, tch::Device::Cpu));
        let output = norm.forward(&input);

        assert_eq!(output.size(), vec![2, 10, 576]);
    }

    #[test]
    fn test_rotary_embeddings() {
        let config = create_test_config();
        let vs = nn::VarStore::new(tch::Device::Cpu);
        let rope = RotaryEmbedding::new(
            &vs.root(),
            config.head_dim as i64,
            config.max_position_embeddings as i64,
            config.rope_theta,
        );

        let (cos, sin) = rope.forward(10, tch::Device::Cpu);

        assert_eq!(cos.size(), vec![1, 10, 64]);
        assert_eq!(sin.size(), vec![1, 10, 64]);
    }

    #[test]
    fn test_grouped_query_attention() {
        let config = create_test_config();
        let vs = nn::VarStore::new(tch::Device::Cpu);
        let attention = GroupedQueryAttention::new(&vs.root(), &config);

        let hidden_states = Tensor::randn([2, 10, 576], (tch::Kind::Float, tch::Device::Cpu));
        let rope = RotaryEmbedding::new(&vs.root(), 64, 512, 10000.0);
        let (cos, sin) = rope.forward(10, tch::Device::Cpu);

        let output = attention
            .forward(&hidden_states, &cos, &sin, None, false)
            .unwrap();

        assert_eq!(output.size(), vec![2, 10, 576]);
    }

    #[test]
    fn test_swiglu_mlp() {
        let config = create_test_config();
        let vs = nn::VarStore::new(tch::Device::Cpu);
        let mlp = SwiGLUMLP::new(&vs.root(), &config);

        let hidden_states = Tensor::randn([2, 10, 576], (tch::Kind::Float, tch::Device::Cpu));
        let output = mlp.forward(&hidden_states);

        assert_eq!(output.size(), vec![2, 10, 576]);
    }

    #[test]
    fn test_text_decoder_layer() {
        let config = create_test_config();
        let vs = nn::VarStore::new(tch::Device::Cpu);
        let layer = TextDecoderLayer::new(&vs.root(), &config);

        let hidden_states = Tensor::randn([2, 10, 576], (tch::Kind::Float, tch::Device::Cpu));
        let rope = RotaryEmbedding::new(&vs.root(), 64, 512, 10000.0);
        let (cos, sin) = rope.forward(10, tch::Device::Cpu);

        let output = layer
            .forward(&hidden_states, &cos, &sin, None, false)
            .unwrap();

        assert_eq!(output.size(), vec![2, 10, 576]);
    }

    #[test]
    fn test_text_decoder_end_to_end() {
        let config = create_test_config();
        let vs = nn::VarStore::new(tch::Device::Cpu);
        let decoder = TextDecoder::new(&vs.root(), &config);

        // Input: token IDs [batch=2, seq_len=10]
        let input_ids = Tensor::randint(1000, [2, 10], (tch::Kind::Int64, tch::Device::Cpu));
        let hidden_states = decoder.forward(&input_ids, None, false).unwrap();

        // Output: hidden_states [batch=2, seq_len=10, hidden_size=576]
        assert_eq!(hidden_states.size(), vec![2, 10, 576]);
    }
}
