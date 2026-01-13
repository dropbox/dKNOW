// Connector for Idefics3 - projects vision embeddings to text space
// Based on HuggingFace transformers/models/idefics3/modeling_idefics3.py

use crate::models::code_formula::config::Idefics3Config;
use tch::{nn, Tensor};

/// Simple MLP projection from vision space to text space
///
/// Projects vision embeddings after pixel shuffle:
/// input_size = vision_hidden * (scale_factor^2)
/// output_size = text_hidden
///
/// For CodeFormula:
/// - vision_hidden = 768
/// - text_hidden = 576
/// - scale_factor = 4
/// - input_size = 768 * 16 = 12288
/// - output_size = 576
pub struct SimpleMLP {
    proj: nn::Linear,
}

impl std::fmt::Debug for SimpleMLP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleMLP")
            .field("proj", &"<Linear>")
            .finish()
    }
}

impl SimpleMLP {
    pub fn new(vs: &nn::Path, config: &Idefics3Config) -> Self {
        let input_size = (config.vision_hidden_size() * config.scale_factor.pow(2)) as i64;
        let output_size = config.text_hidden_size() as i64;

        let proj = nn::linear(
            vs / "proj",
            input_size,
            output_size,
            nn::LinearConfig {
                bias: false,
                ..Default::default()
            },
        );

        Self { proj }
    }

    /// Project vision embeddings to text space
    ///
    /// Input: [batch, seq_len, input_size] where input_size = vision_hidden * (scale_factor^2)
    /// Output: [batch, seq_len, text_hidden]
    pub fn forward(&self, x: &Tensor) -> Tensor {
        x.apply(&self.proj)
    }
}

/// Connector that bridges vision and text modalities
///
/// Two steps:
/// 1. Pixel Shuffle: Reduces spatial resolution by grouping patches
///    - Input: [batch, num_patches, vision_hidden] (e.g., [1, 1024, 768])
///    - Output: [batch, num_patches / scale_factor^2, vision_hidden * scale_factor^2] (e.g., [1, 64, 12288])
///
/// 2. MLP Projection: Projects to text hidden dimension
///    - Input: [batch, seq_len, vision_hidden * scale_factor^2] (e.g., [1, 64, 12288])
///    - Output: [batch, seq_len, text_hidden] (e.g., [1, 64, 576])
///
/// The pixel shuffle operation groups neighboring patches into single tokens,
/// reducing the sequence length while increasing the embedding dimension.
pub struct Idefics3Connector {
    scale_factor: i64,
    modality_projection: SimpleMLP,
}

impl std::fmt::Debug for Idefics3Connector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Idefics3Connector")
            .field("scale_factor", &self.scale_factor)
            .field("modality_projection", &self.modality_projection)
            .finish()
    }
}

impl Idefics3Connector {
    pub fn new(vs: &nn::Path, config: &Idefics3Config) -> Self {
        let scale_factor = config.scale_factor as i64;
        let modality_projection = SimpleMLP::new(&(vs / "modality_projection"), config);

        Self {
            scale_factor,
            modality_projection,
        }
    }

    /// Pixel shuffle operation to reduce spatial resolution
    ///
    /// This operation groups patches in a spatial grid, reducing the number of tokens
    /// while increasing the embedding dimension.
    ///
    /// Example with scale_factor=4:
    /// - Input: [1, 1024, 768] (32x32 patches, 768 dim)
    /// - After pixel shuffle: [1, 64, 12288] (8x8 patches, 768*16 dim)
    ///
    /// Algorithm:
    /// 1. Reshape to spatial grid: [batch, height, width, embed_dim]
    /// 2. Group horizontally: [batch, height, width/scale, embed_dim * scale]
    /// 3. Transpose and group vertically: [batch, width/scale, height/scale, embed_dim * scale^2]
    /// 4. Reshape to sequence: [batch, seq_len / scale^2, embed_dim * scale^2]
    pub fn pixel_shuffle(&self, x: &Tensor) -> Tensor {
        let sizes = x.size();
        let (batch_size, seq_len, embed_dim) = (sizes[0], sizes[1], sizes[2]);

        // Compute spatial dimensions (assume square image)
        let height = (seq_len as f64).sqrt() as i64;
        let width = height; // Square assumption

        // Verify square assumption
        assert_eq!(
            height * width,
            seq_len,
            "pixel_shuffle requires square spatial dimensions, got seq_len={} ({}x{} != {})",
            seq_len,
            height,
            width,
            seq_len
        );

        // Step 1: Reshape to spatial grid [batch, height, width, embed_dim]
        let x = x.view([batch_size, height, width, embed_dim]);

        // Step 2: Group horizontally [batch, height, width/scale, embed_dim * scale]
        let x = x.view([
            batch_size,
            height,
            width / self.scale_factor,
            embed_dim * self.scale_factor,
        ]);

        // Step 3: Transpose [batch, width/scale, height, embed_dim * scale]
        let x = x.permute([0, 2, 1, 3]);

        // Step 4: Group vertically [batch, width/scale, height/scale, embed_dim * scale^2]
        let x = x.reshape([
            batch_size,
            width / self.scale_factor,
            height / self.scale_factor,
            embed_dim * self.scale_factor.pow(2),
        ]);

        // Step 5: Transpose back [batch, height/scale, width/scale, embed_dim * scale^2]
        let x = x.permute([0, 2, 1, 3]);

        // Step 6: Flatten to sequence [batch, (height/scale) * (width/scale), embed_dim * scale^2]
        let new_seq_len = seq_len / self.scale_factor.pow(2);
        let new_embed_dim = embed_dim * self.scale_factor.pow(2);
        x.reshape([batch_size, new_seq_len, new_embed_dim])
    }

    /// Forward pass: pixel shuffle + MLP projection
    ///
    /// Input: vision embeddings [batch, num_patches, vision_hidden]
    /// Output: text-compatible embeddings [batch, num_patches / scale^2, text_hidden]
    pub fn forward(&self, image_hidden_states: &Tensor) -> Tensor {
        let shuffled = self.pixel_shuffle(image_hidden_states);
        self.modality_projection.forward(&shuffled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tch::Device;

    fn get_test_config() -> Idefics3Config {
        let model_dir = std::path::Path::new(env!("HOME"))
            .join(".cache/huggingface/hub/models--ds4sd--CodeFormulaV2/snapshots/ecedbe111d15c2dc60bfd4a823cbe80127b58af4");

        if !model_dir.exists() {
            panic!("Model directory not found: {:?}", model_dir);
        }

        Idefics3Config::from_pretrained(&model_dir).expect("Failed to load config")
    }

    #[test]
    fn test_pixel_shuffle() {
        let config = get_test_config();
        let device = Device::Cpu;

        // Create dummy connector (just for pixel_shuffle, don't need weights)
        let vs = nn::VarStore::new(device);
        let connector = Idefics3Connector::new(&vs.root(), &config);

        // Test with 32x32 patches (1024 patches)
        // scale_factor = 4 → should produce 8x8 = 64 patches
        let batch_size = 1;
        let num_patches = 1024i64; // 32x32
        let vision_hidden = 768i64;
        let input = Tensor::randn(
            [batch_size, num_patches, vision_hidden],
            (tch::Kind::Float, device),
        );

        let output = connector.pixel_shuffle(&input);

        // Expected: 64 patches, 12288 dim (768 * 16)
        let expected_patches = num_patches / (config.scale_factor.pow(2) as i64);
        let expected_dim = vision_hidden * (config.scale_factor.pow(2) as i64);

        assert_eq!(
            output.size(),
            vec![batch_size, expected_patches, expected_dim]
        );
        assert_eq!(output.size(), vec![1, 64, 12288]);
    }

    #[test]
    fn test_simple_mlp_shapes() {
        let config = get_test_config();
        let device = Device::Cpu;

        let vs = nn::VarStore::new(device);
        let mlp = SimpleMLP::new(&vs.root(), &config);

        // Input: [batch, seq_len, vision_hidden * scale^2]
        let batch_size = 1;
        let seq_len = 64; // After pixel shuffle with scale_factor=4
        let input_size = (config.vision_hidden_size() * config.scale_factor.pow(2)) as i64;
        let input = Tensor::randn(
            [batch_size, seq_len, input_size],
            (tch::Kind::Float, device),
        );

        let output = mlp.forward(&input);

        // Expected: [batch, seq_len, text_hidden]
        let expected_output_dim = config.text_hidden_size() as i64;
        assert_eq!(
            output.size(),
            vec![batch_size, seq_len, expected_output_dim]
        );
        assert_eq!(output.size(), vec![1, 64, 576]);
    }

    #[test]
    fn test_connector_end_to_end() {
        let config = get_test_config();
        let device = Device::Cpu;

        let vs = nn::VarStore::new(device);
        let connector = Idefics3Connector::new(&vs.root(), &config);

        // Input: vision embeddings from vision encoder
        // [1, 1024, 768] (32x32 patches, 768 dim)
        let batch_size = 1;
        let num_patches = 1024i64; // 32x32
        let vision_hidden = config.vision_hidden_size() as i64;
        let input = Tensor::randn(
            [batch_size, num_patches, vision_hidden],
            (tch::Kind::Float, device),
        );

        let output = connector.forward(&input);

        // Expected: [1, 64, 576]
        // - 64 patches (8x8 after pixel shuffle with scale_factor=4)
        // - 576 dim (text hidden)
        let expected_patches = num_patches / (config.scale_factor.pow(2) as i64);
        let expected_dim = config.text_hidden_size() as i64;

        assert_eq!(
            output.size(),
            vec![batch_size, expected_patches, expected_dim]
        );
        assert_eq!(output.size(), vec![1, 64, 576]);
    }
}
