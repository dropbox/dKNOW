// Configuration structures for Idefics3 (CodeFormula) model
// Based on HuggingFace transformers Idefics3 architecture

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Vision encoder configuration (SiglipVisionTransformer)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisionConfig {
    pub attention_dropout: f64,
    pub hidden_act: String,
    pub hidden_size: usize,
    pub image_size: usize,
    pub initializer_range: f64,
    pub intermediate_size: usize,
    pub layer_norm_eps: f64,
    pub max_image_size: HashMap<String, usize>,
    pub model_type: String,
    pub num_attention_heads: usize,
    pub num_channels: usize,
    pub num_hidden_layers: usize,
    pub patch_size: usize,
    pub size: HashMap<String, usize>,
    pub torch_dtype: String,
    pub use_base_siglip: bool,
}

/// Text decoder configuration (VLlama3ForCausalLM)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextConfig {
    pub architectures: Vec<String>,
    pub attention_bias: bool,
    pub attention_dropout: f64,
    pub bos_token_id: i64,
    pub eos_token_id: i64,
    pub head_dim: usize,
    pub hidden_act: String,
    pub hidden_size: usize,
    pub initializer_range: f64,
    pub intermediate_size: usize,
    pub max_position_embeddings: usize,
    pub mlp_bias: bool,
    pub model_type: String,
    pub num_attention_heads: usize,
    pub num_hidden_layers: usize,
    pub num_key_value_heads: usize,
    pub pretraining_tp: usize,
    pub rms_norm_eps: f64,
    pub rope_scaling: Option<serde_json::Value>,
    pub rope_theta: f64,
    pub torch_dtype: String,
    pub use_cache: bool,
    pub vocab_size: usize,
    pub tie_word_embeddings: bool,
}

/// Perceiver resampler configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerceiverConfig {
    pub attention_dropout: f64,
    pub hidden_act: String,
    pub model_type: String,
    pub num_key_value_heads: usize,
    pub qk_layer_norms_perceiver: bool,
    pub resampler_depth: usize,
    pub resampler_head_dim: usize,
    pub resampler_n_heads: usize,
    pub resampler_n_latents: usize,
}

/// Full Idefics3 model configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Idefics3Config {
    #[serde(rename = "_flash_attn_2_enabled")]
    pub flash_attn_2_enabled: bool,
    pub architectures: Vec<String>,
    pub attention_bias: bool,
    pub attention_dropout: f64,
    pub bos_token_id: i64,
    pub eos_token_id: i64,
    pub freeze_lm_head: bool,
    pub freeze_text_layers: bool,
    pub freeze_text_module_exceptions: Vec<String>,
    pub freeze_vision_layers: bool,
    pub freeze_vision_module_exceptions: Vec<String>,
    pub head_dim: usize,
    pub hidden_act: String,
    pub hidden_size: usize,
    pub image_token_id: i64,
    pub initializer_range: f64,
    pub intermediate_size: usize,
    pub max_position_embeddings: usize,
    pub mlp_bias: bool,
    pub model_type: String,
    pub neftune_noise_alpha: f64,
    pub num_attention_heads: usize,
    pub num_hidden_layers: usize,
    pub num_key_value_heads: usize,
    pub pad_token_id: i64,
    pub perceiver_config: PerceiverConfig,
    pub pixel_shuffle_factor: usize,
    pub pretraining_tp: usize,
    pub qk_layer_norms: bool,
    pub rms_norm_eps: f64,
    pub rope_scaling: Option<serde_json::Value>,
    pub rope_theta: f64,
    pub scale_factor: usize,
    pub text_config: TextConfig,
    pub tie_word_embeddings: bool,
    pub torch_dtype: String,
    pub transformers_version: String,
    pub use_cache: bool,
    pub use_resampler: bool,
    pub vision_config: VisionConfig,
}

impl Idefics3Config {
    /// Load configuration from config.json file
    pub fn from_pretrained<P: AsRef<Path>>(
        model_dir: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = model_dir.as_ref().join("config.json");
        let config_str = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config.json: {}", e))?;
        let config: Idefics3Config = serde_json::from_str(&config_str)
            .map_err(|e| format!("Failed to parse config.json: {}", e))?;
        Ok(config)
    }

    /// Get total vocabulary size
    #[inline]
    pub const fn vocab_size(&self) -> usize {
        self.text_config.vocab_size
    }

    /// Get hidden dimension for text decoder
    #[inline]
    pub const fn text_hidden_size(&self) -> usize {
        self.text_config.hidden_size
    }

    /// Get hidden dimension for vision encoder
    #[inline]
    pub const fn vision_hidden_size(&self) -> usize {
        self.vision_config.hidden_size
    }

    /// Get number of text decoder layers
    #[inline]
    pub const fn num_text_layers(&self) -> usize {
        self.text_config.num_hidden_layers
    }

    /// Get number of vision encoder layers
    #[inline]
    pub const fn num_vision_layers(&self) -> usize {
        self.vision_config.num_hidden_layers
    }

    /// Get image patch size
    #[inline]
    pub const fn patch_size(&self) -> usize {
        self.vision_config.patch_size
    }

    /// Get image size (height/width)
    #[inline]
    pub const fn image_size(&self) -> usize {
        self.vision_config.image_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log;

    #[test]
    fn test_load_config() {
        // Test loading CodeFormula config
        let model_dir = std::path::Path::new(env!("HOME"))
            .join(".cache/huggingface/hub/models--ds4sd--CodeFormulaV2/snapshots/ecedbe111d15c2dc60bfd4a823cbe80127b58af4");

        if !model_dir.exists() {
            log::warn!("Skipping test: model directory not found");
            return;
        }

        let config = Idefics3Config::from_pretrained(&model_dir).expect("Failed to load config");

        // Verify key configuration values
        assert_eq!(config.model_type, "idefics3");
        assert_eq!(config.architectures[0], "Idefics3ForConditionalGeneration");

        // Verify text config
        assert_eq!(config.text_config.hidden_size, 576);
        assert_eq!(config.text_config.num_hidden_layers, 30);
        assert_eq!(config.text_config.num_attention_heads, 9);
        assert_eq!(config.text_config.num_key_value_heads, 3);
        assert_eq!(config.text_config.vocab_size, 100480);

        // Verify vision config
        assert_eq!(config.vision_config.hidden_size, 768);
        assert_eq!(config.vision_config.num_hidden_layers, 12);
        assert_eq!(config.vision_config.num_attention_heads, 12);
        assert_eq!(config.vision_config.patch_size, 16);
        assert_eq!(config.vision_config.image_size, 512);

        // Verify perceiver config
        assert_eq!(config.perceiver_config.resampler_n_latents, 64);
        assert_eq!(config.perceiver_config.resampler_depth, 6);
        assert_eq!(config.perceiver_config.resampler_n_heads, 16);

        // Verify special tokens
        assert_eq!(config.image_token_id, 100270);
        assert_eq!(config.bos_token_id, 100264);
        assert_eq!(config.eos_token_id, 100338);
        assert_eq!(config.pad_token_id, 100256);
    }

    #[test]
    fn test_config_accessors() {
        let config = Idefics3Config {
            flash_attn_2_enabled: true,
            architectures: vec!["Idefics3ForConditionalGeneration".to_string()],
            attention_bias: false,
            attention_dropout: 0.0,
            bos_token_id: 100264,
            eos_token_id: 100338,
            freeze_lm_head: true,
            freeze_text_layers: true,
            freeze_text_module_exceptions: vec![],
            freeze_vision_layers: true,
            freeze_vision_module_exceptions: vec![],
            head_dim: 64,
            hidden_act: "silu".to_string(),
            hidden_size: 576,
            image_token_id: 100270,
            initializer_range: 0.02,
            intermediate_size: 1536,
            max_position_embeddings: 8192,
            mlp_bias: false,
            model_type: "idefics3".to_string(),
            neftune_noise_alpha: 0.0,
            num_attention_heads: 9,
            num_hidden_layers: 30,
            num_key_value_heads: 3,
            pad_token_id: 100256,
            perceiver_config: PerceiverConfig {
                attention_dropout: 0.0,
                hidden_act: "silu".to_string(),
                model_type: "vllama3".to_string(),
                num_key_value_heads: 1,
                qk_layer_norms_perceiver: false,
                resampler_depth: 6,
                resampler_head_dim: 96,
                resampler_n_heads: 16,
                resampler_n_latents: 64,
            },
            pixel_shuffle_factor: 4,
            pretraining_tp: 1,
            qk_layer_norms: false,
            rms_norm_eps: 1e-05,
            rope_scaling: None,
            rope_theta: 100000.0,
            scale_factor: 4,
            text_config: TextConfig {
                architectures: vec!["VLlama3ForCausalLM".to_string()],
                attention_bias: false,
                attention_dropout: 0.0,
                bos_token_id: 100257,
                eos_token_id: 100257,
                head_dim: 64,
                hidden_act: "silu".to_string(),
                hidden_size: 576,
                initializer_range: 0.02,
                intermediate_size: 1536,
                max_position_embeddings: 8192,
                mlp_bias: false,
                model_type: "llama".to_string(),
                num_attention_heads: 9,
                num_hidden_layers: 30,
                num_key_value_heads: 3,
                pretraining_tp: 1,
                rms_norm_eps: 1e-05,
                rope_scaling: None,
                rope_theta: 10000.0,
                torch_dtype: "bfloat16".to_string(),
                use_cache: true,
                vocab_size: 100480,
                tie_word_embeddings: false,
            },
            tie_word_embeddings: true,
            torch_dtype: "bfloat16".to_string(),
            transformers_version: "4.51.3".to_string(),
            use_cache: false,
            use_resampler: false,
            vision_config: VisionConfig {
                attention_dropout: 0.0,
                hidden_act: "gelu_pytorch_tanh".to_string(),
                hidden_size: 768,
                image_size: 512,
                initializer_range: 0.02,
                intermediate_size: 3072,
                layer_norm_eps: 1e-06,
                max_image_size: [("longest_edge".to_string(), 512)]
                    .iter()
                    .cloned()
                    .collect(),
                model_type: "idefics3_vision".to_string(),
                num_attention_heads: 12,
                num_channels: 3,
                num_hidden_layers: 12,
                patch_size: 16,
                size: [("longest_edge".to_string(), 2048)]
                    .iter()
                    .cloned()
                    .collect(),
                torch_dtype: "bfloat16".to_string(),
                use_base_siglip: true,
            },
        };

        assert_eq!(config.vocab_size(), 100480);
        assert_eq!(config.text_hidden_size(), 576);
        assert_eq!(config.vision_hidden_size(), 768);
        assert_eq!(config.num_text_layers(), 30);
        assert_eq!(config.num_vision_layers(), 12);
        assert_eq!(config.patch_size(), 16);
        assert_eq!(config.image_size(), 512);
    }
}
