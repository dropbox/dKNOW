//! Emotion detection using FER+ model.
//!
//! Detects 8 emotions from facial images (FER+ order):
//! - Neutral
//! - Happiness
//! - Surprise
//! - Sadness
//! - Anger
//! - Disgust
//! - Fear
//! - Contempt
//!
//! Model: Emotion FerPlus-8 from ONNX Model Zoo
//! Input: 64x64 grayscale images
//! Output: Emotion class probabilities

pub mod plugin;

use anyhow::{Context, Result};
use image::{DynamicImage, GrayImage};
use ndarray::{Array, Array4, Axis};
use ort::session::Session;
use ort::value::TensorRef;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Emotion classes supported by the model (FER+ order)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Emotion {
    Neutral,
    Happiness,
    Surprise,
    Sadness,
    Anger,
    Disgust,
    Fear,
    Contempt,
}

impl Emotion {
    /// Get emotion from class index (FER+ order)
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Emotion::Neutral),
            1 => Some(Emotion::Happiness),
            2 => Some(Emotion::Surprise),
            3 => Some(Emotion::Sadness),
            4 => Some(Emotion::Anger),
            5 => Some(Emotion::Disgust),
            6 => Some(Emotion::Fear),
            7 => Some(Emotion::Contempt),
            _ => None,
        }
    }

    /// Get emotion label as string (FER+ labels)
    pub fn as_str(&self) -> &'static str {
        match self {
            Emotion::Neutral => "neutral",
            Emotion::Happiness => "happiness",
            Emotion::Surprise => "surprise",
            Emotion::Sadness => "sadness",
            Emotion::Anger => "anger",
            Emotion::Disgust => "disgust",
            Emotion::Fear => "fear",
            Emotion::Contempt => "contempt",
        }
    }
}

/// Result of emotion detection for a single face
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionResult {
    /// Detected emotion (highest probability)
    pub emotion: Emotion,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Probabilities for all emotion classes (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probabilities: Option<Vec<f32>>,
}

/// Emotion detection configuration
#[derive(Debug, Clone, Default)]
pub struct EmotionDetectorConfig {
    /// Include full probability distribution in results
    pub include_probabilities: bool,
}

/// Emotion detector using ONNX Runtime
pub struct EmotionDetector {
    session: Arc<Mutex<Session>>,
    config: EmotionDetectorConfig,
    input_name: String,
    output_name: String,
}

impl EmotionDetector {
    /// Create new emotion detector from ONNX model file
    pub fn new<P: AsRef<Path>>(model_path: P, config: EmotionDetectorConfig) -> Result<Self> {
        let session = Session::builder()
            .context("Failed to create ONNX session builder")?
            .commit_from_file(model_path)
            .context("Failed to load emotion detection model")?;

        // Get input/output names from the session
        let input_name = session
            .inputs
            .first()
            .context("Model has no inputs")?
            .name
            .clone();
        let output_name = session
            .outputs
            .first()
            .context("Model has no outputs")?
            .name
            .clone();

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            config,
            input_name,
            output_name,
        })
    }

    /// Detect emotion in a single face image
    pub fn detect(&self, image: &DynamicImage) -> Result<EmotionResult> {
        // Preprocess image: convert to grayscale, resize to 64x64 (FER+ model)
        let gray = image.to_luma8();
        let resized = image::imageops::resize(&gray, 64, 64, image::imageops::FilterType::Triangle);

        // Convert to ndarray and normalize
        let input = self.preprocess_image(&resized)?;

        // Run inference
        let probabilities = self.run_inference(input)?;

        // Get emotion with highest probability
        let (emotion_idx, confidence) = probabilities
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        let emotion = Emotion::from_index(emotion_idx).context("Invalid emotion class index")?;

        Ok(EmotionResult {
            emotion,
            confidence: *confidence,
            probabilities: if self.config.include_probabilities {
                Some(probabilities)
            } else {
                None
            },
        })
    }

    /// Detect emotions in multiple face images (batch processing)
    pub fn detect_batch(&self, images: &[DynamicImage]) -> Result<Vec<EmotionResult>> {
        if images.is_empty() {
            return Ok(Vec::new());
        }

        // Preprocess all images
        let mut batch_input = Vec::with_capacity(images.len());
        for image in images {
            let gray = image.to_luma8();
            let resized =
                image::imageops::resize(&gray, 64, 64, image::imageops::FilterType::Triangle);
            let input = self.preprocess_image(&resized)?;
            batch_input.push(input);
        }

        // Stack into batch
        let mut views = Vec::with_capacity(batch_input.len());
        for arr in &batch_input {
            views.push(arr.view());
        }
        let batch_array = ndarray::stack(Axis(0), &views)?;

        // Run inference on batch
        let mut session = self.session.lock().unwrap();
        let input_tensor = TensorRef::from_array_view(batch_array.view())?;
        let outputs = session.run(ort::inputs![&*self.input_name => input_tensor])?;
        let output_tensor = outputs[0].try_extract_tensor::<f32>()?;
        let (_shape, data) = output_tensor;

        // Process results (FER+ has 8 emotion classes)
        let mut results = Vec::with_capacity(images.len());
        for i in 0..images.len() {
            let start_idx = i * 8;
            let end_idx = start_idx + 8;
            let probabilities: Vec<f32> = data[start_idx..end_idx].to_vec();

            // Apply softmax
            let probabilities = softmax(&probabilities);

            // Get emotion with highest probability
            let (emotion_idx, confidence) = probabilities
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();

            let emotion =
                Emotion::from_index(emotion_idx).context("Invalid emotion class index")?;

            results.push(EmotionResult {
                emotion,
                confidence: *confidence,
                probabilities: if self.config.include_probabilities {
                    Some(probabilities)
                } else {
                    None
                },
            });
        }

        Ok(results)
    }

    /// Preprocess image for model input
    fn preprocess_image(&self, image: &GrayImage) -> Result<Array4<f32>> {
        let (width, height) = image.dimensions();

        // Create ndarray from image data
        let mut array = Array::zeros((1, 1, height as usize, width as usize));
        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x, y).0[0];
                // Normalize to [0, 1]
                array[[0, 0, y as usize, x as usize]] = pixel as f32 / 255.0;
            }
        }

        Ok(array)
    }

    /// Run inference on preprocessed input
    fn run_inference(&self, input: Array4<f32>) -> Result<Vec<f32>> {
        let mut session = self.session.lock().unwrap();
        let input_tensor = TensorRef::from_array_view(input.view())?;
        let outputs = session.run(ort::inputs![&*self.input_name => input_tensor])?;

        let output_tensor = outputs[0].try_extract_tensor::<f32>()?;
        let (_shape, data) = output_tensor;

        // Apply softmax to get probabilities
        let probabilities = softmax(data);

        Ok(probabilities)
    }
}

/// Apply softmax function to convert logits to probabilities
fn softmax(logits: &[f32]) -> Vec<f32> {
    let max_logit = logits.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let mut exps = Vec::with_capacity(logits.len());
    exps.extend(logits.iter().map(|&x| (x - max_logit).exp()));
    let sum_exps: f32 = exps.iter().sum();
    let mut probs = Vec::with_capacity(exps.len());
    probs.extend(exps.iter().map(|&x| x / sum_exps));
    probs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_from_index() {
        assert_eq!(Emotion::from_index(0), Some(Emotion::Neutral));
        assert_eq!(Emotion::from_index(1), Some(Emotion::Happiness));
        assert_eq!(Emotion::from_index(7), Some(Emotion::Contempt));
        assert_eq!(Emotion::from_index(8), None);
    }

    #[test]
    fn test_softmax() {
        let logits = vec![1.0, 2.0, 3.0];
        let probs = softmax(&logits);
        assert!((probs.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!(probs[2] > probs[1]);
        assert!(probs[1] > probs[0]);
    }
}
