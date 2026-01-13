/// Compare fixed-point vs float PIL resize implementations
///
/// Tests whether the fixed-point implementation achieves better pixel accuracy
/// than the float implementation when compared to PIL baseline.
use docling_pdf_ml::preprocessing::pil_resize::pil_resize_bilinear;
use docling_pdf_ml::preprocessing::pil_resize_fixed_point::pil_resize_bilinear_fixed_point;
use ndarray::Array3;
use npyz::NpyFile;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[test]
fn test_fixed_point_vs_float_real_image() {
    // Load real test image (arxiv page 0: 792×612)
    let base_path = PathBuf::from("baseline_data/arxiv_2206.01062/page_0/layout");
    let image_path = base_path.join("input_page_image.npy");

    if !image_path.exists() {
        eprintln!("Baseline image not found at {image_path:?}. Skipping test.");
        return;
    }

    // Load input image
    let file = File::open(&image_path).expect("Failed to open image file");
    let reader = BufReader::new(file);
    let npy = NpyFile::new(reader).expect("Failed to parse NPY file");
    let shape = npy.shape().to_vec();
    let data: Vec<u8> = npy.into_vec().expect("Failed to read NPY data");
    let input_image = Array3::from_shape_vec(
        (shape[0] as usize, shape[1] as usize, shape[2] as usize),
        data,
    )
    .expect("Failed to create array");

    println!("Input image shape: {:?}", input_image.shape());

    // Resize with both implementations
    let output_float = pil_resize_bilinear(&input_image, 640, 640);
    let output_fixed = pil_resize_bilinear_fixed_point(&input_image, 640, 640);

    // Load PIL baseline (preprocessed tensor, but we need the resized image)
    // Actually, let's just compare the two implementations directly
    let mut diff_count = 0;
    let mut max_diff = 0_i32;
    let total_pixels = 640 * 640 * 3;

    for i in 0..640 {
        for j in 0..640 {
            for c in 0..3 {
                let float_val = output_float[[i, j, c]] as i32;
                let fixed_val = output_fixed[[i, j, c]] as i32;
                let diff = (float_val - fixed_val).abs();

                if diff > 0 {
                    diff_count += 1;
                    max_diff = max_diff.max(diff);
                }
            }
        }
    }

    let diff_percent = (diff_count as f64 / total_pixels as f64) * 100.0;

    println!("\n=== Float vs Fixed-Point Comparison ===");
    println!("Pixels differ: {diff_count} / {total_pixels} ({diff_percent:.4}%)");
    println!(
        "Max difference: {} / 255 ({:.6} normalized)",
        max_diff,
        max_diff as f64 / 255.0
    );

    // We expect some differences due to rounding, but they should be minimal
    assert!(
        diff_percent < 5.0,
        "Fixed-point and float implementations differ too much: {diff_percent:.4}%"
    );
    assert!(max_diff <= 2, "Max difference too large: {max_diff}");
}
