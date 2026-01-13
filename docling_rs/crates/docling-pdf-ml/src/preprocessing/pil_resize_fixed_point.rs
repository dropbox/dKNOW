// Intentional ML conversions: fixed-point arithmetic, pixel coordinates, image dimensions
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

/// PIL-compatible bilinear image resize with FIXED-POINT arithmetic
///
/// This implementation matches PIL/Pillow's exact fixed-point arithmetic
/// to achieve 100% pixel-perfect match.
///
/// **Key implementation details:**
/// 1. **Weight normalization (CRITICAL):** Weights are normalized to sum to 1.0 BEFORE converting to fixed-point
///    - Discovered N=186: Missing normalization caused 181/1.2M pixels (0.0147%) to differ by ±1
///    - PIL normalizes in `precompute_coeffs()` before conversion to fixed-point
///    - Without normalization, wsum ≠ 2^22 when downsampling (filterscale > 1.0)
///
/// 2. **f64 precision:** Uses f64 (not f32) for all weight calculations
///    - Discovered N=184: f32 caused 333→181 pixel errors (45% improvement)
///    - PIL Python uses f64 internally, must match exactly
///
/// 3. **Fixed-point arithmetic:** Uses 22-bit fixed-point (not float) for accumulation
///    - ss = 2^21 + sum(pixel\[i\] * `weight_fixed`\[i\])
///    - result = ss >> 22
///
/// **PIL's algorithm (from libImaging/Resample.c):**
/// ```c
/// #define PRECISION_BITS 22
///
/// // 1. Compute and normalize weights (precompute_coeffs)
/// for (x = 0; x < xmax; x++) {
///     k[x] = filter((x - center) * scale);
///     ww += k[x];
/// }
/// for (x = 0; x < xmax; x++) {
///     k[x] /= ww;  // Normalize to sum to 1.0
/// }
///
/// // 2. Convert to fixed-point (normalize_coeffs_8bpc)
/// kk[x] = (int)(k[x] * (1 << PRECISION_BITS) + 0.5);
///
/// // 3. Accumulate (ImagingResampleHorizontal_8bpc)
/// ss = 1 << (PRECISION_BITS - 1);  // Start with 2^21 for rounding
/// for (x = 0; x < xmax; x++) {
///     ss += pixel[x] * kk[x];
/// }
/// result = clip8(ss >> PRECISION_BITS);  // Shift right by 22
/// ```
///
/// **References:**
/// - PIL/Pillow libImaging/Resample.c (`precompute_coeffs`, `normalize_coeffs_8bpc`)
/// - N=186 report: Weight normalization fix (181 pixels → 0 pixels)
/// - N=184 report: f64 precision fix (333 pixels → 181 pixels)
/// - N=183 report: Root cause - uint8 off by ±1
use ndarray::Array3;

/// PIL's precision: 22 bits (32 - 8 - 2)
/// This gives sufficient precision for 8-bit image values
const PRECISION_BITS: u32 = 22;

/// Bilinear filter kernel (same as float version)
/// Returns weight for pixel at distance `x` from center
/// filter(x) = max(0, 1.0 - abs(x))
/// **CRITICAL:** Uses f64 (not f32) to match PIL's double precision
#[inline]
fn bilinear_filter(x: f64) -> f64 {
    let abs_x = x.abs();
    if abs_x < 1.0 {
        1.0 - abs_x
    } else {
        0.0
    }
}

/// Clip value to [0, 255] range
#[inline]
fn clip8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

/// Pre-computed weight information for a single output position
struct WeightInfo {
    /// First contributing input pixel index
    start_idx: usize,
    /// Number of contributing pixels
    count: usize,
    /// Normalized weights (sum to 1.0) - stored for debugging/verification
    #[allow(dead_code, reason = "debugging field to verify weight normalization")]
    weights: Vec<f64>,
    /// Fixed-point weights (pre-converted for fast accumulation)
    weights_fixed: Vec<i64>,
}

/// Pre-compute weights for all output positions in one dimension
///
/// This function computes weights ONCE per output position, eliminating
/// redundant computation across channels and rows/columns.
///
/// # Arguments
/// * `input_size` - Input dimension size (width or height)
/// * `output_size` - Output dimension size (width or height)
///
/// # Returns
/// Vector of `WeightInfo`, one per output position, indexed by output coordinate
fn precompute_weights(input_size: usize, output_size: usize) -> Vec<WeightInfo> {
    let scale = input_size as f64 / output_size as f64;
    let filterscale = scale.max(1.0);
    let support = filterscale; // bilinear filter has support = 1.0

    let mut weights_table = Vec::with_capacity(output_size);

    for out_pos in 0..output_size {
        // Center position in input space
        let center = (out_pos as f64 + 0.5) * scale;

        // Contributing pixel range
        let min_idx = ((center - support + 0.5).floor() as i32).max(0) as usize;
        let max_idx = ((center + support + 0.5).floor() as i32).min(input_size as i32) as usize;

        // Step 1: Compute raw weights
        let count = max_idx - min_idx;
        let mut weights = Vec::with_capacity(count);
        let mut wsum = 0.0_f64;

        for idx in min_idx..max_idx {
            let distance = (idx as f64 + 0.5 - center) / filterscale;
            let weight = bilinear_filter(distance);
            weights.push(weight);
            wsum += weight;
        }

        // Step 2: Normalize weights to sum to 1.0
        if wsum > 0.0 {
            for w in &mut weights {
                *w /= wsum;
            }
        }

        // Step 3: Pre-convert to fixed-point for fast accumulation
        let weights_fixed: Vec<i64> = weights
            .iter()
            .map(|&w| w.mul_add(f64::from(1_u32 << PRECISION_BITS), 0.5) as i64)
            .collect();

        weights_table.push(WeightInfo {
            start_idx: min_idx,
            count,
            weights,
            weights_fixed,
        });
    }

    weights_table
}

/// Resize RGB image using PIL-compatible bilinear interpolation with FIXED-POINT arithmetic
///
/// # Arguments
/// * `input` - Input image in HWC format (height, width, channels=3), uint8 [0, 255]
/// * `out_height` - Output height
/// * `out_width` - Output width
///
/// # Returns
/// Resized image in HWC format, uint8 [0, 255]
#[must_use = "returns the resized image array using fixed-point math"]
pub fn pil_resize_bilinear_fixed_point(
    input: &Array3<u8>,
    out_height: usize,
    out_width: usize,
) -> Array3<u8> {
    use std::time::Instant;

    let (in_height, in_width, channels) = input.dim();
    assert_eq!(channels, 3, "Input must have 3 channels (RGB)");

    // Two-pass separable filtering: horizontal then vertical
    // PIL uses uint8 intermediate buffer (not float) to match PIL exactly

    // Pass 1: Horizontal resampling with fixed-point arithmetic
    // **CRITICAL:** Use f64 (not f32) to match PIL's double precision
    let horizontal_start = Instant::now();

    // Pre-compute weights once for all output columns (OPTIMIZATION: N=493)
    let horizontal_weights = precompute_weights(in_width, out_width);

    let mut temp = Array3::<u8>::zeros((in_height, out_width, channels));

    for y in 0..in_height {
        for out_x in 0..out_width {
            // Look up pre-computed weights for this output column
            let weight_info = &horizontal_weights[out_x];
            let xmin = weight_info.start_idx;
            let xmax = xmin + weight_info.count;

            for c in 0..channels {
                // PIL algorithm (from libImaging/Resample.c):
                // 1. Compute weights and normalize to sum to 1.0 ← PRE-COMPUTED
                // 2. Convert normalized weights to fixed-point ← PRE-COMPUTED
                // 3. Accumulate: ss = 2^21 + sum(pixel[i] * weight_fixed[i])
                // 4. Result: ss >> PRECISION_BITS

                // Start with 2^(PRECISION_BITS-1) for rounding
                let mut ss = 1_i64 << (PRECISION_BITS - 1);

                // Accumulate using pre-computed fixed-point weights
                for (i, x) in (xmin..xmax).enumerate() {
                    ss += i64::from(input[[y, x, c]]) * weight_info.weights_fixed[i];
                }

                // Shift right by PRECISION_BITS (equivalent to divide by 2^22)
                let value = ss >> PRECISION_BITS;
                temp[[y, out_x, c]] = clip8(value as i32);
            }
        }
    }

    let _horizontal_time = horizontal_start.elapsed();
    #[cfg(feature = "debug-profiling")]
    {
        if std::env::var("PROFILE_PREPROCESS").is_ok() {
            log::warn!(
                "[PROFILE_PREPROCESS]   - Horizontal pass: {:.2} ms",
                _horizontal_time.as_secs_f64() * 1000.0
            );
        }
    }

    // Pass 2: Vertical resampling with fixed-point arithmetic
    // **CRITICAL:** Use f64 (not f32) to match PIL's double precision
    let vertical_start = Instant::now();

    // Pre-compute weights once for all output rows (OPTIMIZATION: N=493)
    let vertical_weights = precompute_weights(in_height, out_height);

    let mut output = Array3::<u8>::zeros((out_height, out_width, channels));

    for out_y in 0..out_height {
        // Look up pre-computed weights for this output row
        let weight_info = &vertical_weights[out_y];
        let ymin = weight_info.start_idx;
        let ymax = ymin + weight_info.count;

        for x in 0..out_width {
            for c in 0..channels {
                // PIL algorithm (same as horizontal pass)
                // 1. Compute weights and normalize to sum to 1.0 ← PRE-COMPUTED
                // 2. Convert normalized weights to fixed-point ← PRE-COMPUTED
                // 3. Accumulate: ss = 2^21 + sum(pixel[i] * weight_fixed[i])
                // 4. Result: ss >> PRECISION_BITS

                // Start with 2^(PRECISION_BITS-1) for rounding
                let mut ss = 1_i64 << (PRECISION_BITS - 1);

                // Accumulate using pre-computed fixed-point weights
                for (i, y) in (ymin..ymax).enumerate() {
                    ss += i64::from(temp[[y, x, c]]) * weight_info.weights_fixed[i];
                }

                // Shift right by PRECISION_BITS (equivalent to divide by 2^22)
                let value = ss >> PRECISION_BITS;
                output[[out_y, x, c]] = clip8(value as i32);
            }
        }
    }

    let _vertical_time = vertical_start.elapsed();
    #[cfg(feature = "debug-profiling")]
    {
        if std::env::var("PROFILE_PREPROCESS").is_ok() {
            log::warn!(
                "[PROFILE_PREPROCESS]   - Vertical pass: {:.2} ms",
                _vertical_time.as_secs_f64() * 1000.0
            );
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

    #[test]
    fn test_fixed_point_resize_shape() {
        let input = Array3::<u8>::zeros((792, 612, 3));
        let output = pil_resize_bilinear_fixed_point(&input, 640, 640);
        assert_eq!(output.shape(), &[640, 640, 3]);
    }

    #[test]
    fn test_fixed_point_simple_upsampling() {
        // Test case from N=181: 1×2 → 1×5
        let mut input = Array3::<u8>::zeros((1, 2, 3));
        input[[0, 0, 0]] = 100;
        input[[0, 1, 0]] = 200;
        input[[0, 0, 1]] = 100;
        input[[0, 1, 1]] = 200;
        input[[0, 0, 2]] = 100;
        input[[0, 1, 2]] = 200;

        let output = pil_resize_bilinear_fixed_point(&input, 1, 5);

        // Expected (from PIL): [100, 110, 150, 190, 200]
        assert_eq!(output[[0, 0, 0]], 100);
        assert_eq!(output[[0, 1, 0]], 110);
        assert_eq!(output[[0, 2, 0]], 150);
        assert_eq!(output[[0, 3, 0]], 190);
        assert_eq!(output[[0, 4, 0]], 200);
    }

    #[test]
    fn test_fixed_point_downsampling() {
        // Test case: 10×10 → 4×4 (verified with PIL)
        let mut input = Array3::<u8>::zeros((10, 10, 3));
        for i in 0..10 {
            for j in 0..10 {
                let val = (i * 10 + j) as u8;
                input[[i, j, 0]] = val;
                input[[i, j, 1]] = val;
                input[[i, j, 2]] = val;
            }
        }

        let output = pil_resize_bilinear_fixed_point(&input, 4, 4);

        // Expected (from PIL - verified with generate_correct_test_cases.py):
        // [[11 13 16 18]
        //  [33 35 38 40]
        //  [59 61 64 66]
        //  [81 83 86 88]]
        assert_eq!(output[[0, 0, 0]], 11);
        assert_eq!(output[[0, 1, 0]], 13);
        assert_eq!(output[[0, 2, 0]], 16);
        assert_eq!(output[[0, 3, 0]], 18);

        assert_eq!(output[[1, 0, 0]], 33);
        assert_eq!(output[[1, 1, 0]], 35);
        assert_eq!(output[[1, 2, 0]], 38);
        assert_eq!(output[[1, 3, 0]], 40);

        assert_eq!(output[[2, 0, 0]], 59);
        assert_eq!(output[[2, 1, 0]], 61);
        assert_eq!(output[[2, 2, 0]], 64);
        assert_eq!(output[[2, 3, 0]], 66);

        assert_eq!(output[[3, 0, 0]], 81);
        assert_eq!(output[[3, 1, 0]], 83);
        assert_eq!(output[[3, 2, 0]], 86);
        assert_eq!(output[[3, 3, 0]], 88);
    }
}
