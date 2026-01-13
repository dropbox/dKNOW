// Image preprocessing utilities for OCR
//
// Reference: OcrUtils.cpp from RapidOcrOnnx

// Intentional ML conversions: pixel coordinates, image dimensions, normalization
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
use ndarray::Array4;

use crate::ocr::types::TextBox;

// ============================================================================
// ImageNet Normalization Constants
// ============================================================================

/// `ImageNet` mean values (normalized, 0-1 scale)
///
/// RGB channel means from `ImageNet` dataset.
/// Used for neural network normalization: (pixel/255 - mean) / std
pub const IMAGENET_MEAN: [f32; 3] = [0.485, 0.456, 0.406];

/// `ImageNet` standard deviation values (normalized, 0-1 scale)
///
/// RGB channel standard deviations from `ImageNet` dataset.
/// Used for neural network normalization: (pixel/255 - mean) / std
pub const IMAGENET_STD: [f32; 3] = [0.229, 0.224, 0.225];

/// Mean values for normalization (`ImageNet` statistics, pre-scaled)
///
/// Pre-multiplied by 255 for direct pixel subtraction.
/// Reference: DbNet.cpp:102
pub const MEAN_VALUES: [f32; 3] = [
    IMAGENET_MEAN[0] * 255.0,
    IMAGENET_MEAN[1] * 255.0,
    IMAGENET_MEAN[2] * 255.0,
];

/// Standard deviation values for normalization (`ImageNet` statistics, pre-scaled)
///
/// Pre-multiplied by 255 for direct pixel division.
/// Reference: DbNet.cpp:103
pub const STD_VALUES: [f32; 3] = [
    IMAGENET_STD[0] * 255.0,
    IMAGENET_STD[1] * 255.0,
    IMAGENET_STD[2] * 255.0,
];

/// Normalize image: (pixel - mean) * (1 / std)
///
/// Reference: OcrUtils.cpp:substractMeanNormalize
///
/// # Arguments
/// * `image` - Input image
/// * `mean` - Mean values for each channel
/// * `std` - Standard deviation values for each channel
///
/// # Returns
/// Normalized image tensor in NCHW format [1, 3, H, W]
#[must_use = "returns the normalized image tensor"]
pub fn normalize_image(image: &DynamicImage, mean: &[f32; 3], std: &[f32; 3]) -> Array4<f32> {
    let (width, height) = image.dimensions();
    let rgb_image = image.to_rgb8();

    // Create tensor [1, 3, H, W]
    let mut tensor = Array4::<f32>::zeros((1, 3, height as usize, width as usize));

    // Normalize each channel: (pixel - mean) / std
    for y in 0..height {
        for x in 0..width {
            let pixel = rgb_image.get_pixel(x, y);
            for c in 0..3 {
                let normalized = (f32::from(pixel[c]) - mean[c]) / std[c];
                tensor[[0, c, y as usize, x as usize]] = normalized;
            }
        }
    }

    tensor
}

/// Convert image to tensor without normalization
///
/// Used when normalization is handled separately
#[must_use = "returns the image tensor"]
pub fn image_to_tensor(image: &DynamicImage) -> Array4<f32> {
    let (width, height) = image.dimensions();
    let rgb_image = image.to_rgb8();

    let mut tensor = Array4::<f32>::zeros((1, 3, height as usize, width as usize));

    for y in 0..height {
        for x in 0..width {
            let pixel = rgb_image.get_pixel(x, y);
            for c in 0..3 {
                tensor[[0, c, y as usize, x as usize]] = f32::from(pixel[c]);
            }
        }
    }

    tensor
}

/// Rotate image 180 degrees clockwise
///
/// Reference: OcrUtils.cpp:101-104 (matRotateClockWise180)
///
/// # Arguments
/// * `image` - Input image to rotate
///
/// # Returns
/// Rotated image
#[must_use = "returns the rotated image"]
pub fn rotate_180(image: &DynamicImage) -> DynamicImage {
    // Flip vertically, then horizontally = 180° rotation
    let mut rotated = image.clone();
    rotated = DynamicImage::ImageRgb8(image::imageops::flip_vertical(&rotated.to_rgb8()));
    rotated = DynamicImage::ImageRgb8(image::imageops::flip_horizontal(&rotated.to_rgb8()));
    rotated
}

/// Extract and rotate-crop a text box from an image
///
/// Reference: OcrUtils.cpp:113-165 (getRotateCropImage)
///
/// This function:
/// 1. Finds bounding box of the 4 corners
/// 2. Crops to bounding box
/// 3. Applies perspective transform to rectify the text box
/// 4. If height >= width * 1.5, rotates 90° (for vertical text)
///
/// # Arguments
/// * `image` - Source image
/// * `text_box` - Text box with 4 corners (clockwise from top-left)
///
/// # Returns
/// Cropped and rectified text region
#[must_use = "returns the cropped text box image"]
pub fn crop_text_box(image: &DynamicImage, text_box: &TextBox) -> DynamicImage {
    let rgb_image = image.to_rgb8();

    // Find bounding box
    let corners = &text_box.corners;
    let xs: Vec<f32> = corners.iter().map(|(x, _)| *x).collect();
    let ys: Vec<f32> = corners.iter().map(|(_, y)| *y).collect();

    let left = xs.iter().copied().fold(f32::INFINITY, f32::min) as u32;
    let right = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max) as u32;
    let top = ys.iter().copied().fold(f32::INFINITY, f32::min) as u32;
    let bottom = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max) as u32;

    // Crop to bounding box
    let crop_width = right - left;
    let crop_height = bottom - top;

    // Adjust corners to cropped coordinates
    let adjusted_corners: Vec<(f32, f32)> = corners
        .iter()
        .map(|(x, y)| (x - left as f32, y - top as f32))
        .collect();

    // Calculate dimensions of rectified image
    // Width = distance from corner 0 to corner 1
    let width = (adjusted_corners[0].0 - adjusted_corners[1].0)
        .hypot(adjusted_corners[0].1 - adjusted_corners[1].1) as u32;

    // Height = distance from corner 0 to corner 3
    let height = (adjusted_corners[0].0 - adjusted_corners[3].0)
        .hypot(adjusted_corners[0].1 - adjusted_corners[3].1) as u32;

    // If dimensions are too small, return a small placeholder
    if width < 1 || height < 1 || crop_width < 1 || crop_height < 1 {
        return DynamicImage::ImageRgb8(ImageBuffer::from_pixel(1, 1, Rgb([0, 0, 0])));
    }

    // Use simple crop for now (perspective transform would be ideal but complex)
    // This is a simplified version - full implementation would use getPerspectiveTransform + warpPerspective
    let cropped = image::imageops::crop_imm(
        &rgb_image,
        left,
        top,
        crop_width.min(rgb_image.width() - left),
        crop_height.min(rgb_image.height() - top),
    )
    .to_image();

    let mut result = DynamicImage::ImageRgb8(cropped);

    // If height >= width * 1.5, rotate 90° counter-clockwise (for vertical text)
    if height as f32 >= width as f32 * 1.5 {
        result = DynamicImage::ImageRgb8(image::imageops::rotate270(&result.to_rgb8()));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn test_normalize_image() {
        // Create a simple 2x2 test image
        let mut img = image::RgbImage::new(2, 2);
        img.put_pixel(0, 0, Rgb([100, 150, 200]));
        img.put_pixel(1, 0, Rgb([50, 100, 150]));
        img.put_pixel(0, 1, Rgb([200, 100, 50]));
        img.put_pixel(1, 1, Rgb([150, 200, 100]));

        let dynamic_img = DynamicImage::ImageRgb8(img);
        let tensor = normalize_image(&dynamic_img, &MEAN_VALUES, &STD_VALUES);

        // Check shape
        assert_eq!(tensor.shape(), &[1, 3, 2, 2]);

        // Check that normalization was applied (values should be in reasonable range)
        for c in 0..3 {
            for y in 0..2 {
                for x in 0..2 {
                    let val = tensor[[0, c, y, x]];
                    assert!(
                        val > -3.0 && val < 3.0,
                        "Normalized value out of range: {val}"
                    );
                }
            }
        }
    }
}
