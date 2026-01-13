/// Test new PIL-compatible resize implementation with variable filter support
///
/// This validates that our Rust implementation matches PIL's exact algorithm,
/// including the variable filter support for downsampling.
use docling_pdf_ml::preprocessing::pil_resize::pil_resize_bilinear;
use ndarray::Array3;
use std::fs::File;

fn load_npy_u8_3d(path: &str) -> Array3<u8> {
    let file = File::open(path).unwrap();
    let npy = npyz::NpyFile::new(file).unwrap();
    let shape_vec: Vec<u64> = npy.shape().to_vec();
    assert_eq!(shape_vec.len(), 3, "Expected 3D array");

    let data: Vec<u8> = npy.into_vec().unwrap();
    Array3::from_shape_vec(
        (
            shape_vec[0] as usize,
            shape_vec[1] as usize,
            shape_vec[2] as usize,
        ),
        data,
    )
    .unwrap()
}

#[allow(dead_code)]
fn load_npy_f32_3d(path: &str) -> Array3<f32> {
    let file = File::open(path).unwrap();
    let npy = npyz::NpyFile::new(file).unwrap();
    let shape_vec: Vec<u64> = npy.shape().to_vec();
    assert_eq!(shape_vec.len(), 3, "Expected 3D array");

    let data: Vec<f32> = npy.into_vec().unwrap();
    Array3::from_shape_vec(
        (
            shape_vec[0] as usize,
            shape_vec[1] as usize,
            shape_vec[2] as usize,
        ),
        data,
    )
    .unwrap()
}

#[test]
#[ignore] // Requires test data files in /tmp that are not committed
fn test_simple_upsampling() {
    println!("\n=== TEST 1: Simple Upsampling (1x2 -> 1x5) ===");

    // Load test input
    let input = load_npy_u8_3d("/tmp/test_upsampling_input.npy");
    println!("Input shape: {:?}", input.shape());
    println!("Input: {:?}", input.slice(ndarray::s![0, .., 0]));

    // Run Rust resize
    let output = pil_resize_bilinear(&input, 1, 5);
    println!("Output shape: {:?}", output.shape());
    println!("Output: {:?}", output.slice(ndarray::s![0, .., 0]));

    // Expected from PIL: [100, 110, 150, 190, 200]
    let expected = [100u8, 110, 150, 190, 200];

    for (i, &expected_val) in expected.iter().enumerate() {
        let rust_val = output[[0, i, 0]];
        assert_eq!(
            rust_val, expected_val,
            "Pixel {} mismatch: Rust={}, PIL={}",
            i, rust_val, expected_val
        );
    }

    println!("✓ PASS: Upsampling matches PIL exactly");
}

#[test]
#[ignore] // Requires test data files in /tmp that are not committed
fn test_downsampling() {
    println!("\n=== TEST 2: Downsampling (10x10 -> 4x4) ===");

    // Load test input
    let input = load_npy_u8_3d("/tmp/test_downsampling_input.npy");
    println!("Input shape: {:?}", input.shape());

    // Run Rust resize
    let output = pil_resize_bilinear(&input, 4, 4);
    println!("Output shape: {:?}", output.shape());
    println!("Rust output:\n{:?}", output.slice(ndarray::s![.., .., 0]));

    // Expected from PIL:
    // [[ 22  26  32  36]
    //  [ 67  71  77  81]
    //  [117 121 127 131]
    //  [162 166 172 176]]
    let expected = [
        vec![22u8, 26, 32, 36],
        vec![67, 71, 77, 81],
        vec![117, 121, 127, 131],
        vec![162, 166, 172, 176],
    ];

    let mut max_diff = 0i32;
    let mut total_diff = 0i32;
    let mut diff_count = 0;

    for y in 0..4 {
        for x in 0..4 {
            let rust_val = output[[y, x, 0]];
            let pil_val = expected[y][x];
            let diff = (rust_val as i32 - pil_val as i32).abs();

            if diff > 0 {
                println!(
                    "Pixel ({}, {}) diff: Rust={}, PIL={}, diff={}",
                    y, x, rust_val, pil_val, diff
                );
                diff_count += 1;
                total_diff += diff;
                max_diff = max_diff.max(diff);
            }
        }
    }

    if diff_count > 0 {
        println!(
            "⚠ {} pixels differ, max_diff={}, avg_diff={:.2}",
            diff_count,
            max_diff,
            total_diff as f32 / diff_count as f32
        );
    }

    // Allow up to 1 pixel difference (rounding)
    assert!(
        max_diff <= 1,
        "Max pixel difference {} exceeds threshold 1",
        max_diff
    );

    println!("✓ PASS: Downsampling matches PIL (max_diff={})", max_diff);
}

#[test]
#[ignore] // Requires test data files in /tmp that are not committed
fn test_real_image_case() {
    println!("\n=== TEST 3: Real Image Case (792x612 -> 640x640) ===");

    // Load test input and PIL output
    let input = load_npy_u8_3d("/tmp/test_real_image_input.npy");
    let pil_output = load_npy_u8_3d("/tmp/test_real_image_pil_output.npy");

    println!("Input shape: {:?}", input.shape());
    println!("PIL output shape: {:?}", pil_output.shape());

    // Run Rust resize
    let rust_output = pil_resize_bilinear(&input, 640, 640);
    println!("Rust output shape: {:?}", rust_output.shape());

    // Compare
    let mut max_diff = 0i32;
    let mut total_diff = 0i32;
    let mut diff_count = 0;

    for y in 0..640 {
        for x in 0..640 {
            for c in 0..3 {
                let rust_val = rust_output[[y, x, c]];
                let pil_val = pil_output[[y, x, c]];
                let diff = (rust_val as i32 - pil_val as i32).abs();

                if diff > 0 {
                    diff_count += 1;
                    total_diff += diff;
                    max_diff = max_diff.max(diff);
                }
            }
        }
    }

    let total_pixels = 640 * 640 * 3;
    let diff_pct = (diff_count as f64 / total_pixels as f64) * 100.0;

    println!(
        "Pixels differ: {} / {} ({:.2}%)",
        diff_count, total_pixels, diff_pct
    );
    println!("Max diff: {}", max_diff);
    println!(
        "Avg diff: {:.2}",
        if diff_count > 0 {
            total_diff as f32 / diff_count as f32
        } else {
            0.0
        }
    );

    // Allow up to 1 pixel difference (rounding)
    assert!(
        max_diff <= 1,
        "Max pixel difference {} exceeds threshold 1",
        max_diff
    );

    // Should be very close (< 1% pixels differ)
    assert!(
        diff_pct < 1.0,
        "Too many pixels differ: {:.2}% (expected < 1%)",
        diff_pct
    );

    println!(
        "✓ PASS: Real image resize matches PIL (max_diff={}, {:.2}% pixels differ)",
        max_diff, diff_pct
    );
}
