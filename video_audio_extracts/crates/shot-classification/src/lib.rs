//! Shot Classification - Classify camera shot types (close-up, medium, wide, aerial)
//!
//! This module provides shot type classification for video frames using image analysis.
//!
//! **Shot Types:**
//! - Close-up: Tight framing on subject (face/detail fills frame)
//! - Medium: Waist-up or moderate subject distance
//! - Wide: Full scene visible, subject(s) smaller in frame
//! - Aerial: Overhead/birds-eye view
//! - Extreme Close-up: Very tight detail shot
//!
//! **V1 Implementation:** Rule-based heuristics using image statistics
//! **Future:** Can be enhanced with ML models (ResNet, CLIP zero-shot)

pub mod plugin;

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info};

/// Shot type classification result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShotType {
    #[serde(rename = "extreme_closeup")]
    ExtremeCloseup,
    #[serde(rename = "closeup")]
    Closeup,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "wide")]
    Wide,
    #[serde(rename = "aerial")]
    Aerial,
}

impl std::fmt::Display for ShotType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShotType::ExtremeCloseup => write!(f, "extreme_closeup"),
            ShotType::Closeup => write!(f, "closeup"),
            ShotType::Medium => write!(f, "medium"),
            ShotType::Wide => write!(f, "wide"),
            ShotType::Aerial => write!(f, "aerial"),
        }
    }
}

/// Shot classification result for a single frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShotClassification {
    pub timestamp_ms: Option<u64>,
    pub frame_number: Option<u32>,
    pub shot_type: ShotType,
    pub confidence: f32,
    pub metadata: ShotMetadata,
}

/// Additional metadata about the shot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShotMetadata {
    pub edge_density: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub dominant_region: String, // "center", "edges", "top", "bottom"
}

/// Image statistics for shot classification
#[derive(Debug)]
struct ImageStats {
    edge_density: f32,
    center_edge_density: f32,
    outer_edge_density: f32,
    brightness: f32,
    contrast: f32,
    top_edge_density: f32,
    bottom_edge_density: f32,
    left_right_asymmetry: f32,
}

/// Classify shot type from an image file
pub fn classify_shot(image_path: &Path) -> Result<ShotClassification> {
    info!("Classifying shot type for: {}", image_path.display());

    let img = image::open(image_path)
        .with_context(|| format!("Failed to open image: {}", image_path.display()))?;

    classify_shot_from_image(&img, None, None)
}

/// Classify shot type from a loaded image
pub fn classify_shot_from_image(
    img: &DynamicImage,
    timestamp_ms: Option<u64>,
    frame_number: Option<u32>,
) -> Result<ShotClassification> {
    let stats = compute_image_stats(img)?;

    debug!("Image stats: edge_density={:.3}, center_edge={:.3}, outer_edge={:.3}, brightness={:.3}, contrast={:.3}",
        stats.edge_density, stats.center_edge_density, stats.outer_edge_density,
        stats.brightness, stats.contrast);

    // Classify based on heuristics
    let (shot_type, confidence) = classify_from_stats(&stats);

    let dominant_region = determine_dominant_region(&stats);

    Ok(ShotClassification {
        timestamp_ms,
        frame_number,
        shot_type,
        confidence,
        metadata: ShotMetadata {
            edge_density: stats.edge_density,
            brightness: stats.brightness,
            contrast: stats.contrast,
            dominant_region,
        },
    })
}

/// Compute image statistics for classification
fn compute_image_stats(img: &DynamicImage) -> Result<ImageStats> {
    let (width, height) = img.dimensions();
    let gray = img.to_luma8();

    // Compute edge density using Sobel-like operator
    let edge_density = compute_edge_density(&gray);

    // Compute edge density in center region (30% of image)
    let center_x1 = width * 35 / 100;
    let center_y1 = height * 35 / 100;
    let center_x2 = width * 65 / 100;
    let center_y2 = height * 65 / 100;
    let center_region = image::imageops::crop_imm(
        &gray,
        center_x1,
        center_y1,
        center_x2 - center_x1,
        center_y2 - center_y1,
    )
    .to_image();
    let center_edge_density = compute_edge_density(&center_region);

    // Compute edge density in outer regions (edges of frame)
    let outer_width = width / 10; // 10% border
    let outer_height = height / 10;
    let mut outer_edge_sum = 0.0;
    let mut outer_pixel_count = 0;

    // Top and bottom edges
    for y in [0, height.saturating_sub(outer_height)] {
        let region =
            image::imageops::crop_imm(&gray, 0, y, width, outer_height.min(height - y)).to_image();
        outer_edge_sum +=
            compute_edge_density(&region) * region.width() as f32 * region.height() as f32;
        outer_pixel_count += region.width() * region.height();
    }

    // Left and right edges (excluding corners already counted)
    for x in [0, width.saturating_sub(outer_width)] {
        let region = image::imageops::crop_imm(
            &gray,
            x,
            outer_height,
            outer_width.min(width - x),
            height.saturating_sub(2 * outer_height),
        )
        .to_image();
        outer_edge_sum +=
            compute_edge_density(&region) * region.width() as f32 * region.height() as f32;
        outer_pixel_count += region.width() * region.height();
    }

    let outer_edge_density = if outer_pixel_count > 0 {
        outer_edge_sum / outer_pixel_count as f32
    } else {
        0.0
    };

    // Compute brightness and contrast
    let (brightness, contrast) = compute_brightness_contrast(&gray);

    // Top vs bottom edge density (for aerial detection)
    let top_region = image::imageops::crop_imm(&gray, 0, 0, width, height / 3).to_image();
    let top_edge_density = compute_edge_density(&top_region);

    let bottom_region =
        image::imageops::crop_imm(&gray, 0, height * 2 / 3, width, height / 3).to_image();
    let bottom_edge_density = compute_edge_density(&bottom_region);

    // Left vs right asymmetry
    let left_region = image::imageops::crop_imm(&gray, 0, 0, width / 2, height).to_image();
    let right_region = image::imageops::crop_imm(&gray, width / 2, 0, width / 2, height).to_image();
    let left_edge = compute_edge_density(&left_region);
    let right_edge = compute_edge_density(&right_region);
    let left_right_asymmetry = (left_edge - right_edge).abs();

    Ok(ImageStats {
        edge_density,
        center_edge_density,
        outer_edge_density,
        brightness,
        contrast,
        top_edge_density,
        bottom_edge_density,
        left_right_asymmetry,
    })
}

/// Compute edge density using simple gradient approximation
fn compute_edge_density(gray: &image::GrayImage) -> f32 {
    let (width, height) = gray.dimensions();
    if width < 2 || height < 2 {
        return 0.0;
    }

    let mut edge_sum = 0.0;
    let mut count = 0;

    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let p = gray.get_pixel(x, y)[0] as f32;
            let px = gray.get_pixel(x + 1, y)[0] as f32;
            let py = gray.get_pixel(x, y + 1)[0] as f32;

            let gx = (px - p).abs();
            let gy = (py - p).abs();
            let gradient = (gx * gx + gy * gy).sqrt();

            edge_sum += gradient;
            count += 1;
        }
    }

    if count > 0 {
        edge_sum / count as f32 / 255.0 // Normalize to [0, 1]
    } else {
        0.0
    }
}

/// Compute brightness and contrast
fn compute_brightness_contrast(gray: &image::GrayImage) -> (f32, f32) {
    let pixel_count = (gray.width() * gray.height()) as usize;
    let mut pixels: Vec<u8> = Vec::with_capacity(pixel_count);
    pixels.extend(gray.pixels().map(|p| p[0]));
    if pixels.is_empty() {
        return (0.0, 0.0);
    }

    let sum: u32 = pixels.iter().map(|&p| p as u32).sum();
    let mean = sum as f32 / pixels.len() as f32;
    let brightness = mean / 255.0;

    let variance: f32 = pixels
        .iter()
        .map(|&p| {
            let diff = p as f32 - mean;
            diff * diff
        })
        .sum::<f32>()
        / pixels.len() as f32;

    let std_dev = variance.sqrt();
    let contrast = std_dev / 255.0;

    (brightness, contrast)
}

/// Classify shot type from computed statistics
fn classify_from_stats(stats: &ImageStats) -> (ShotType, f32) {
    // Heuristic rules for shot classification

    // Aerial shot: high edge density at top (structured patterns from above)
    if stats.top_edge_density > stats.bottom_edge_density * 1.3
        && stats.edge_density > 0.15
        && stats.left_right_asymmetry < 0.05
    {
        return (ShotType::Aerial, 0.75);
    }

    // Extreme close-up: very high center edge density, high contrast
    if stats.center_edge_density > 0.25 && stats.contrast > 0.25 {
        return (ShotType::ExtremeCloseup, 0.80);
    }

    // Close-up: high center edge density
    if stats.center_edge_density > 0.18 {
        return (ShotType::Closeup, 0.75);
    }

    // Wide shot: high outer edge density, lower center density
    if stats.outer_edge_density > stats.center_edge_density * 1.2 && stats.edge_density > 0.12 {
        return (ShotType::Wide, 0.70);
    }

    // Medium shot: balanced edge distribution
    if stats.center_edge_density > 0.10 && stats.edge_density > 0.10 {
        return (ShotType::Medium, 0.65);
    }

    // Default: medium shot (most common)
    (ShotType::Medium, 0.50)
}

/// Determine dominant region of activity
fn determine_dominant_region(stats: &ImageStats) -> String {
    if stats.center_edge_density > stats.outer_edge_density * 1.3 {
        "center".to_string()
    } else if stats.outer_edge_density > stats.center_edge_density * 1.3 {
        "edges".to_string()
    } else if stats.top_edge_density > stats.bottom_edge_density * 1.2 {
        "top".to_string()
    } else if stats.bottom_edge_density > stats.top_edge_density * 1.2 {
        "bottom".to_string()
    } else {
        "balanced".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shot_type_display() {
        assert_eq!(ShotType::Closeup.to_string(), "closeup");
        assert_eq!(ShotType::Medium.to_string(), "medium");
        assert_eq!(ShotType::Wide.to_string(), "wide");
        assert_eq!(ShotType::Aerial.to_string(), "aerial");
        assert_eq!(ShotType::ExtremeCloseup.to_string(), "extreme_closeup");
    }

    #[test]
    fn test_edge_density_computation() {
        // Create a simple gradient image
        let mut img = image::GrayImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                img.put_pixel(x, y, image::Luma([(x + y) as u8]));
            }
        }

        let density = compute_edge_density(&img);
        assert!(
            density > 0.0,
            "Edge density should be positive for gradient"
        );
        assert!(density < 1.0, "Edge density should be normalized");
    }

    #[test]
    fn test_brightness_contrast() {
        // Bright image
        let bright = image::GrayImage::from_fn(100, 100, |_, _| image::Luma([200]));
        let (brightness, _) = compute_brightness_contrast(&bright);
        assert!(brightness > 0.7, "Bright image should have high brightness");

        // Dark image
        let dark = image::GrayImage::from_fn(100, 100, |_, _| image::Luma([50]));
        let (darkness, _) = compute_brightness_contrast(&dark);
        assert!(darkness < 0.3, "Dark image should have low brightness");
    }
}
