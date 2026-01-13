// Intentional ML conversions: pixel coordinates, image dimensions, interpolation
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

/// PIL-compatible bilinear image resize
///
/// Implements PIL/Pillow's exact bilinear interpolation algorithm to match
/// preprocessing outputs byte-for-byte.
///
/// **Why this exists:**
/// The Rust `image` crate's bilinear interpolation produces slightly different
/// results than PIL (±1/255 at 3.7% of pixels). This small difference amplifies
/// 1000x through the ONNX model, causing validation failures.
///
/// **Algorithm (from PIL/Pillow libImaging/Resample.c):**
/// PIL uses a two-pass separable resampling with variable filter support:
/// 1. `filterscale = max(scale, 1.0)` - widen filter when downsampling
/// 2. `support = filter.support * filterscale` - bilinear has support=1.0
/// 3. `center = in0 + (out + 0.5) * scale` - center coordinate in input space
/// 4. `xmin = int(center - support + 0.5)`, `xmax = int(center + support + 0.5)` - contributing pixels
/// 5. Apply bilinear filter: `weight = max(0, 1.0 - abs(distance))`
/// 6. Accumulate: `sum(pixel[i] * weight[i]) / sum(weight[i])`
/// 7. Round result: `int(value + 0.5)`
///
/// **Key difference from simple bilinear:**
/// When downsampling (e.g., 792→640, scale=1.2375), filterscale=1.2375, support=1.2375
/// This means 3-4 input pixels contribute (not just 2), with bilinear weights applied
///
/// **References:**
/// - PIL/Pillow Imaging library (C implementation)
/// - <https://github.com/python-pillow/Pillow/blob/main/src/libImaging/Resample.c>
/// - Investigation: N=180 found simple bilinear matches upsampling but not downsampling
use ndarray::Array3;

/// Bilinear filter kernel (PIL's `bilinear_filter` function)
/// Returns weight for pixel at distance `x` from center
/// filter(x) = max(0, 1.0 - abs(x))
#[inline]
fn bilinear_filter(x: f32) -> f32 {
    let abs_x = x.abs();
    if abs_x < 1.0 {
        1.0 - abs_x
    } else {
        0.0
    }
}

/// Resize RGB image using PIL-compatible bilinear interpolation
///
/// # Arguments
/// * `input` - Input image in HWC format (height, width, channels=3), uint8 [0, 255]
/// * `out_height` - Output height
/// * `out_width` - Output width
///
/// # Returns
/// Resized image in HWC format, uint8 [0, 255]
#[must_use = "returns the resized image array"]
pub fn pil_resize_bilinear(input: &Array3<u8>, out_height: usize, out_width: usize) -> Array3<u8> {
    let (in_height, in_width, channels) = input.dim();
    assert_eq!(channels, 3, "Input must have 3 channels (RGB)");

    // Two-pass separable filtering: horizontal then vertical
    // PIL uses uint8 intermediate buffer (not float) to match PIL exactly
    // Pass 1: Horizontal resampling
    let scale_x = in_width as f32 / out_width as f32;
    let filterscale_x = scale_x.max(1.0);
    let support_x = filterscale_x; // bilinear filter has support = 1.0

    let mut temp = Array3::<u8>::zeros((in_height, out_width, channels));

    for y in 0..in_height {
        for out_x in 0..out_width {
            // Center position in input space
            let center_x = (out_x as f32 + 0.5) * scale_x;

            // Contributing pixel range
            let xmin = ((center_x - support_x + 0.5).floor() as i32).max(0) as usize;
            let xmax = ((center_x + support_x + 0.5).floor() as i32).min(in_width as i32) as usize;

            for c in 0..channels {
                let mut sum = 0.0f32;
                let mut wsum = 0.0f32;

                for x in xmin..xmax {
                    let distance = (x as f32 + 0.5 - center_x) / filterscale_x;
                    let weight = bilinear_filter(distance);

                    sum += f32::from(input[[y, x, c]]) * weight;
                    wsum += weight;
                }

                // Normalize, round, and clamp (PIL uses uint8 intermediate)
                let value = if wsum > 0.0 { sum / wsum } else { 0.0 };
                let rounded = (value + 0.5).floor();
                temp[[y, out_x, c]] = rounded.clamp(0.0, 255.0) as u8;
            }
        }
    }

    // Pass 2: Vertical resampling
    let scale_y = in_height as f32 / out_height as f32;
    let filterscale_y = scale_y.max(1.0);
    let support_y = filterscale_y; // bilinear filter has support = 1.0

    let mut output = Array3::<u8>::zeros((out_height, out_width, channels));

    for out_y in 0..out_height {
        for x in 0..out_width {
            // Center position in input space
            let center_y = (out_y as f32 + 0.5) * scale_y;

            // Contributing pixel range
            let ymin = ((center_y - support_y + 0.5).floor() as i32).max(0) as usize;
            let ymax = ((center_y + support_y + 0.5).floor() as i32).min(in_height as i32) as usize;

            for c in 0..channels {
                let mut sum = 0.0f32;
                let mut wsum = 0.0f32;

                for y in ymin..ymax {
                    let distance = (y as f32 + 0.5 - center_y) / filterscale_y;
                    let weight = bilinear_filter(distance);

                    sum += f32::from(temp[[y, x, c]]) * weight;
                    wsum += weight;
                }

                // Normalize and round (PIL uses fixed-point rounding)
                // PIL: ss0 = 1 << (PRECISION_BITS - 1); ... clip8(ss0 >> PRECISION_BITS)
                // Equivalent in float: round(value) which is same as (value + 0.5).floor()
                // But we need to clamp to [0, 255] range
                let value = if wsum > 0.0 { sum / wsum } else { 0.0 };
                let rounded = (value + 0.5).floor();
                output[[out_y, x, c]] = rounded.clamp(0.0, 255.0) as u8;
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

    #[test]
    fn test_pil_resize_shape() {
        let input = Array3::<u8>::zeros((792, 612, 3));
        let output = pil_resize_bilinear(&input, 640, 640);
        assert_eq!(output.shape(), &[640, 640, 3]);
    }

    #[test]
    fn test_pil_resize_values() {
        // Create a simple test image
        let mut input = Array3::<u8>::zeros((4, 4, 3));
        for i in 0..4 {
            for j in 0..4 {
                let val = ((i * 4 + j) * 16) as u8;
                input[[i, j, 0]] = val;
                input[[i, j, 1]] = val;
                input[[i, j, 2]] = val;
            }
        }

        // Resize to 8x8
        let output = pil_resize_bilinear(&input, 8, 8);

        // Check output shape
        assert_eq!(output.shape(), &[8, 8, 3]);
    }

    #[test]
    fn test_pil_resize_identity() {
        // Resize to same size should be identity (approximately)
        let mut input = Array3::<u8>::zeros((10, 10, 3));
        for i in 0..10 {
            for j in 0..10 {
                input[[i, j, 0]] = (i * 10 + j) as u8;
                input[[i, j, 1]] = (i * 10 + j) as u8;
                input[[i, j, 2]] = (i * 10 + j) as u8;
            }
        }

        let output = pil_resize_bilinear(&input, 10, 10);

        // Should be very close to input
        for i in 0..10 {
            for j in 0..10 {
                for c in 0..3 {
                    let diff = (output[[i, j, c]] as i32 - input[[i, j, c]] as i32).abs();
                    assert!(diff <= 1, "Pixel ({i}, {j}, {c}) diff too large: {diff}");
                }
            }
        }
    }
}
