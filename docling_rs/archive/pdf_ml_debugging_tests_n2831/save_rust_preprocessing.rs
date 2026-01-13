/// Save Rust preprocessing output for comparison with Python PIL
///
/// This test runs Rust preprocessing on the same input image Python uses,
/// then saves the output so we can compare with Python's PIL baseline.
use docling_pdf_ml::preprocessing::layout::layout_preprocess;
use ndarray::Array3;
use npyz::{NpyFile, WriterBuilder};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[test]
fn save_rust_preprocessing_arxiv_page0() {
    // Load input image (same one Python uses)
    let image_path =
        PathBuf::from("baseline_data/arxiv_2206.01062/page_0/layout/input_page_image.npy");
    let file = File::open(&image_path).expect("Failed to open input image");
    let reader = BufReader::new(file);
    let npy = NpyFile::new(reader).expect("Failed to parse NPY file");
    let shape = npy.shape().to_vec();
    let data: Vec<u8> = npy.into_vec().expect("Failed to read NPY data");
    let image = Array3::from_shape_vec(
        (shape[0] as usize, shape[1] as usize, shape[2] as usize),
        data,
    )
    .expect("Failed to create Array3");

    println!("Input image shape: {:?}", image.shape());
    println!(
        "Input image range: [{}, {}]",
        image.iter().min().unwrap(),
        image.iter().max().unwrap()
    );

    // Run Rust preprocessing
    let preprocessed = layout_preprocess(&image);
    println!("Preprocessed shape: {:?}", preprocessed.shape());
    println!(
        "Preprocessed range: [{:.10}, {:.10}]",
        preprocessed.iter().cloned().fold(f32::INFINITY, f32::min),
        preprocessed
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max)
    );

    // Save to .npy file
    let output_path = PathBuf::from("debug_rust_preprocessing.npy");
    let file = File::create(&output_path).expect("Failed to create output file");

    // Convert shape to u64
    let shape_u64: Vec<u64> = preprocessed.shape().iter().map(|&x| x as u64).collect();

    let mut writer = npyz::WriteOptions::new()
        .dtype(npyz::DType::Plain("<f4".parse().unwrap()))
        .shape(&shape_u64)
        .writer(file)
        .begin_nd()
        .expect("Failed to create NPY writer");

    for &val in preprocessed.iter() {
        writer.push(&val).expect("Failed to write value");
    }

    println!("✓ Saved Rust preprocessing to: {:?}", output_path);
    println!();
    println!("Now run: python3 compare_rust_vs_pil.py");
}
