//! Text generation logic for caption generation models
//!
//! This module provides tokenizer loading and autoregressive text generation
//! for vision-language models like BLIP.

use ndarray::{Array2, Array4};
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
    value::Value,
};
use std::path::Path;
use tokenizers::Tokenizer;
use tracing::{debug, info};

use crate::CaptionError;

/// Text generator with tokenizer and ONNX session
pub struct TextGenerator {
    tokenizer: Tokenizer,
    session: Session,
    bos_token_id: u32,
    eos_token_id: u32,
    #[allow(dead_code)]
    pad_token_id: u32, // Reserved for future use (e.g., batch generation)
}

impl TextGenerator {
    /// Load tokenizer and ONNX session
    pub fn new(
        tokenizer_path: impl AsRef<Path>,
        model_path: impl AsRef<Path>,
    ) -> Result<Self, CaptionError> {
        let tokenizer_path = tokenizer_path.as_ref();
        let model_path = model_path.as_ref();

        info!("Loading tokenizer from {:?}", tokenizer_path);
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| CaptionError::InvalidConfig(format!("Failed to load tokenizer: {}", e)))?;

        info!("Loading ONNX model from {:?}", model_path);
        let session = Session::builder()
            .map_err(CaptionError::OrtError)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(CaptionError::OrtError)?
            .commit_from_file(model_path)
            .map_err(CaptionError::OrtError)?;

        // Get special token IDs from tokenizer
        let bos_token_id = tokenizer
            .token_to_id("[CLS]")
            .unwrap_or(101); // BERT [CLS] is typically 101
        let eos_token_id = tokenizer
            .token_to_id("[SEP]")
            .unwrap_or(102); // BERT [SEP] is typically 102
        let pad_token_id = tokenizer
            .token_to_id("[PAD]")
            .unwrap_or(0); // BERT [PAD] is typically 0

        debug!(
            "Special tokens: BOS={}, EOS={}, PAD={}",
            bos_token_id, eos_token_id, pad_token_id
        );

        Ok(Self {
            tokenizer,
            session,
            bos_token_id,
            eos_token_id,
            pad_token_id,
        })
    }

    /// Generate caption using greedy decoding
    ///
    /// # Arguments
    /// * `pixel_values` - Image tensor [1, 3, 384, 384]
    /// * `max_length` - Maximum caption length in tokens
    ///
    /// # Returns
    /// Generated caption text
    pub fn generate_greedy(
        &mut self,
        pixel_values: &Array4<f32>,
        max_length: usize,
    ) -> Result<String, CaptionError> {
        let batch_size = 1;

        // Initialize with BOS token
        let mut input_ids = vec![self.bos_token_id as i64];
        let mut attention_mask = vec![1i64];

        debug!("Starting greedy generation (max_length={})", max_length);

        // Autoregressive generation loop
        for step in 0..max_length {
            // Prepare inputs
            let input_ids_len = input_ids.len();
            let input_ids_array = Array2::from_shape_vec(
                (batch_size, input_ids_len),
                input_ids.clone(),
            )
            .map_err(|e| CaptionError::InvalidOutput(format!("Failed to create input_ids array: {}", e)))?;

            let attention_mask_array = Array2::from_shape_vec(
                (batch_size, attention_mask.len()),
                attention_mask.clone(),
            )
            .map_err(|e| CaptionError::InvalidOutput(format!("Failed to create attention_mask array: {}", e)))?;

            // Create ONNX tensors
            let pixel_values_tensor = Value::from_array(pixel_values.clone())
                .map_err(|e| CaptionError::InvalidOutput(format!("Failed to create pixel_values tensor: {}", e)))?;
            let input_ids_tensor = Value::from_array(input_ids_array)
                .map_err(|e| CaptionError::InvalidOutput(format!("Failed to create input_ids tensor: {}", e)))?;
            let attention_mask_tensor = Value::from_array(attention_mask_array)
                .map_err(|e| CaptionError::InvalidOutput(format!("Failed to create attention_mask tensor: {}", e)))?;

            // Run inference
            let outputs = self.session
                .run(ort::inputs![
                    "pixel_values" => pixel_values_tensor,
                    "input_ids" => input_ids_tensor,
                    "attention_mask" => attention_mask_tensor,
                ])
                .map_err(CaptionError::OrtError)?;

            // Extract logits: [batch_size, seq_len, vocab_size]
            let (logits_shape, logits_data) = outputs["logits"]
                .try_extract_tensor::<f32>()
                .map_err(|e| CaptionError::InvalidOutput(format!("Failed to extract logits: {}", e)))?;

            // Validate shape: [batch_size, seq_len, vocab_size]
            if logits_shape.len() != 3 {
                return Err(CaptionError::InvalidOutput(format!(
                    "Invalid logits shape: {:?}", logits_shape
                )));
            }

            let vocab_size = logits_shape[2] as usize;

            // Get logits for last position: logits[0, input_ids_len-1, :]
            // Index calculation: batch * (seq_len * vocab_size) + (input_ids_len-1) * vocab_size
            let last_position_offset = (input_ids_len - 1) * vocab_size;
            let last_logits = &logits_data[last_position_offset..last_position_offset + vocab_size];

            // Find token with highest probability (greedy decoding)
            let next_token_id = last_logits
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| idx as i64)
                .ok_or_else(|| CaptionError::InvalidOutput("No valid token found".to_string()))?;

            debug!("Step {}: Generated token ID {}", step, next_token_id);

            // Check for EOS token
            if next_token_id == self.eos_token_id as i64 {
                debug!("Generated EOS token, stopping generation");
                break;
            }

            // Append next token
            input_ids.push(next_token_id);
            attention_mask.push(1);

            // Stop if we exceed max length
            if input_ids.len() >= max_length + 1 {
                debug!("Reached max length, stopping generation");
                break;
            }
        }

        // Decode tokens to text
        let token_ids: Vec<u32> = input_ids
            .iter()
            .skip(1) // Skip BOS token
            .map(|&id| id as u32)
            .collect();

        let caption = self.tokenizer
            .decode(&token_ids, true)
            .map_err(|e| CaptionError::InvalidOutput(format!("Failed to decode tokens: {}", e)))?;

        debug!("Generated caption: '{}'", caption);

        Ok(caption)
    }

    /// Generate caption using beam search
    ///
    /// # Arguments
    /// * `pixel_values` - Image tensor [1, 3, 384, 384]
    /// * `max_length` - Maximum caption length in tokens
    /// * `num_beams` - Number of beams (typically 3-5)
    ///
    /// # Returns
    /// Generated caption text
    ///
    /// # Note
    /// Beam search provides better quality captions than greedy decoding
    /// but is slower (O(num_beams) time complexity).
    pub fn generate_beam_search(
        &mut self,
        pixel_values: &Array4<f32>,
        max_length: usize,
        num_beams: usize,
    ) -> Result<String, CaptionError> {
        // TODO: Implement beam search
        // For now, fall back to greedy decoding
        debug!(
            "Beam search not yet implemented (num_beams={}), falling back to greedy",
            num_beams
        );
        self.generate_greedy(pixel_values, max_length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_generator_creation() {
        // This test requires tokenizer and model files
        // Skip if files don't exist
        let tokenizer_path = "models/caption-generation/tokenizer.json";
        let model_path = "models/caption-generation/blip.onnx";

        if std::path::Path::new(tokenizer_path).exists()
            && std::path::Path::new(model_path).exists()
        {
            let generator = TextGenerator::new(tokenizer_path, model_path);
            assert!(generator.is_ok());
        }
    }
}
