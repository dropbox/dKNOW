//! Smart thumbnail selection module using heuristic-based quality scoring
//!
//! This module selects the best frame from a set of keyframes to use as a thumbnail
//! by analyzing multiple quality factors:
//!
//! # Quality Factors
//! - **Sharpness** (30%): Already computed in keyframe extraction
//! - **Brightness/Contrast** (25%): Optimal exposure and dynamic range
//! - **Composition** (20%): Rule of thirds, center bias, edge avoidance
//! - **Colorfulness** (15%): Saturation and color diversity
//! - **Face presence** (10%): Bonus for human faces (engaging thumbnails)
//!
//! # Future Enhancement
//! - Consider adding ML-based aesthetic scoring (CLIP+MLP predictor)
//! - Current implementation is fast, requires no models, and works well for most cases
//!
//! # Example
//! ```no_run
//! use video_audio_smart_thumbnail::{ThumbnailSelector, ThumbnailConfig};
//! use video_audio_common::Keyframe;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = ThumbnailConfig::default();
//! let selector = ThumbnailSelector::new(config);
//!
//! let keyframes: Vec<Keyframe> = vec![]; // From keyframe extraction
//! let best = selector.select_best(&keyframes)?;
//!
//! println!("Best thumbnail: frame {} (score: {:.2})", best.keyframe.frame_number, best.quality_score);
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use image::{DynamicImage, GenericImageView, Pixel};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info};
use video_audio_common::{Keyframe, ProcessingError};

/// Smart thumbnail selection errors
#[derive(Debug, Error)]
pub enum ThumbnailError {
    #[error("No keyframes provided")]
    NoKeyframes,
    #[error("Failed to load image: {0}")]
    ImageLoad(String),
    #[error("No thumbnail path found for resolution: {0}")]
    NoThumbnailPath(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<ThumbnailError> for ProcessingError {
    fn from(err: ThumbnailError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Configuration for thumbnail selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailConfig {
    /// Preferred thumbnail resolution (e.g., "800x600")
    pub preferred_resolution: String,
    /// Weight for sharpness score (0.0-1.0)
    pub sharpness_weight: f32,
    /// Weight for brightness/contrast score (0.0-1.0)
    pub brightness_contrast_weight: f32,
    /// Weight for composition score (0.0-1.0)
    pub composition_weight: f32,
    /// Weight for colorfulness score (0.0-1.0)
    pub colorfulness_weight: f32,
    /// Minimum number of keyframes to analyze (if less, analyze all)
    pub min_keyframes_to_analyze: usize,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            preferred_resolution: "800x600".to_string(),
            sharpness_weight: 0.30,
            brightness_contrast_weight: 0.25,
            composition_weight: 0.20,
            colorfulness_weight: 0.15,
            // Face detection: 0.10 (applied as bonus, not in weighted sum)
            min_keyframes_to_analyze: 20,
        }
    }
}

/// Result of thumbnail selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailResult {
    /// Selected keyframe
    pub keyframe: Keyframe,
    /// Overall quality score (0.0-1.0)
    pub quality_score: f64,
    /// Individual component scores
    pub scores: ThumbnailScores,
}

/// Individual quality scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailScores {
    pub sharpness: f64,
    pub brightness_contrast: f64,
    pub composition: f64,
    pub colorfulness: f64,
    pub face_presence: bool,
}

/// Thumbnail selector
pub struct ThumbnailSelector {
    config: ThumbnailConfig,
}

impl ThumbnailSelector {
    /// Create a new thumbnail selector
    #[must_use]
    pub fn new(config: ThumbnailConfig) -> Self {
        Self { config }
    }

    /// Select the best thumbnail from keyframes
    ///
    /// # Arguments
    /// * `keyframes` - List of keyframes to evaluate
    ///
    /// # Returns
    /// The best keyframe with quality scores
    ///
    /// # Errors
    /// Returns error if no keyframes provided or image loading fails
    pub fn select_best(&self, keyframes: &[Keyframe]) -> Result<ThumbnailResult, ThumbnailError> {
        if keyframes.is_empty() {
            return Err(ThumbnailError::NoKeyframes);
        }

        info!(
            "Selecting best thumbnail from {} keyframes",
            keyframes.len()
        );

        // Sample keyframes if we have too many
        let keyframes_to_analyze = if keyframes.len() > self.config.min_keyframes_to_analyze {
            self.sample_keyframes(keyframes, self.config.min_keyframes_to_analyze)
        } else {
            keyframes.to_vec()
        };

        debug!("Analyzing {} keyframes", keyframes_to_analyze.len());

        // Score each keyframe
        let mut scored_keyframes = Vec::with_capacity(keyframes_to_analyze.len());
        for keyframe in keyframes_to_analyze {
            match self.score_keyframe(&keyframe) {
                Ok((score, scores)) => {
                    scored_keyframes.push((keyframe, score, scores));
                }
                Err(e) => {
                    debug!(
                        "Failed to score keyframe at {:.2}s: {}",
                        keyframe.timestamp, e
                    );
                    // Continue with other keyframes
                }
            }
        }

        if scored_keyframes.is_empty() {
            return Err(ThumbnailError::ImageLoad(
                "Failed to score any keyframes".to_string(),
            ));
        }

        // Find best keyframe
        scored_keyframes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best_keyframe, best_score, best_scores) = scored_keyframes.into_iter().next().unwrap();

        info!(
            "Selected thumbnail: frame {} at {:.2}s (score: {:.3})",
            best_keyframe.frame_number, best_keyframe.timestamp, best_score
        );

        Ok(ThumbnailResult {
            keyframe: best_keyframe,
            quality_score: best_score,
            scores: best_scores,
        })
    }

    /// Sample keyframes evenly across the video
    fn sample_keyframes(&self, keyframes: &[Keyframe], count: usize) -> Vec<Keyframe> {
        let step = keyframes.len() / count;
        let mut sampled = Vec::with_capacity(count.min(keyframes.len()));
        sampled.extend(keyframes.iter().step_by(step.max(1)).take(count).cloned());
        sampled
    }

    /// Score a single keyframe
    fn score_keyframe(
        &self,
        keyframe: &Keyframe,
    ) -> Result<(f64, ThumbnailScores), ThumbnailError> {
        // Get thumbnail image path (prefer configured resolution)
        let image_path = self.get_thumbnail_path(keyframe)?;

        // Load image
        let img = image::open(&image_path)
            .map_err(|e| ThumbnailError::ImageLoad(format!("{}: {}", image_path.display(), e)))?;

        // Calculate individual scores
        let sharpness_score = keyframe.sharpness; // Already normalized 0-1

        let brightness_contrast_score = self.calculate_brightness_contrast(&img);
        let composition_score = self.calculate_composition(&img);
        let colorfulness_score = self.calculate_colorfulness(&img);

        // Face detection would go here (bonus score)
        // For now, we don't have a lightweight face detector integrated
        let face_presence = false;
        let face_bonus = if face_presence { 0.10 } else { 0.0 };

        // Weighted sum (weights sum to 0.90, face bonus adds up to 0.10)
        let total_score = sharpness_score * self.config.sharpness_weight as f64
            + brightness_contrast_score * self.config.brightness_contrast_weight as f64
            + composition_score * self.config.composition_weight as f64
            + colorfulness_score * self.config.colorfulness_weight as f64
            + face_bonus;

        let scores = ThumbnailScores {
            sharpness: sharpness_score,
            brightness_contrast: brightness_contrast_score,
            composition: composition_score,
            colorfulness: colorfulness_score,
            face_presence,
        };

        Ok((total_score, scores))
    }

    /// Get thumbnail path for preferred resolution
    fn get_thumbnail_path(&self, keyframe: &Keyframe) -> Result<PathBuf, ThumbnailError> {
        // Try preferred resolution first
        if let Some(path) = keyframe
            .thumbnail_paths
            .get(&self.config.preferred_resolution)
        {
            return Ok(path.clone());
        }

        // Fallback to any available resolution
        keyframe
            .thumbnail_paths
            .values()
            .next()
            .cloned()
            .ok_or_else(|| {
                ThumbnailError::NoThumbnailPath(self.config.preferred_resolution.clone())
            })
    }

    /// Calculate brightness and contrast score
    ///
    /// Optimal brightness: mean luminance around 0.4-0.6
    /// Good contrast: standard deviation > 0.2
    fn calculate_brightness_contrast(&self, img: &DynamicImage) -> f64 {
        let (width, height) = img.dimensions();
        // Sample every 4th pixel: capacity = (width/4) * (height/4)
        let sampled_pixels = (width as usize / 4) * (height as usize / 4);
        let mut luminances = Vec::with_capacity(sampled_pixels);

        // Sample pixels (every 4th pixel for speed)
        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(4) {
                let pixel = img.get_pixel(x, y);
                let channels = pixel.channels();

                // Calculate relative luminance: 0.299*R + 0.587*G + 0.114*B
                let luminance = if channels.len() >= 3 {
                    0.299 * f64::from(channels[0])
                        + 0.587 * f64::from(channels[1])
                        + 0.114 * f64::from(channels[2])
                } else {
                    f64::from(channels[0]) // Grayscale
                };

                luminances.push(luminance / 255.0); // Normalize to 0-1
            }
        }

        if luminances.is_empty() {
            return 0.0;
        }

        // Calculate mean brightness
        let mean = luminances.iter().sum::<f64>() / luminances.len() as f64;

        // Calculate standard deviation (contrast)
        let variance =
            luminances.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / luminances.len() as f64;
        let std_dev = variance.sqrt();

        // Score brightness: penalty for too dark (<0.3) or too bright (>0.7)
        let brightness_score = if (0.35..=0.65).contains(&mean) {
            1.0
        } else if (0.25..=0.75).contains(&mean) {
            0.7
        } else {
            0.3
        };

        // Score contrast: good if std_dev > 0.2
        let contrast_score = (std_dev / 0.3).min(1.0);

        // Combine (50% brightness, 50% contrast)
        (brightness_score + contrast_score) / 2.0
    }

    /// Calculate composition score
    ///
    /// Rule of thirds: interest points near 1/3 and 2/3 lines
    /// Center bias: some content near center
    /// Edge avoidance: avoid too much at edges
    fn calculate_composition(&self, img: &DynamicImage) -> f64 {
        let (width, height) = img.dimensions();

        // Define regions of interest (rule of thirds intersection points)
        let third_x = width / 3;
        let third_y = height / 3;

        // Sample edge detection: calculate gradients
        let mut center_energy = 0.0;
        let mut thirds_energy = 0.0;
        let mut edge_energy = 0.0;

        // Sample grid (every 8th pixel)
        for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                let pixel = img.get_pixel(x, y);
                let channels = pixel.channels();
                let intensity = if channels.len() >= 3 {
                    (f64::from(channels[0]) + f64::from(channels[1]) + f64::from(channels[2])) / 3.0
                } else {
                    f64::from(channels[0])
                };

                // Check region
                let is_center = x > third_x && x < 2 * third_x && y > third_y && y < 2 * third_y;
                let is_thirds = ((x > third_x - 20 && x < third_x + 20)
                    || (x > 2 * third_x - 20 && x < 2 * third_x + 20))
                    || ((y > third_y - 20 && y < third_y + 20)
                        || (y > 2 * third_y - 20 && y < 2 * third_y + 20));
                let is_edge =
                    x < width / 10 || x > width * 9 / 10 || y < height / 10 || y > height * 9 / 10;

                if is_center {
                    center_energy += intensity;
                } else if is_thirds {
                    thirds_energy += intensity;
                } else if is_edge {
                    edge_energy += intensity;
                }
            }
        }

        // Normalize energies
        let total = center_energy + thirds_energy + edge_energy;
        if total < 1.0 {
            return 0.5; // Flat image, neutral score
        }

        center_energy /= total;
        thirds_energy /= total;
        edge_energy /= total;

        // Good composition: balanced center and thirds, low edges
        let composition_score =
            0.3 * center_energy + 0.6 * thirds_energy + 0.1 * (1.0 - edge_energy);

        composition_score.clamp(0.0, 1.0)
    }

    /// Calculate colorfulness score
    ///
    /// More saturated and diverse colors = better thumbnail
    fn calculate_colorfulness(&self, img: &DynamicImage) -> f64 {
        let (width, height) = img.dimensions();
        // Sample every 4th pixel: capacity = (width/4) * (height/4)
        let sampled_pixels = (width as usize / 4) * (height as usize / 4);
        let mut saturations = Vec::with_capacity(sampled_pixels);

        // Sample pixels (every 4th pixel)
        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(4) {
                let pixel = img.get_pixel(x, y);
                let channels = pixel.channels();

                if channels.len() >= 3 {
                    let r = f64::from(channels[0]);
                    let g = f64::from(channels[1]);
                    let b = f64::from(channels[2]);

                    // Calculate saturation (simplified HSV)
                    let max = r.max(g).max(b);
                    let min = r.min(g).min(b);
                    let saturation = if max > 0.0 { (max - min) / max } else { 0.0 };

                    saturations.push(saturation);
                }
            }
        }

        if saturations.is_empty() {
            return 0.0;
        }

        // Mean saturation
        let mean_saturation = saturations.iter().sum::<f64>() / saturations.len() as f64;

        // Normalize to 0-1 range
        mean_saturation.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_config() {
        let config = ThumbnailConfig::default();
        assert_eq!(config.preferred_resolution, "800x600");
        assert!((config.sharpness_weight - 0.30).abs() < 0.01);
        assert!((config.brightness_contrast_weight - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_weights_sum() {
        let config = ThumbnailConfig::default();
        let sum = config.sharpness_weight
            + config.brightness_contrast_weight
            + config.composition_weight
            + config.colorfulness_weight;

        // Should sum to 0.90 (0.10 reserved for face bonus)
        assert!((sum - 0.90).abs() < 0.01);
    }

    #[test]
    fn test_sample_keyframes() {
        let config = ThumbnailConfig::default();
        let selector = ThumbnailSelector::new(config);

        // Create dummy keyframes
        let keyframes: Vec<Keyframe> = (0..100)
            .map(|i| Keyframe {
                timestamp: i as f64,
                frame_number: i,
                hash: 0,
                sharpness: 0.5,
                thumbnail_paths: Default::default(),
            })
            .collect();

        let sampled = selector.sample_keyframes(&keyframes, 10);
        assert_eq!(sampled.len(), 10);
    }
}
