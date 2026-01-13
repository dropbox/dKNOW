// Intentional ML conversions: tensor indices, image dimensions
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

use crate::preprocessing::pil_resize_fixed_point::pil_resize_bilinear_fixed_point;
use ndarray::{Array3, Array4};

/// `LayoutPredictor` (RT-DETR) Preprocessing
///
/// Transforms raw page image into tensor for layout detection model.
///
/// Pipeline:
/// 1. Resize to 640x640 (bilinear interpolation) - fixed size, square
/// 2. Rescale to [0, 1] by dividing by 255
/// 3. Convert HWC → CHW
/// 4. Add batch dimension
///
/// Input: Raw RGB image (H, W, 3) uint8 [0, 255]
/// Output: Preprocessed tensor (1, 3, 640, 640) float32 [0, 1]
///
/// ## Preprocessing Parameters (from `RTDetrImageProcessor`)
///
/// **`HuggingFace` `RTDetrImageProcessor` config:**
/// - Size: {"height": 640, "width": 640}
/// - Do resize: True
/// - Do rescale: True (`rescale_factor` = 1/255 = 0.00392156862745098)
/// - Do normalize: False (no mean/std normalization)
/// - Do pad: False
/// - Resample: 2 (BILINEAR)
///
/// **Key differences from `RapidOCR`:**
/// - Fixed 640x640 size (`RapidOCR` uses variable sizes)
/// - No aspect ratio preservation (`RapidOCR` pads to preserve aspect)
/// - Range [0, 1] (`RapidOCR` uses [-1, 1])
/// - No normalization (`RapidOCR` normalizes to [-1, 1])
///
/// See: `inspect_rtdetr_preprocessing.py` for full parameter details
#[must_use = "returns the preprocessed tensor for layout model"]
pub fn layout_preprocess(image: &Array3<u8>) -> Array4<f32> {
    layout_preprocess_with_size(image, 640)
}

/// Default layout input resolution (640x640)
pub const DEFAULT_LAYOUT_RESOLUTION: usize = 640;

/// Resolution presets for layout model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum LayoutResolution {
    /// Full resolution (640x640) - baseline accuracy
    #[default]
    Full,
    /// Medium resolution (512x512) - ~1.56x faster, ~1-2% accuracy loss
    Medium,
    /// Fast resolution (448x448) - ~2x faster, ~3-5% accuracy loss
    Fast,
    /// Custom resolution
    Custom(usize),
}

impl std::fmt::Display for LayoutResolution {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "full (640x640)"),
            Self::Medium => write!(f, "medium (512x512)"),
            Self::Fast => write!(f, "fast (448x448)"),
            Self::Custom(size) => write!(f, "custom ({size}x{size})"),
        }
    }
}

impl std::str::FromStr for LayoutResolution {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        let trimmed = lower.trim();

        // Check named presets first (with optional size suffix like "full (640x640)")
        if trimmed.starts_with("full") {
            return Ok(Self::Full);
        }
        if trimmed.starts_with("medium") {
            return Ok(Self::Medium);
        }
        if trimmed.starts_with("fast") {
            return Ok(Self::Fast);
        }

        // Check exact numeric matches
        match trimmed {
            "640" | "640x640" => return Ok(Self::Full),
            "512" | "512x512" => return Ok(Self::Medium),
            "448" | "448x448" => return Ok(Self::Fast),
            _ => {}
        }

        // Check for "custom (NxN)" format from Display
        if let Some(rest) = trimmed.strip_prefix("custom") {
            let inner = rest.trim().trim_matches(|c| c == '(' || c == ')');
            if let Some((w, _h)) = inner.split_once('x') {
                if let Ok(size) = w.trim().parse::<usize>() {
                    return Ok(Self::Custom(size));
                }
            }
        }

        // Try parsing as a plain number
        if let Ok(size) = trimmed.parse::<usize>() {
            // Map to presets if exact match
            match size {
                640 => return Ok(Self::Full),
                512 => return Ok(Self::Medium),
                448 => return Ok(Self::Fast),
                _ => return Ok(Self::Custom(size)),
            }
        }

        // Try parsing "NxN" format
        if let Some((w, h)) = trimmed.split_once('x') {
            if let (Ok(width), Ok(height)) = (w.trim().parse::<usize>(), h.trim().parse::<usize>())
            {
                if width == height {
                    match width {
                        640 => return Ok(Self::Full),
                        512 => return Ok(Self::Medium),
                        448 => return Ok(Self::Fast),
                        _ => return Ok(Self::Custom(width)),
                    }
                }
            }
        }

        Err(format!(
            "Unknown layout resolution '{s}'. Expected: full, medium, fast, or a number (e.g., 512 or 512x512)"
        ))
    }
}

impl LayoutResolution {
    /// Get the target size in pixels
    #[must_use = "returns the resolution size in pixels"]
    pub const fn size(self) -> usize {
        match self {
            Self::Full => 640,
            Self::Medium => 512,
            Self::Fast => 448,
            Self::Custom(s) => s,
        }
    }

    /// Get expected speedup relative to Full resolution
    /// Based on quadratic compute scaling: speedup = `(DEFAULT_LAYOUT_RESOLUTION/size)²`
    #[must_use = "returns the expected speedup factor"]
    pub fn expected_speedup(self) -> f64 {
        let base = DEFAULT_LAYOUT_RESOLUTION as f64;
        let size = self.size() as f64;
        (base / size).powi(2)
    }
}

/// `LayoutPredictor` preprocessing with configurable resolution
///
/// Same as `layout_preprocess` but allows specifying target resolution.
/// Useful for speed/accuracy tradeoffs:
/// - 640x640: Full accuracy (baseline)
/// - 512x512: ~1.56x faster, ~1-2% accuracy loss
/// - 448x448: ~2.04x faster, ~3-5% accuracy loss
///
/// # Arguments
///
/// * `image` - Input RGB image (H, W, 3) uint8 [0, 255]
/// * `target_size` - Target resolution (both height and width)
///
/// # Returns
///
/// Preprocessed tensor (1, 3, `target_size`, `target_size`) float32 [0, 1]
#[must_use = "returns the preprocessed tensor for layout model"]
pub fn layout_preprocess_with_size(image: &Array3<u8>, target_size: usize) -> Array4<f32> {
    use std::time::Instant;

    let (_height, _width, channels) = image.dim();
    assert_eq!(channels, 3, "Image must have 3 channels (RGB)");
    assert!(
        target_size >= 224,
        "Target size must be at least 224 for meaningful detection"
    );
    assert!(
        target_size <= 1280,
        "Target size above 1280 not recommended"
    );

    // Step 1: Resize to target_size x target_size using PIL-compatible bilinear interpolation
    // Uses 22-bit fixed-point arithmetic to exactly match PIL's implementation

    let resize_start = Instant::now();
    let resized_rgb = pil_resize_bilinear_fixed_point(image, target_size, target_size);
    let _resize_time = resize_start.elapsed();
    #[cfg(feature = "debug-profiling")]
    {
        if std::env::var("PROFILE_PREPROCESS").is_ok() {
            log::warn!(
                "[PROFILE_PREPROCESS] Resize: {:.2} ms",
                _resize_time.as_secs_f64() * 1000.0
            );
        }
    }

    // Step 2: Convert HWC → CHW and rescale [0, 255] → [0, 1]
    let convert_start = Instant::now();
    let mut tensor = Array4::<f32>::zeros((1, 3, target_size, target_size));

    for y in 0..target_size {
        for x in 0..target_size {
            tensor[[0, 0, y, x]] = f32::from(resized_rgb[[y, x, 0]]) / 255.0;
            tensor[[0, 1, y, x]] = f32::from(resized_rgb[[y, x, 1]]) / 255.0;
            tensor[[0, 2, y, x]] = f32::from(resized_rgb[[y, x, 2]]) / 255.0;
        }
    }
    let _convert_time = convert_start.elapsed();
    #[cfg(feature = "debug-profiling")]
    {
        if std::env::var("PROFILE_PREPROCESS").is_ok() {
            log::warn!(
                "[PROFILE_PREPROCESS] Convert HWC→CHW + Rescale: {:.2} ms",
                _convert_time.as_secs_f64() * 1000.0
            );
        }
    }

    tensor
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

    #[test]
    fn test_layout_preprocess_shape() {
        // Create a dummy 612x792 RGB image
        let image = Array3::<u8>::zeros((792, 612, 3));
        let tensor = layout_preprocess(&image);

        // Check output shape
        assert_eq!(tensor.shape(), &[1, 3, 640, 640]);
    }

    #[test]
    fn test_layout_preprocess_range() {
        // Create a test image with known values
        let mut image = Array3::<u8>::zeros((792, 612, 3));
        image[[0, 0, 0]] = 0; // Black
        image[[0, 1, 0]] = 255; // White
        image[[0, 2, 0]] = 128; // Gray

        let tensor = layout_preprocess(&image);

        // Check value range
        let min = tensor.iter().copied().fold(f32::INFINITY, f32::min);
        let max = tensor.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        assert!(min >= 0.0, "Min value should be >= 0");
        assert!(max <= 1.0, "Max value should be <= 1");
    }

    #[test]
    fn test_layout_preprocess_with_size_512() {
        let image = Array3::<u8>::zeros((792, 612, 3));
        let tensor = layout_preprocess_with_size(&image, 512);
        assert_eq!(tensor.shape(), &[1, 3, 512, 512]);
    }

    #[test]
    fn test_layout_preprocess_with_size_448() {
        let image = Array3::<u8>::zeros((792, 612, 3));
        let tensor = layout_preprocess_with_size(&image, 448);
        assert_eq!(tensor.shape(), &[1, 3, 448, 448]);
    }

    #[test]
    fn test_layout_resolution_display() {
        assert_eq!(LayoutResolution::Full.to_string(), "full (640x640)");
        assert_eq!(LayoutResolution::Medium.to_string(), "medium (512x512)");
        assert_eq!(LayoutResolution::Fast.to_string(), "fast (448x448)");
        assert_eq!(
            LayoutResolution::Custom(320).to_string(),
            "custom (320x320)"
        );
    }

    #[test]
    fn test_layout_resolution_sizes() {
        assert_eq!(LayoutResolution::Full.size(), 640);
        assert_eq!(LayoutResolution::Medium.size(), 512);
        assert_eq!(LayoutResolution::Fast.size(), 448);
        assert_eq!(LayoutResolution::Custom(320).size(), 320);
    }

    #[test]
    fn test_layout_resolution_expected_speedup() {
        // Full: (640/640)² = 1.0
        assert!((LayoutResolution::Full.expected_speedup() - 1.0).abs() < 0.01);
        // Medium: (640/512)² ≈ 1.5625
        assert!((LayoutResolution::Medium.expected_speedup() - 1.5625).abs() < 0.01);
        // Fast: (640/448)² ≈ 2.04
        assert!((LayoutResolution::Fast.expected_speedup() - 2.04).abs() < 0.05);
    }

    #[test]
    #[should_panic(expected = "Target size must be at least 224")]
    fn test_layout_preprocess_rejects_too_small() {
        let image = Array3::<u8>::zeros((792, 612, 3));
        let _ = layout_preprocess_with_size(&image, 100);
    }

    #[test]
    #[should_panic(expected = "Target size above 1280")]
    fn test_layout_preprocess_rejects_too_large() {
        let image = Array3::<u8>::zeros((792, 612, 3));
        let _ = layout_preprocess_with_size(&image, 2000);
    }

    #[test]
    fn test_layout_resolution_from_str() {
        // Named presets
        assert_eq!(
            "full".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Full
        );
        assert_eq!(
            "medium".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Medium
        );
        assert_eq!(
            "fast".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Fast
        );

        // Numeric presets
        assert_eq!(
            "640".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Full
        );
        assert_eq!(
            "512".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Medium
        );
        assert_eq!(
            "448".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Fast
        );

        // NxN format
        assert_eq!(
            "640x640".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Full
        );
        assert_eq!(
            "512x512".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Medium
        );
        assert_eq!(
            "448x448".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Fast
        );

        // Custom sizes
        assert_eq!(
            "320".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Custom(320)
        );
        assert_eq!(
            "320x320".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Custom(320)
        );

        // Case insensitive
        assert_eq!(
            "FULL".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Full
        );
        assert_eq!(
            "Medium".parse::<LayoutResolution>().unwrap(),
            LayoutResolution::Medium
        );

        // Invalid
        assert!("invalid".parse::<LayoutResolution>().is_err());
        assert!("abc".parse::<LayoutResolution>().is_err());
    }

    #[test]
    fn test_layout_resolution_roundtrip() {
        // Standard presets
        for res in [
            LayoutResolution::Full,
            LayoutResolution::Medium,
            LayoutResolution::Fast,
        ] {
            let s = res.to_string();
            let parsed: LayoutResolution = s.parse().unwrap();
            assert_eq!(parsed, res, "Roundtrip failed for {res:?}");
        }

        // Custom values - roundtrip uses "custom (NxN)" format
        let custom = LayoutResolution::Custom(320);
        let s = custom.to_string();
        assert_eq!(s, "custom (320x320)");
        let parsed: LayoutResolution = s.parse().unwrap();
        assert_eq!(parsed, custom);
    }
}
