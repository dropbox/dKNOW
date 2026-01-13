// Intentional ML conversions: tensor indices, image dimensions
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

use ndarray::{Array3, Array4};

// ============================================================================
// TableFormer Constants
// ============================================================================

/// `TableFormer` expects 448×448 pixel input.
///
/// This is the standard input resolution for Microsoft Table Transformer models.
/// Both `TableFormer` preprocessing and inference use this size.
pub const TABLEFORMER_INPUT_SIZE: usize = 448;

/// Bilinear resize for float32 HWC images (pure Rust implementation)
///
/// This replaces `OpenCV`'s `cv2.resize(INTER_LINEAR)` for normalized float32 data.
/// Bilinear interpolation: for each output pixel, find 4 nearest source pixels
/// and interpolate using their distances as weights.
fn bilinear_resize_f32(src: &Array3<f32>, new_height: usize, new_width: usize) -> Array3<f32> {
    let (src_height, src_width, channels) = src.dim();
    let mut dst = Array3::<f32>::zeros((new_height, new_width, channels));

    let scale_y = src_height as f32 / new_height as f32;
    let scale_x = src_width as f32 / new_width as f32;

    for y in 0..new_height {
        for x in 0..new_width {
            // Map output (x, y) to source coordinates
            // OpenCV uses: src_x = (dst_x + 0.5) * scale - 0.5
            let src_y = (y as f32 + 0.5).mul_add(scale_y, -0.5);
            let src_x = (x as f32 + 0.5).mul_add(scale_x, -0.5);

            // Get integer and fractional parts
            let y0 = src_y.floor() as i32;
            let x0 = src_x.floor() as i32;
            let y1 = y0 + 1;
            let x1 = x0 + 1;

            let fy = src_y - y0 as f32;
            let fx = src_x - x0 as f32;

            // Clamp to valid range
            let y0 = y0.clamp(0, src_height as i32 - 1) as usize;
            let y1 = y1.clamp(0, src_height as i32 - 1) as usize;
            let x0 = x0.clamp(0, src_width as i32 - 1) as usize;
            let x1 = x1.clamp(0, src_width as i32 - 1) as usize;

            // Bilinear interpolation for each channel
            for c in 0..channels {
                let v00 = src[[y0, x0, c]];
                let v01 = src[[y0, x1, c]];
                let v10 = src[[y1, x0, c]];
                let v11 = src[[y1, x1, c]];

                // Horizontal interpolation
                let v0 = (1.0 - fx).mul_add(v00, v01 * fx);
                let v1 = (1.0 - fx).mul_add(v10, v11 * fx);

                // Vertical interpolation
                dst[[y, x, c]] = (1.0 - fy).mul_add(v0, v1 * fy);
            }
        }
    }

    dst
}

/// `TableFormer` Preprocessing (Pure Rust)
///
/// Transforms raw table crop into tensor for table structure model.
///
/// Pipeline (from `TFPredictor._prepare_image()`):
/// 1. Normalize: (pixel - 255*mean) / std (per-channel, on uint8 values)
/// 2. Resize to 448x448 (bilinear interpolation on normalized float32 values)
/// 3. Transpose HWC → CWH (channels, width, height) - NOTE: Width-height swap!
/// 4. Divide by 255
/// 5. Add batch dimension
///
/// Input: Raw table crop (H, W, 3) uint8 [0, 255]
/// Output: Preprocessed tensor (1, 3, 448, 448) float32 (normalized)
///
/// ## Implementation (N=3453+)
///
/// Uses custom bilinear resize for f32 values - removes `OpenCV` dependency.
#[must_use = "returns the preprocessed tensor for TableFormer"]
pub fn tableformer_preprocess(image: &Array3<u8>) -> Array4<f32> {
    let (height, width, channels) = image.dim();
    assert_eq!(channels, 3, "Image must have 3 channels (RGB)");

    // TableFormer normalization parameters (from config)
    let mean = [0.942_478_51_f32, 0.942_546_7_f32, 0.942_926_1_f32];
    let std = [0.179_109_56_f32, 0.179_404_03_f32, 0.179_316_63_f32];

    // Step 1: Normalize (applied to uint8 values, per-channel)
    // CRITICAL: Python functional.normalize for numpy arrays uses: (pixel - 255.0 * mean) / std
    let mut normalized = Array3::<f32>::zeros((height, width, channels));
    for y in 0..height {
        for x in 0..width {
            for c in 0..channels {
                let pixel_val = f32::from(image[[y, x, c]]);
                normalized[[y, x, c]] = (pixel_val - mean[c].mul_add(255.0, 0.0)) / std[c];
            }
        }
    }

    // Step 2: Resize to TABLEFORMER_INPUT_SIZE (bilinear interpolation on normalized float values)
    let resized_normalized =
        bilinear_resize_f32(&normalized, TABLEFORMER_INPUT_SIZE, TABLEFORMER_INPUT_SIZE);

    // Step 3: Transpose HWC → CWH (channels, width, height)
    // CRITICAL: This is (2, 1, 0) which gives (C, W, H), NOT (C, H, W)!
    let mut transposed =
        Array3::<f32>::zeros((channels, TABLEFORMER_INPUT_SIZE, TABLEFORMER_INPUT_SIZE));
    for c in 0..channels {
        for h in 0..TABLEFORMER_INPUT_SIZE {
            for w in 0..TABLEFORMER_INPUT_SIZE {
                // CWH indexing: transposed[c, w, h] = resized[h, w, c]
                transposed[[c, w, h]] = resized_normalized[[h, w, c]];
            }
        }
    }

    // Step 4: Divide by 255
    transposed /= 255.0;

    // Step 5: Add batch dimension
    let mut batch =
        Array4::<f32>::zeros((1, channels, TABLEFORMER_INPUT_SIZE, TABLEFORMER_INPUT_SIZE));
    for c in 0..channels {
        for h in 0..TABLEFORMER_INPUT_SIZE {
            for w in 0..TABLEFORMER_INPUT_SIZE {
                batch[[0, c, h, w]] = transposed[[c, h, w]];
            }
        }
    }

    batch
}

/// TableFormer Preprocessing (OpenCV - Legacy)
///
/// Transforms raw table crop into tensor for table structure model.
///
/// Pipeline (from `TFPredictor._prepare_image()`):
/// 1. Normalize: (pixel - 255*mean) / std (per-channel, on uint8 values)
/// 2. Resize to 448x448 (OpenCV bilinear, on normalized float32 values)
/// 3. Transpose HWC → CWH (channels, width, height) - NOTE: Width-height swap!
/// 4. Divide by 255
/// 5. Add batch dimension
///
/// Input: Raw table crop (H, W, 3) uint8 [0, 255]
/// Output: Preprocessed tensor (1, 3, 448, 448) float32 (normalized)
///
/// NOTE: Legacy function kept for comparison. Use tableformer_preprocess() instead.
#[cfg(feature = "opencv-preprocessing")]
#[must_use = "returns the preprocessed tensor for TableFormer (OpenCV variant)"]
pub fn tableformer_preprocess_opencv(image: &Array3<u8>) -> Array4<f32> {
    let (height, width, channels) = image.dim();
    assert_eq!(channels, 3, "Image must have 3 channels (RGB)");

    // TableFormer normalization parameters (from config)
    let mean = [0.942_478_51_f32, 0.942_546_7_f32, 0.942_926_1_f32];
    let std = [0.179_109_56_f32, 0.179_404_03_f32, 0.179_316_63_f32];

    // Step 1: Normalize (applied to uint8 values, per-channel)
    // CRITICAL: Python functional.normalize for numpy arrays uses: (pixel - 255.0 * mean) / std
    // See: docling_ibm_models/tableformer/data_management/functional.py:normalize()
    // Formula: (pixel - 255 * mean) / std
    let mut normalized = Array3::<f32>::zeros((height, width, channels));
    for y in 0..height {
        for x in 0..width {
            for c in 0..channels {
                let pixel_val = image[[y, x, c]] as f32;
                normalized[[y, x, c]] = (pixel_val - mean[c].mul_add(255.0, 0.0)) / std[c];
            }
        }
    }

    // Step 2: Resize to TABLEFORMER_INPUT_SIZE (bilinear interpolation on normalized float values)
    // Use OpenCV for float32 resize (matches Python cv2.resize exactly)
    use opencv::{core, imgproc, prelude::*};

    // Convert normalized array to OpenCV Mat (Vec3f for float32 RGB)
    let src_vec: Vec<core::Vec3f> = (0..height * width)
        .map(|i| {
            let y = i / width;
            let x = i % width;
            core::Vec3f::from([
                normalized[[y, x, 0]],
                normalized[[y, x, 1]],
                normalized[[y, x, 2]],
            ])
        })
        .collect();

    // Create Mat from Vec3f data
    let src_mat = core::Mat::new_rows_cols_with_data(height as i32, width as i32, &src_vec)
        .expect("Failed to create source Mat");

    // Resize with bilinear interpolation (cv::INTER_LINEAR)
    let mut dst_mat = core::Mat::default();
    #[allow(clippy::cast_possible_wrap)]
    let target_i32 = TABLEFORMER_INPUT_SIZE as i32;
    imgproc::resize(
        &src_mat,
        &mut dst_mat,
        core::Size::new(target_i32, target_i32),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )
    .expect("Failed to resize image");

    // Convert Mat back to ndarray
    let resized_data: Vec<f32> = dst_mat
        .data_typed::<core::Vec3f>()
        .expect("Failed to get Mat data")
        .iter()
        .flat_map(|v| vec![v[0], v[1], v[2]])
        .collect();

    let mut resized_normalized =
        Array3::<f32>::zeros((TABLEFORMER_INPUT_SIZE, TABLEFORMER_INPUT_SIZE, channels));
    for y in 0..TABLEFORMER_INPUT_SIZE {
        for x in 0..TABLEFORMER_INPUT_SIZE {
            for c in 0..channels {
                let idx = (y * TABLEFORMER_INPUT_SIZE + x) * 3 + c;
                resized_normalized[[y, x, c]] = resized_data[idx];
            }
        }
    }

    // Step 3: Transpose HWC → CWH (channels, width, height)
    // CRITICAL: This is (2, 1, 0) which gives (C, W, H), NOT (C, H, W)!
    // Width and height are swapped!
    let mut transposed =
        Array3::<f32>::zeros((channels, TABLEFORMER_INPUT_SIZE, TABLEFORMER_INPUT_SIZE));
    for c in 0..channels {
        for h in 0..TABLEFORMER_INPUT_SIZE {
            for w in 0..TABLEFORMER_INPUT_SIZE {
                // CWH indexing: transposed[c, w, h] = resized[h, w, c]
                transposed[[c, w, h]] = resized_normalized[[h, w, c]];
            }
        }
    }

    // Step 4: Divide by 255
    transposed /= 255.0;

    // Step 5: Add batch dimension
    let mut batch =
        Array4::<f32>::zeros((1, channels, TABLEFORMER_INPUT_SIZE, TABLEFORMER_INPUT_SIZE));
    for c in 0..channels {
        for h in 0..TABLEFORMER_INPUT_SIZE {
            for w in 0..TABLEFORMER_INPUT_SIZE {
                batch[[0, c, h, w]] = transposed[[c, h, w]];
            }
        }
    }

    batch
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

    #[test]
    fn test_tableformer_preprocess_shape() {
        // Create a dummy 225x418 RGB image (like extracted table)
        let image = Array3::<u8>::zeros((225, 418, 3));
        let tensor = tableformer_preprocess(&image);

        // Check output shape
        assert_eq!(tensor.shape(), &[1, 3, 448, 448]);
    }

    #[test]
    fn test_tableformer_preprocess_white_image() {
        // Create a white image (like document background)
        let image = Array3::<u8>::from_elem((225, 418, 3), 255);
        let tensor = tableformer_preprocess(&image);

        // White pixels (255) with TableFormer normalization:
        // Step 1: (255 - 255*0.942) / 0.179 ≈ 0.324/0.179 ≈ 1.81
        // Step 4: / 255 ≈ 0.0071
        // So we expect small positive values for white pixels

        // Check that preprocessing ran (not all zeros)
        let sum: f32 = tensor.iter().sum();
        assert!(sum.abs() > 0.1, "Tensor should not be all zeros");
    }

    #[test]
    fn test_bilinear_resize_identity() {
        // Test that bilinear resize of a constant image stays constant
        let src = Array3::<f32>::from_elem((100, 100, 3), 0.5);
        let dst = bilinear_resize_f32(&src, 50, 50);

        // All values should be 0.5 (constant input → constant output)
        for y in 0..50 {
            for x in 0..50 {
                for c in 0..3 {
                    assert!(
                        (dst[[y, x, c]] - 0.5).abs() < 0.01,
                        "Expected 0.5, got {} at ({}, {}, {})",
                        dst[[y, x, c]],
                        y,
                        x,
                        c
                    );
                }
            }
        }
    }
}
