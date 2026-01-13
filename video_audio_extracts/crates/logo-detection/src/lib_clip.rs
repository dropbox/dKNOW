//! CLIP-based logo detection via similarity search
//!
//! This module provides brand logo detection using CLIP embeddings and cosine similarity
//! instead of trained YOLOv8 models. This approach requires no training and is easily
//! extensible by adding new logo images to the database.
//!
//! # Features
//! - Zero-shot logo detection using pre-trained CLIP model
//! - Easily extensible logo database (just add images)
//! - Fast similarity search using cosine similarity
//! - Configurable confidence thresholds
//!
//! # Example
//! ```no_run
//! use video_audio_logo_detection::{ClipLogoDetector, ClipLogoConfig};
//! use image::open;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = ClipLogoConfig::default();
//! let mut detector = ClipLogoDetector::new(
//!     "models/embeddings/clip_vit_b32.onnx",
//!     "models/logo-detection/clip_database/logo_database.json",
//!     config
//! )?;
//!
//! let img = open("image.jpg")?.to_rgb8();
//! let detections = detector.detect(&img)?;
//!
//! for detection in detections {
//!     println!("{}: {:.2}% (similarity: {:.3})",
//!         detection.brand,
//!         detection.confidence * 100.0,
//!         detection.similarity
//!     );
//! }
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use image::{DynamicImage, RgbImage};
use ort::session::Session;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{debug, info};
use video_audio_embeddings::{CLIPModel, VisionEmbeddingConfig, VisionEmbeddings};

/// Configuration for CLIP-based logo detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipLogoConfig {
    /// Minimum similarity threshold for detections (0.0-1.0, cosine similarity)
    pub similarity_threshold: f32,
    /// Maximum number of detections to return per image
    pub max_detections: usize,
    /// Grid size for region extraction (NxN grid)
    /// Higher values = more regions = slower but more thorough
    pub grid_size: u32,
    /// Overlap ratio for sliding window (0.0-0.5)
    /// Higher values = more overlap = slower but more thorough
    pub overlap_ratio: f32,
}

impl Default for ClipLogoConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.75, // High threshold for CLIP similarity
            max_detections: 50,
            grid_size: 4, // 4x4 = 16 regions
            overlap_ratio: 0.25,
        }
    }
}

impl ClipLogoConfig {
    /// Create a fast detection config (fewer regions, higher threshold)
    #[must_use]
    pub fn fast() -> Self {
        Self {
            similarity_threshold: 0.80,
            max_detections: 20,
            grid_size: 3, // 3x3 = 9 regions
            overlap_ratio: 0.0,
        }
    }

    /// Create an accurate detection config (more regions, lower threshold)
    #[must_use]
    pub fn accurate() -> Self {
        Self {
            similarity_threshold: 0.70,
            max_detections: 100,
            grid_size: 6, // 6x6 = 36 regions
            overlap_ratio: 0.5,
        }
    }
}

/// Bounding box with normalized coordinates (0-1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    /// X coordinate of top-left corner (normalized 0-1)
    pub x: f32,
    /// Y coordinate of top-left corner (normalized 0-1)
    pub y: f32,
    /// Width of box (normalized 0-1)
    pub width: f32,
    /// Height of box (normalized 0-1)
    pub height: f32,
}

impl BoundingBox {
    /// Create a new bounding box
    #[must_use]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Calculate Intersection over Union (IoU) with another box
    #[must_use]
    #[inline]
    pub fn iou(&self, other: &BoundingBox) -> f32 {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        let intersection_width = (x2 - x1).max(0.0);
        let intersection_height = (y2 - y1).max(0.0);
        let intersection_area = intersection_width * intersection_height;

        let union_area = (self.width * self.height) + (other.width * other.height) - intersection_area;

        if union_area > 0.0 {
            intersection_area / union_area
        } else {
            0.0
        }
    }
}

/// Logo detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipLogoDetection {
    /// Logo ID from database
    pub logo_id: String,
    /// Brand name
    pub brand: String,
    /// Brand category (e.g., "tech", "sportswear")
    pub category: String,
    /// Confidence score (0-1), same as similarity for CLIP approach
    pub confidence: f32,
    /// Cosine similarity to logo embedding (0-1)
    pub similarity: f32,
    /// Bounding box with normalized coordinates
    pub bbox: BoundingBox,
}

/// Logo entry from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoEntry {
    pub id: String,
    pub brand: String,
    pub category: String,
    pub image_path: String,
    pub embedding: Vec<f32>,
}

/// Logo database
#[derive(Debug, Serialize, Deserialize)]
pub struct LogoDatabase {
    pub model: String,
    pub embedding_dim: usize,
    pub logos: Vec<LogoEntry>,
}

/// CLIP-based logo detector
pub struct ClipLogoDetector {
    /// CLIP model path
    clip_model_path: String,
    /// Logo database
    logo_database: LogoDatabase,
    /// Detection configuration
    config: ClipLogoConfig,
}

impl ClipLogoDetector {
    /// Create a new CLIP logo detector
    ///
    /// # Arguments
    /// * `clip_model_path` - Path to CLIP ONNX model (e.g., clip_vit_b32.onnx)
    /// * `logo_database_path` - Path to logo database JSON file
    /// * `config` - Detection configuration
    pub fn new<P: AsRef<Path>>(
        clip_model_path: P,
        logo_database_path: P,
        config: ClipLogoConfig,
    ) -> Result<Self> {
        info!(
            "Loading CLIP logo detector from {:?} with database {:?}",
            clip_model_path.as_ref(),
            logo_database_path.as_ref()
        );

        // Store CLIP model path (we'll load per-request to match static method pattern)
        let clip_model_path_str = clip_model_path.as_ref().to_string_lossy().to_string();

        // Load logo database
        let database_json = fs::read_to_string(logo_database_path.as_ref()).with_context(|| {
            format!(
                "Failed to read logo database from {:?}",
                logo_database_path.as_ref()
            )
        })?;

        let logo_database: LogoDatabase =
            serde_json::from_str(&database_json).context("Failed to parse logo database JSON")?;

        info!(
            "Loaded {} logos from database (model: {}, dim: {})",
            logo_database.logos.len(),
            logo_database.model,
            logo_database.embedding_dim
        );

        Ok(Self {
            clip_model_path: clip_model_path_str,
            logo_database,
            config,
        })
    }

    /// Detect logos using a pre-loaded session and database (for model caching)
    ///
    /// # Arguments
    /// * `session` - Pre-loaded ONNX Session for CLIP model
    /// * `image` - RGB image to process
    /// * `config` - Detection configuration
    /// * `vision_config` - CLIP vision configuration
    /// * `logo_database` - Logo database with embeddings
    pub fn detect_with_session(
        session: &mut Session,
        image: &RgbImage,
        config: &ClipLogoConfig,
        vision_config: &VisionEmbeddingConfig,
        logo_database: &LogoDatabase,
    ) -> Result<Vec<ClipLogoDetection>> {
        debug!(
            "Running CLIP logo detection on {}x{} image",
            image.width(),
            image.height()
        );

        // Extract regions from image (sliding window or grid)
        let regions = Self::extract_regions_static(image, config)?;
        debug!("Extracted {} regions for logo detection", regions.len());

        if regions.is_empty() {
            return Ok(Vec::new());
        }

        // Convert regions to DynamicImage for CLIP
        let region_images: Vec<DynamicImage> = regions
            .iter()
            .map(|(img, _bbox)| DynamicImage::ImageRgb8(img.clone()))
            .collect();

        // Extract CLIP embeddings for regions sequentially (one at a time)
        // Note: Sequential processing avoids CoreML batch inference issues
        // See reports/main/N236_LOGO_DETECTION_COREML_ISSUE.md for details
        let mut region_embeddings = Vec::with_capacity(region_images.len());
        for (idx, region_img) in region_images.iter().enumerate() {
            let embedding = VisionEmbeddings::extract_embeddings_with_session(
                session,
                vision_config,
                &[region_img.clone()],
            )
            .with_context(|| format!("Failed to extract CLIP embedding for region {}", idx))?;

            if embedding.is_empty() {
                return Err(anyhow::anyhow!("Empty embedding returned for region {}", idx));
            }

            region_embeddings.push(embedding[0].clone());
        }

        debug!(
            "Extracted {} CLIP embeddings for regions",
            region_embeddings.len()
        );

        // Compute similarities to all logos in database
        let mut detections = Vec::new();

        for (region_idx, ((_region_img, bbox), region_embedding)) in
            regions.iter().zip(region_embeddings.iter()).enumerate()
        {
            // Find best matching logo
            let mut best_match: Option<(&LogoEntry, f32)> = None;

            for logo in &logo_database.logos {
                let similarity = cosine_similarity(region_embedding, &logo.embedding);

                if similarity >= config.similarity_threshold {
                    if let Some((_, best_sim)) = best_match {
                        if similarity > best_sim {
                            best_match = Some((logo, similarity));
                        }
                    } else {
                        best_match = Some((logo, similarity));
                    }
                }
            }

            // Add detection if match found
            if let Some((logo, similarity)) = best_match {
                debug!(
                    "Region {} matched logo '{}' with similarity {:.3}",
                    region_idx, logo.brand, similarity
                );

                detections.push(ClipLogoDetection {
                    logo_id: logo.id.clone(),
                    brand: logo.brand.clone(),
                    category: logo.category.clone(),
                    confidence: similarity, // Confidence = similarity for CLIP approach
                    similarity,
                    bbox: bbox.clone(),
                });
            }
        }

        debug!("Found {} logo matches before NMS", detections.len());

        // Apply non-maximum suppression to remove duplicates
        let detections = Self::apply_nms_static(detections, 0.5);

        // Limit to max detections
        let detections: Vec<_> = detections
            .into_iter()
            .take(config.max_detections)
            .collect();

        info!("Detected {} logos", detections.len());

        Ok(detections)
    }

    /// Detect logos in a single image
    pub fn detect(&mut self, image: &RgbImage) -> Result<Vec<ClipLogoDetection>> {
        let vision_config = VisionEmbeddingConfig {
            model: CLIPModel::VitB32,
            model_path: self.clip_model_path.clone(),
            normalize: true,
            image_size: 224,
        };

        // Create VisionEmbeddings instance for this detection
        // TODO: Consider caching session for better performance
        let mut vision_embeddings = VisionEmbeddings::new(vision_config.clone())
            .context("Failed to load CLIP vision embeddings model")?;

        // Extract regions from image (sliding window or grid)
        let regions = Self::extract_regions_static(image, &self.config)?;
        debug!("Extracted {} regions for logo detection", regions.len());

        if regions.is_empty() {
            return Ok(Vec::new());
        }

        // Convert regions to DynamicImage for CLIP
        let region_images: Vec<DynamicImage> = regions
            .iter()
            .map(|(img, _bbox)| DynamicImage::ImageRgb8(img.clone()))
            .collect();

        // Extract CLIP embeddings for all regions (batch inference)
        let region_embeddings = vision_embeddings
            .extract_embeddings(&region_images)
            .context("Failed to extract CLIP embeddings for regions")?;

        debug!(
            "Extracted {} CLIP embeddings for regions",
            region_embeddings.len()
        );

        // Compute similarities to all logos in database
        let mut detections = Vec::new();

        for (region_idx, ((_region_img, bbox), region_embedding)) in
            regions.iter().zip(region_embeddings.iter()).enumerate()
        {
            // Find best matching logo
            let mut best_match: Option<(&LogoEntry, f32)> = None;

            for logo in &self.logo_database.logos {
                let similarity = cosine_similarity(region_embedding, &logo.embedding);

                if similarity >= self.config.similarity_threshold {
                    if let Some((_, best_sim)) = best_match {
                        if similarity > best_sim {
                            best_match = Some((logo, similarity));
                        }
                    } else {
                        best_match = Some((logo, similarity));
                    }
                }
            }

            // Add detection if match found
            if let Some((logo, similarity)) = best_match {
                debug!(
                    "Region {} matched logo '{}' with similarity {:.3}",
                    region_idx, logo.brand, similarity
                );

                detections.push(ClipLogoDetection {
                    logo_id: logo.id.clone(),
                    brand: logo.brand.clone(),
                    category: logo.category.clone(),
                    confidence: similarity, // Confidence = similarity for CLIP approach
                    similarity,
                    bbox: bbox.clone(),
                });
            }
        }

        debug!("Found {} logo matches before NMS", detections.len());

        // Apply non-maximum suppression to remove duplicates
        let detections = Self::apply_nms_static(detections, 0.5);

        // Limit to max detections
        let detections: Vec<_> = detections
            .into_iter()
            .take(self.config.max_detections)
            .collect();

        info!("Detected {} logos", detections.len());

        Ok(detections)
    }

    /// Extract regions from image using grid-based approach
    fn extract_regions_static(
        image: &RgbImage,
        config: &ClipLogoConfig,
    ) -> Result<Vec<(RgbImage, BoundingBox)>> {
        let img_width = image.width();
        let img_height = image.height();

        let grid_size = config.grid_size as f32;
        let overlap = config.overlap_ratio;

        // Calculate region size with overlap
        let region_width = img_width as f32 / grid_size;
        let region_height = img_height as f32 / grid_size;
        let step_x = region_width * (1.0 - overlap);
        let step_y = region_height * (1.0 - overlap);

        let mut regions = Vec::new();

        // Extract regions in grid pattern
        let mut y = 0.0;
        while y + region_height <= img_height as f32 {
            let mut x = 0.0;
            while x + region_width <= img_width as f32 {
                // Extract region
                let x_start = x as u32;
                let y_start = y as u32;
                let x_end = (x + region_width).min(img_width as f32) as u32;
                let y_end = (y + region_height).min(img_height as f32) as u32;

                if x_end > x_start && y_end > y_start {
                    let region = image::imageops::crop_imm(
                        image,
                        x_start,
                        y_start,
                        x_end - x_start,
                        y_end - y_start,
                    )
                    .to_image();

                    // Create normalized bounding box
                    let bbox = BoundingBox::new(
                        x / img_width as f32,
                        y / img_height as f32,
                        (x_end - x_start) as f32 / img_width as f32,
                        (y_end - y_start) as f32 / img_height as f32,
                    );

                    regions.push((region, bbox));
                }

                x += step_x;
            }
            y += step_y;
        }

        Ok(regions)
    }

    /// Apply non-maximum suppression to remove duplicate detections
    fn apply_nms_static(
        mut detections: Vec<ClipLogoDetection>,
        iou_threshold: f32,
    ) -> Vec<ClipLogoDetection> {
        // Sort by similarity/confidence (highest first)
        detections.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut keep = Vec::with_capacity(detections.len());

        while !detections.is_empty() {
            let current = detections.swap_remove(0);

            // Remove all detections with IoU > threshold for the same brand
            detections.retain(|det| {
                det.brand != current.brand || det.bbox.iou(&current.bbox) < iou_threshold
            });

            keep.push(current);
        }

        debug!("Detections after NMS: {}", keep.len());
        keep
    }

    /// Get number of logos in database
    #[must_use]
    pub fn num_logos(&self) -> usize {
        self.logo_database.logos.len()
    }
}

/// Compute cosine similarity between two embeddings
#[inline]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Embeddings must have same dimension");

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 1e-12 && norm_b > 1e-12 {
        dot_product / (norm_a * norm_b)
    } else {
        0.0
    }
}

// Note: Error conversion to ProcessingError is handled in plugin.rs via anyhow::Error

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&c, &d) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_bbox_iou() {
        let bbox1 = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
        let bbox2 = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
        assert!((bbox1.iou(&bbox2) - 1.0).abs() < 1e-6);

        let bbox3 = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
        let bbox4 = BoundingBox::new(0.5, 0.5, 0.5, 0.5);
        assert!((bbox3.iou(&bbox4) - 0.0).abs() < 1e-6);
    }
}
