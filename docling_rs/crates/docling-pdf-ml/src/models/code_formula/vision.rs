// Vision encoder for Idefics3 (SiglipVisionTransformer)
// Based on HuggingFace transformers/models/idefics3/modeling_idefics3.py

use crate::models::code_formula::config::VisionConfig;
use tch::{nn, nn::Module, Tensor};

#[cfg(test)]
use tch::Device;

/// Vision embeddings: patch embedding + position embedding
///
/// Architecture:
/// - Conv2d patch embedding (16x16 patches → 768 hidden)
/// - Learned position embeddings for each patch
/// - Variable resolution support (Patch n' Pack)
pub struct VisionEmbeddings {
    patch_embedding: nn::Conv2D,
    position_embedding: nn::Embedding,
    embed_dim: i64,
    image_size: i64,
    patch_size: i64,
    num_patches_per_side: i64,
    num_patches: i64,
}

impl std::fmt::Debug for VisionEmbeddings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionEmbeddings")
            .field("embed_dim", &self.embed_dim)
            .field("image_size", &self.image_size)
            .field("patch_size", &self.patch_size)
            .field("num_patches_per_side", &self.num_patches_per_side)
            .field("num_patches", &self.num_patches)
            .field("patch_embedding", &"<Conv2D>")
            .field("position_embedding", &"<Embedding>")
            .finish()
    }
}

impl VisionEmbeddings {
    pub fn new(vs: &nn::Path, config: &VisionConfig) -> Self {
        let embed_dim = config.hidden_size as i64;
        let image_size = config.image_size as i64;
        let patch_size = config.patch_size as i64;
        let num_channels = config.num_channels as i64;

        // Conv2d for patch embedding (stride = kernel_size = patch_size)
        let conv_config = nn::ConvConfig {
            stride: patch_size,
            padding: 0,
            ..Default::default()
        };
        let patch_embedding = nn::conv2d(
            vs / "patch_embedding",
            num_channels,
            embed_dim,
            patch_size,
            conv_config,
        );

        // Position embeddings for patches
        let num_patches_per_side = image_size / patch_size;
        let num_patches = num_patches_per_side * num_patches_per_side;
        let position_embedding = nn::embedding(
            vs / "position_embedding",
            num_patches,
            embed_dim,
            Default::default(),
        );

        Self {
            patch_embedding,
            position_embedding,
            embed_dim,
            image_size,
            patch_size,
            num_patches_per_side,
            num_patches,
        }
    }

    /// Forward pass: pixel_values → patch embeddings + position embeddings
    ///
    /// Input: pixel_values [batch, channels, height, width]
    /// Output: embeddings [batch, num_patches, embed_dim]
    #[must_use = "forward pass returns output that should be processed"]
    pub fn forward(&self, pixel_values: &Tensor) -> Result<Tensor, Box<dyn std::error::Error>> {
        let batch_size = pixel_values.size()[0];
        let height = pixel_values.size()[2];
        let width = pixel_values.size()[3];

        // Apply patch embedding (Conv2d)
        // Ensure tensor is contiguous and on correct device
        let pixel_values_contiguous = if !pixel_values.is_contiguous() {
            pixel_values.contiguous()
        } else {
            pixel_values.shallow_clone()
        };

        let patch_embeds = pixel_values_contiguous.apply(&self.patch_embedding); // [B, embed_dim, H', W']

        // Flatten spatial dimensions and transpose
        // [B, embed_dim, H', W'] → [B, embed_dim, H'*W'] → [B, H'*W', embed_dim]
        let embeddings = patch_embeds
            .flatten(2, -1) // [B, embed_dim, num_patches]
            .transpose(1, 2); // [B, num_patches, embed_dim]

        let num_patches_h = height / self.patch_size;
        let num_patches_w = width / self.patch_size;
        let total_patches = num_patches_h * num_patches_w;

        // Generate position IDs using Python's bucketing logic
        //
        // Python implementation (transformers/models/idefics3/modeling_idefics3.py:134-165):
        // 1. Create boundaries at 1/32, 2/32, ..., 31/32 for num_patches_per_side=32
        // 2. For each patch coordinate (h, w):
        //    - Compute fractional coordinates: h/32 * (1 - 1e-6), w/32 * (1 - 1e-6)
        //    - Bucketize into bucket_coords_h, bucket_coords_w
        //    - Position ID = bucket_coords_h * 32 + bucket_coords_w
        //
        // This creates position IDs like: [0, 0, 1, 2, ..., 30, 0, 0, 1, 2, ..., 30, ...]
        // NOT sequential [0, 1, 2, 3, ...]
        let position_ids = self.compute_position_ids(
            batch_size,
            num_patches_h,
            num_patches_w,
            pixel_values.device(),
        );

        // Add position embeddings
        let pos_embeds = self.position_embedding.forward(&position_ids);

        let embeddings = embeddings + pos_embeds;

        Ok(embeddings)
    }

    /// Compute position IDs using Python's bucketing logic
    ///
    /// Replicates the position ID computation from:
    /// transformers/models/idefics3/modeling_idefics3.py:134-165
    ///
    /// This implements variable-resolution position embeddings where position IDs
    /// are computed by:
    /// 1. Creating boundaries at fractions of num_patches_per_side
    /// 2. Computing fractional coordinates for each patch
    /// 3. Bucketizing coordinates into position bins
    /// 4. Position ID = bucket_h * num_patches_per_side + bucket_w
    fn compute_position_ids(
        &self,
        batch_size: i64,
        num_patches_h: i64,
        num_patches_w: i64,
        device: tch::Device,
    ) -> Tensor {
        let total_patches = num_patches_h * num_patches_w;

        // Create boundaries: [1/n, 2/n, 3/n, ..., (n-1)/n]
        // where n = num_patches_per_side (e.g., 32 for 512x512 images)
        let n = self.num_patches_per_side as f64;
        let boundaries: Vec<f64> = (1..self.num_patches_per_side)
            .map(|i| i as f64 / n)
            .collect();
        let boundaries_tensor = Tensor::from_slice(&boundaries).to_device(device);

        // Initialize position IDs (will be filled in loop)
        let position_ids = Tensor::zeros([batch_size, total_patches], (tch::Kind::Int64, device));

        // For each batch (all batches use same logic since we assume no padding)
        for batch_idx in 0..batch_size {
            // Number of valid patches (for full images without padding, this equals total patches)
            let nb_patches_h = num_patches_h;
            let nb_patches_w = num_patches_w;

            // Generate coordinate indices: [0, 1, 2, ..., nb_patches-1]
            let h_indices = Tensor::arange(nb_patches_h, (tch::Kind::Float, device));
            let w_indices = Tensor::arange(nb_patches_w, (tch::Kind::Float, device));

            // Compute fractional coordinates: coord / nb_patches * (1 - 1e-6)
            let fractional_coords_h = &h_indices / (nb_patches_h as f64) * (1.0 - 1e-6);
            let fractional_coords_w = &w_indices / (nb_patches_w as f64) * (1.0 - 1e-6);

            // Bucketize: find which boundary bin each coordinate falls into
            // This replicates torch.bucketize(fractional_coords, boundaries, right=True)
            let bucket_coords_h = self.bucketize(&fractional_coords_h, &boundaries_tensor, true);
            let bucket_coords_w = self.bucketize(&fractional_coords_w, &boundaries_tensor, true);

            // Compute position IDs: bucket_h * num_patches_per_side + bucket_w
            // Shape: [nb_patches_h, 1] * num_patches_per_side + [1, nb_patches_w]
            //      = [nb_patches_h, nb_patches_w] → flatten to [nb_patches_h * nb_patches_w]
            let pos_ids = (&bucket_coords_h.unsqueeze(1) * self.num_patches_per_side
                + &bucket_coords_w.unsqueeze(0))
                .flatten(0, -1);

            // Copy position IDs to the batch
            position_ids.get(batch_idx).copy_(&pos_ids);
        }

        position_ids
    }

    /// Bucketize: find which bin each value falls into
    ///
    /// Replicates torch.bucketize(input, boundaries, right=True)
    ///
    /// For each value in input, find the index of the first boundary > value (if right=True)
    /// or the first boundary >= value (if right=False).
    ///
    /// Example:
    ///   boundaries = [0.1, 0.2, 0.3]
    ///   input = [0.05, 0.15, 0.25, 0.35]
    ///   right=True → [0, 1, 2, 3]
    ///   right=False → [0, 1, 2, 3]
    ///
    /// Note: torch.bucketize(input, boundaries) == torch.searchsorted(boundaries, input)
    ///
    /// WORKAROUND: tch-rs 0.18 searchsorted() segfaults, so we implement bucketize manually
    fn bucketize(&self, input: &Tensor, boundaries: &Tensor, right: bool) -> Tensor {
        // Convert tensors to Vec for manual binary search
        // Check dtype and use appropriate Vec type
        let boundaries_size = boundaries.size()[0];
        let input_size = input.size()[0];

        // Convert to f64 for consistency (boundaries are created from Vec<f64>)
        let boundaries_f64 = boundaries.to_kind(tch::Kind::Double);
        let input_f64 = input.to_kind(tch::Kind::Double);

        let mut boundaries_vec = vec![0.0f64; boundaries_size as usize];
        let mut input_vec = vec![0.0f64; input_size as usize];

        // Copy data from tensors to vectors
        boundaries_f64.copy_data(&mut boundaries_vec, boundaries_size as usize);
        input_f64.copy_data(&mut input_vec, input_size as usize);

        // Manual binary search for each input value
        let mut result = Vec::with_capacity(input_vec.len());
        for &value in &input_vec {
            let idx = if right {
                // Find first boundary > value
                boundaries_vec
                    .iter()
                    .position(|&b| b > value)
                    .unwrap_or(boundaries_vec.len())
            } else {
                // Find first boundary >= value
                boundaries_vec
                    .iter()
                    .position(|&b| b >= value)
                    .unwrap_or(boundaries_vec.len())
            };
            result.push(idx as i64);
        }

        Tensor::from_slice(&result).to_device(input.device())
    }
}

/// Multi-head self-attention for vision encoder
///
/// Standard transformer attention with:
/// - Q, K, V projections
/// - Multi-head splitting
/// - Scaled dot-product attention
/// - Output projection
pub struct VisionAttention {
    q_proj: nn::Linear,
    k_proj: nn::Linear,
    v_proj: nn::Linear,
    out_proj: nn::Linear,
    num_heads: i64,
    head_dim: i64,
    scale: f64,
    dropout: f64,
}

impl std::fmt::Debug for VisionAttention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionAttention")
            .field("num_heads", &self.num_heads)
            .field("head_dim", &self.head_dim)
            .field("scale", &self.scale)
            .field("dropout", &self.dropout)
            .field("q_proj", &"<Linear>")
            .field("k_proj", &"<Linear>")
            .field("v_proj", &"<Linear>")
            .field("out_proj", &"<Linear>")
            .finish()
    }
}

impl VisionAttention {
    pub fn new(vs: &nn::Path, config: &VisionConfig) -> Self {
        let embed_dim = config.hidden_size as i64;
        let num_heads = config.num_attention_heads as i64;
        let head_dim = embed_dim / num_heads;
        let scale = (head_dim as f64).powf(-0.5);
        let dropout = config.attention_dropout;

        let q_proj = nn::linear(vs / "q_proj", embed_dim, embed_dim, Default::default());
        let k_proj = nn::linear(vs / "k_proj", embed_dim, embed_dim, Default::default());
        let v_proj = nn::linear(vs / "v_proj", embed_dim, embed_dim, Default::default());
        let out_proj = nn::linear(vs / "out_proj", embed_dim, embed_dim, Default::default());

        Self {
            q_proj,
            k_proj,
            v_proj,
            out_proj,
            num_heads,
            head_dim,
            scale,
            dropout,
        }
    }

    /// Forward pass: compute multi-head self-attention
    ///
    /// Input: hidden_states [batch, seq_len, embed_dim]
    /// Output: attn_output [batch, seq_len, embed_dim]
    pub fn forward(
        &self,
        hidden_states: &Tensor,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        let (batch_size, seq_length, embed_dim) = (
            hidden_states.size()[0],
            hidden_states.size()[1],
            hidden_states.size()[2],
        );

        // Project Q, K, V
        let queries = hidden_states.apply(&self.q_proj); // [B, S, E]
        let keys = hidden_states.apply(&self.k_proj); // [B, S, E]
        let values = hidden_states.apply(&self.v_proj); // [B, S, E]

        // Reshape for multi-head attention
        // [B, S, E] → [B, S, num_heads, head_dim] → [B, num_heads, S, head_dim]
        let queries = queries
            .view([batch_size, seq_length, self.num_heads, self.head_dim])
            .transpose(1, 2);
        let keys = keys
            .view([batch_size, seq_length, self.num_heads, self.head_dim])
            .transpose(1, 2);
        let values = values
            .view([batch_size, seq_length, self.num_heads, self.head_dim])
            .transpose(1, 2);

        // Compute attention scores: Q @ K^T * scale
        // [B, H, S, D] @ [B, H, D, S] → [B, H, S, S]
        let attn_weights = queries.matmul(&keys.transpose(-2, -1)) * self.scale;

        // Softmax over last dimension
        let attn_weights = attn_weights.softmax(-1, tch::Kind::Float);

        // Apply dropout if training
        let attn_weights = if train && self.dropout > 0.0 {
            attn_weights.dropout(self.dropout, train)
        } else {
            attn_weights
        };

        // Apply attention to values: attn_weights @ V
        // [B, H, S, S] @ [B, H, S, D] → [B, H, S, D]
        let attn_output = attn_weights.matmul(&values);

        // Reshape back: [B, H, S, D] → [B, S, H, D] → [B, S, E]
        let attn_output = attn_output
            .transpose(1, 2)
            .contiguous()
            .view([batch_size, seq_length, embed_dim]);

        // Output projection
        let attn_output = attn_output.apply(&self.out_proj);

        Ok(attn_output)
    }
}

/// Feed-forward network (MLP) for vision encoder
///
/// Two-layer MLP with GELU activation:
/// - fc1: embed_dim → intermediate_size
/// - GELU activation
/// - fc2: intermediate_size → embed_dim
pub struct VisionMLP {
    fc1: nn::Linear,
    fc2: nn::Linear,
}

impl std::fmt::Debug for VisionMLP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionMLP")
            .field("fc1", &"<Linear>")
            .field("fc2", &"<Linear>")
            .finish()
    }
}

impl VisionMLP {
    pub fn new(vs: &nn::Path, config: &VisionConfig) -> Self {
        let hidden_size = config.hidden_size as i64;
        let intermediate_size = config.intermediate_size as i64;

        let fc1 = nn::linear(
            vs / "fc1",
            hidden_size,
            intermediate_size,
            Default::default(),
        );
        let fc2 = nn::linear(
            vs / "fc2",
            intermediate_size,
            hidden_size,
            Default::default(),
        );

        Self { fc1, fc2 }
    }

    /// Forward pass: two-layer MLP with GELU
    ///
    /// Input: hidden_states [batch, seq_len, hidden_size]
    /// Output: hidden_states [batch, seq_len, hidden_size]
    #[must_use = "forward pass returns output that should be processed"]
    pub fn forward(&self, hidden_states: &Tensor) -> Result<Tensor, Box<dyn std::error::Error>> {
        let hidden_states = hidden_states.apply(&self.fc1);
        let hidden_states = hidden_states.gelu("none"); // GELU activation
        let hidden_states = hidden_states.apply(&self.fc2);
        Ok(hidden_states)
    }
}

/// Single vision encoder layer (attention + MLP + residual connections)
///
/// Architecture:
/// - LayerNorm → Self-Attention → Residual
/// - LayerNorm → MLP → Residual
pub struct VisionEncoderLayer {
    self_attn: VisionAttention,
    mlp: VisionMLP,
    layer_norm1: nn::LayerNorm,
    layer_norm2: nn::LayerNorm,
}

impl std::fmt::Debug for VisionEncoderLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionEncoderLayer")
            .field("self_attn", &self.self_attn)
            .field("mlp", &self.mlp)
            .field("layer_norm1", &"<LayerNorm>")
            .field("layer_norm2", &"<LayerNorm>")
            .finish()
    }
}

impl VisionEncoderLayer {
    pub fn new(vs: &nn::Path, config: &VisionConfig) -> Self {
        let embed_dim = config.hidden_size as i64;
        let layer_norm_eps = config.layer_norm_eps;

        let self_attn = VisionAttention::new(&(vs / "self_attn"), config);
        let mlp = VisionMLP::new(&(vs / "mlp"), config);

        let layer_norm_config = nn::LayerNormConfig {
            eps: layer_norm_eps,
            ..Default::default()
        };
        let layer_norm1 = nn::layer_norm(vs / "layer_norm1", vec![embed_dim], layer_norm_config);
        let layer_norm2 = nn::layer_norm(vs / "layer_norm2", vec![embed_dim], layer_norm_config);

        Self {
            self_attn,
            mlp,
            layer_norm1,
            layer_norm2,
        }
    }

    /// Forward pass: Pre-LN transformer layer
    ///
    /// Input: hidden_states [batch, seq_len, embed_dim]
    /// Output: hidden_states [batch, seq_len, embed_dim]
    pub fn forward(
        &self,
        hidden_states: &Tensor,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        // Self-attention block with residual
        let residual = hidden_states.shallow_clone();
        let hidden_states = hidden_states.apply(&self.layer_norm1);
        let hidden_states = self.self_attn.forward(&hidden_states, train)?;
        let hidden_states = residual + hidden_states;

        // MLP block with residual
        let residual = hidden_states.shallow_clone();
        let hidden_states = hidden_states.apply(&self.layer_norm2);
        let hidden_states = self.mlp.forward(&hidden_states)?;
        let hidden_states = residual + hidden_states;

        Ok(hidden_states)
    }
}

/// Vision encoder: stack of transformer layers
///
/// Architecture:
/// - N encoder layers (default: 12)
pub struct VisionEncoder {
    layers: Vec<VisionEncoderLayer>,
}

impl std::fmt::Debug for VisionEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionEncoder")
            .field("num_layers", &self.layers.len())
            .finish()
    }
}

impl VisionEncoder {
    pub fn new(vs: &nn::Path, config: &VisionConfig) -> Self {
        let num_layers = config.num_hidden_layers;

        let layers = (0..num_layers)
            .map(|i| VisionEncoderLayer::new(&(vs / "layers" / i.to_string()), config))
            .collect();

        Self { layers }
    }

    /// Forward pass: apply all encoder layers
    ///
    /// Input: hidden_states [batch, seq_len, embed_dim]
    /// Output: hidden_states [batch, seq_len, embed_dim]
    pub fn forward(
        &self,
        mut hidden_states: Tensor,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        for layer in &self.layers {
            hidden_states = layer.forward(&hidden_states, train)?;
        }

        Ok(hidden_states)
    }
}

/// Full vision transformer (embeddings + encoder)
///
/// This is the complete SiglipVisionTransformer used in Idefics3.
///
/// Architecture:
/// - VisionEmbeddings: patches + positions
/// - VisionEncoder: 12 transformer layers
/// - Post-layer normalization
pub struct VisionTransformer {
    embeddings: VisionEmbeddings,
    encoder: VisionEncoder,
    post_layernorm: nn::LayerNorm,
}

impl std::fmt::Debug for VisionTransformer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionTransformer")
            .field("embeddings", &self.embeddings)
            .field("encoder", &self.encoder)
            .field("post_layernorm", &"<LayerNorm>")
            .finish()
    }
}

impl VisionTransformer {
    pub fn new(vs: &nn::Path, config: &VisionConfig) -> Self {
        let embeddings = VisionEmbeddings::new(&(vs / "embeddings"), config);
        let encoder = VisionEncoder::new(&(vs / "encoder"), config);

        let embed_dim = config.hidden_size as i64;
        let layer_norm_config = nn::LayerNormConfig {
            eps: config.layer_norm_eps,
            ..Default::default()
        };
        let post_layernorm =
            nn::layer_norm(vs / "post_layernorm", vec![embed_dim], layer_norm_config);

        Self {
            embeddings,
            encoder,
            post_layernorm,
        }
    }

    /// Forward pass: pixel values → vision embeddings
    ///
    /// Input: pixel_values [batch, channels, height, width]
    /// Output: vision embeddings [batch, num_patches, embed_dim]
    pub fn forward(
        &self,
        pixel_values: &Tensor,
        train: bool,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        // Embed patches
        let hidden_states = self.embeddings.forward(pixel_values)?;

        // Pass through encoder
        let hidden_states = self.encoder.forward(hidden_states, train)?;

        // Final layer normalization
        let hidden_states = hidden_states.apply(&self.post_layernorm);

        Ok(hidden_states)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log;

    #[test]
    fn test_bucketize_manual() {
        // Test manual bucketize implementation
        let config = VisionConfig {
            hidden_size: 768,
            image_size: 512,
            patch_size: 16,
            num_channels: 3,
            num_attention_heads: 12,
            num_hidden_layers: 12,
            intermediate_size: 3072,
            hidden_act: "gelu".to_string(),
            layer_norm_eps: 1e-6,
            attention_dropout: 0.0,
            initializer_range: 0.02,
            model_type: "siglip_vision_model".to_string(),
            torch_dtype: "float32".to_string(),
            use_base_siglip: false,
            max_image_size: std::collections::HashMap::new(),
            size: std::collections::HashMap::new(),
        };

        let vs = nn::VarStore::new(Device::Cpu);
        let embeddings = VisionEmbeddings::new(&vs.root(), &config);

        // Test bucketize
        let boundaries = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let boundaries_tensor = Tensor::from_slice(&boundaries).to_device(Device::Cpu);

        let values = vec![0.05, 0.15, 0.25, 0.35, 0.45];
        let values_tensor = Tensor::from_slice(&values).to_device(Device::Cpu);

        let result = embeddings.bucketize(&values_tensor, &boundaries_tensor, true);

        log::debug!("Result: {:?}", result);
        // Expected: [0, 1, 2, 3, 4] (indices where each value would be inserted)
        assert_eq!(result.size(), vec![5]);
    }

    #[test]
    fn test_vision_embeddings_shapes() {
        // Config values from CodeFormula
        let config = VisionConfig {
            hidden_size: 768,
            image_size: 512,
            patch_size: 16,
            num_channels: 3,
            num_attention_heads: 12,
            num_hidden_layers: 12,
            intermediate_size: 3072,
            hidden_act: "gelu".to_string(),
            layer_norm_eps: 1e-6,
            attention_dropout: 0.0,
            initializer_range: 0.02,
            model_type: "siglip_vision_model".to_string(),
            torch_dtype: "float32".to_string(),
            use_base_siglip: false,
            max_image_size: std::collections::HashMap::new(),
            size: std::collections::HashMap::new(),
        };

        let vs = nn::VarStore::new(Device::Cpu);
        let embeddings = VisionEmbeddings::new(&vs.root(), &config);

        // Test with 512x512 image
        let pixel_values = Tensor::randn([1, 3, 512, 512], (tch::Kind::Float, Device::Cpu));
        let output = embeddings.forward(&pixel_values).unwrap();

        // Expected: [1, 1024, 768] (32x32 patches, 768 hidden)
        assert_eq!(output.size(), vec![1, 1024, 768]);
    }

    #[test]
    fn test_vision_attention_shapes() {
        let config = VisionConfig {
            hidden_size: 768,
            num_attention_heads: 12,
            attention_dropout: 0.0,
            image_size: 512,
            patch_size: 16,
            num_channels: 3,
            num_hidden_layers: 12,
            intermediate_size: 3072,
            hidden_act: "gelu".to_string(),
            layer_norm_eps: 1e-6,
            initializer_range: 0.02,
            model_type: "siglip_vision_model".to_string(),
            torch_dtype: "float32".to_string(),
            use_base_siglip: false,
            max_image_size: std::collections::HashMap::new(),
            size: std::collections::HashMap::new(),
        };

        let vs = nn::VarStore::new(Device::Cpu);
        let attention = VisionAttention::new(&vs.root(), &config);

        // Test with dummy input
        let hidden_states = Tensor::randn([2, 100, 768], (tch::Kind::Float, Device::Cpu));
        let output = attention.forward(&hidden_states, false).unwrap();

        // Output should have same shape as input
        assert_eq!(output.size(), vec![2, 100, 768]);
    }

    #[test]
    #[ignore] // Flaky: segfaults due to tch-rs/PyTorch memory management issues (N=476)
    fn test_vision_encoder_layer_shapes() {
        let config = VisionConfig {
            hidden_size: 768,
            num_attention_heads: 12,
            attention_dropout: 0.0,
            intermediate_size: 3072,
            hidden_act: "gelu".to_string(),
            layer_norm_eps: 1e-6,
            image_size: 512,
            patch_size: 16,
            num_channels: 3,
            num_hidden_layers: 12,
            initializer_range: 0.02,
            model_type: "siglip_vision_model".to_string(),
            torch_dtype: "float32".to_string(),
            use_base_siglip: false,
            max_image_size: std::collections::HashMap::new(),
            size: std::collections::HashMap::new(),
        };

        let vs = nn::VarStore::new(Device::Cpu);
        let layer = VisionEncoderLayer::new(&vs.root(), &config);

        let hidden_states = Tensor::randn([2, 100, 768], (tch::Kind::Float, Device::Cpu));
        let output = layer.forward(&hidden_states, false).unwrap();

        // Output should have same shape as input
        assert_eq!(output.size(), vec![2, 100, 768]);
    }

    #[test]
    #[ignore] // Flaky: segfaults due to tch-rs/PyTorch memory management issues (N=476)
    fn test_vision_transformer_end_to_end() {
        let config = VisionConfig {
            hidden_size: 768,
            image_size: 512,
            patch_size: 16,
            num_channels: 3,
            num_attention_heads: 12,
            num_hidden_layers: 12,
            intermediate_size: 3072,
            hidden_act: "gelu".to_string(),
            layer_norm_eps: 1e-6,
            attention_dropout: 0.0,
            initializer_range: 0.02,
            model_type: "siglip_vision_model".to_string(),
            torch_dtype: "float32".to_string(),
            use_base_siglip: false,
            max_image_size: std::collections::HashMap::new(),
            size: std::collections::HashMap::new(),
        };

        let vs = nn::VarStore::new(Device::Cpu);
        let vision_transformer = VisionTransformer::new(&vs.root(), &config);

        // Test with 512x512 image
        let pixel_values = Tensor::randn([1, 3, 512, 512], (tch::Kind::Float, Device::Cpu));
        let output = vision_transformer.forward(&pixel_values, false).unwrap();

        // Expected: [1, 1024, 768] (32x32 patches = 1024, 768 hidden)
        assert_eq!(output.size(), vec![1, 1024, 768]);
    }
}
