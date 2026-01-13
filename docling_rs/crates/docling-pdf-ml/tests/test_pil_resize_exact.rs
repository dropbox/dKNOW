/// Test: Verify Rust PIL resize produces EXACT output as Python PIL
///
/// This test loads the same input image Python uses, runs Rust PIL resize,
/// and saves the output for comparison with Python.
use docling_pdf_ml::preprocessing::pil_resize::pil_resize_bilinear;
use ndarray::Array3;
use npyz::{NpyFile, WriterBuilder};
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[test]
fn test_rust_pil_resize_exact() {
    // Load input image that Python uses
    let baseline_dir = "baseline_data/arxiv_2206.01062/page_0/layout";
    let input_path = format!("{baseline_dir}/input_page_image.npy");

    let file = File::open(&input_path).expect("Failed to open input image");
    let reader = BufReader::new(file);
    let npy = NpyFile::new(reader).expect("Failed to read npy file");

    let shape = npy.shape().to_vec();
    let data: Vec<u8> = npy.into_vec::<u8>().expect("Failed to read data");

    let input_image = Array3::<u8>::from_shape_vec(
        (shape[0] as usize, shape[1] as usize, shape[2] as usize),
        data,
    )
    .expect("Failed to reshape");

    println!("Input image shape: {:?}", input_image.dim());

    // Resize using Rust PIL implementation
    let resized = pil_resize_bilinear(&input_image, 640, 640);
    println!("Resized image shape: {:?}", resized.dim());

    // Save output for Python comparison
    let output_path = "/tmp/rust_pil_resized.npy";
    let mut writer =
        BufWriter::new(File::create(output_path).expect("Failed to create output file"));

    let mut npy_writer = npyz::WriteOptions::new()
        .dtype(npyz::DType::Plain("<u1".parse().unwrap()))
        .shape(&[640, 640, 3])
        .writer(&mut writer)
        .begin_nd()
        .expect("Failed to write header");

    npy_writer
        .extend(resized.iter().copied())
        .expect("Failed to write data");

    npy_writer.finish().expect("Failed to finish writing");

    println!("Saved Rust output to: {output_path}");
    println!("\nRun Python script to compare:");
    println!("  python3 test_pil_rust_exact.py");
}
