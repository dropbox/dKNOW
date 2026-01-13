#![cfg(feature = "pytorch")]
// Extract image features from Rust CodeFormula model for comparison with Python baseline
//
// This test loads the same pixel_values used by Python and extracts:
// 1. Vision embeddings (after vision encoder)
// 2. Image features (after connector)
//
// Outputs are saved to debug_output/rust_image_features/ for comparison
use npyz::{NpyFile, WriterBuilder};
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use tch::{Device, Tensor};

// Import our CodeFormula model
use docling_pdf_ml::models::code_formula::CodeFormulaModel;

#[test]
#[ignore = "Requires model weights"]
fn extract_rust_image_features() -> Result<(), Box<dyn std::error::Error>> {
    // Load baseline pixel_values
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let baseline_dir = base_dir.join("baseline_data/code_and_formula/page_0/code_formula");
    let pixel_values_path = baseline_dir.join("code_0_pixel_values.npy");

    assert!(
        pixel_values_path.exists(),
        "Missing pixel_values: {:?}",
        pixel_values_path
    );

    println!("Loading pixel_values from: {:?}", pixel_values_path);

    // Load pixel_values
    let pixel_values_file = File::open(&pixel_values_path)?;
    let pixel_values_npy =
        NpyFile::new(pixel_values_file).expect("Failed to parse pixel_values .npy");

    let pixel_values_shape: Vec<i64> = pixel_values_npy.shape().iter().map(|&x| x as i64).collect();
    println!("  Shape: {:?}", pixel_values_shape);

    let pixel_values_vec: Vec<f32> = pixel_values_npy
        .into_vec()
        .expect("Failed to read pixel_values data");

    let device = Device::cuda_if_available();
    println!("Using device: {:?}", device);

    let pixel_values = Tensor::from_slice(&pixel_values_vec)
        .reshape(&pixel_values_shape)
        .to_device(device);

    // Load model
    println!("\nLoading CodeFormula model...");
    let home_dir = std::env::var("HOME").expect("HOME env var not set");
    let model_dir = PathBuf::from(home_dir)
        .join(".cache/huggingface/hub/models--ds4sd--CodeFormulaV2/snapshots");

    // Find the latest snapshot
    let mut snapshots: Vec<_> = std::fs::read_dir(&model_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().ok().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    snapshots.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

    let snapshot_dir = snapshots.last().ok_or("No snapshots found")?.path();

    println!("  Model dir: {:?}", snapshot_dir);

    let model = CodeFormulaModel::from_pretrained(&snapshot_dir, device)?;

    println!("\n=== Extracting Image Features ===");

    // Create output directory
    let output_dir = base_dir.join("debug_output/rust_image_features");
    fs::create_dir_all(&output_dir)?;

    // Stage 1: Extract vision embeddings (vision encoder only, before connector)
    println!("\n[1/2] Running vision encoder...");
    let vision_embeddings = model.inner_model().get_vision_embeddings(&pixel_values)?;

    let vision_shape = vision_embeddings.size();
    println!("  Vision embeddings shape: {:?}", vision_shape);

    // Get statistics
    let vision_min = vision_embeddings.min().double_value(&[]);
    let vision_max = vision_embeddings.max().double_value(&[]);
    let vision_mean = vision_embeddings.mean(tch::Kind::Float).double_value(&[]);
    let vision_std = vision_embeddings.std(false).double_value(&[]);

    println!("  Vision embeddings dtype: Float");
    println!("  Vision embeddings min: {:.6}", vision_min);
    println!("  Vision embeddings max: {:.6}", vision_max);
    println!("  Vision embeddings mean: {:.6}", vision_mean);
    println!("  Vision embeddings std: {:.6}", vision_std);

    // Save vision embeddings to .npy file
    let vision_path = output_dir.join("vision_embeddings.npy");
    println!("  Saving to: {:?}", vision_path);

    // Convert to Vec<f32> for saving
    let vision_vec: Vec<f32> = vision_embeddings.flatten(0, -1).try_into()?;

    // Write to .npy file
    {
        let mut writer = npyz::WriteOptions::new()
            .default_dtype()
            .shape(&vision_shape.iter().map(|&x| x as u64).collect::<Vec<_>>())
            .writer(File::create(&vision_path)?)
            .begin_nd()?;

        writer.extend(vision_vec)?;
        writer.finish()?;
    }

    println!("  ✓ Saved vision_embeddings.npy");

    // Stage 2: Extract image features (vision encoder + connector)
    println!("\n[2/2] Running connector (perceiver resampler)...");
    let image_features = model.inner_model().get_image_features(&pixel_values)?;

    let img_feat_shape = image_features.size();
    println!("  Image features shape: {:?}", img_feat_shape);

    // Get statistics
    let img_feat_min = image_features.min().double_value(&[]);
    let img_feat_max = image_features.max().double_value(&[]);
    let img_feat_mean = image_features.mean(tch::Kind::Float).double_value(&[]);
    let img_feat_std = image_features.std(false).double_value(&[]);

    println!("  Image features dtype: Float");
    println!("  Image features min: {:.6}", img_feat_min);
    println!("  Image features max: {:.6}", img_feat_max);
    println!("  Image features mean: {:.6}", img_feat_mean);
    println!("  Image features std: {:.6}", img_feat_std);

    // Save image features to .npy file
    let image_path = output_dir.join("image_features.npy");
    println!("  Saving to: {:?}", image_path);

    // Convert to Vec<f32> for saving
    let image_features_vec: Vec<f32> = image_features.flatten(0, -1).try_into()?;

    // Write to .npy file
    {
        let mut writer = npyz::WriteOptions::new()
            .default_dtype()
            .shape(&img_feat_shape.iter().map(|&x| x as u64).collect::<Vec<_>>())
            .writer(File::create(&image_path)?)
            .begin_nd()?;

        writer.extend(image_features_vec)?;
        writer.finish()?;
    }

    println!("  ✓ Saved image_features.npy");

    println!("\n=== Summary ===");
    println!("Vision embeddings: {:?} -> {:?}", vision_shape, vision_path);
    println!("Image features: {:?} -> {:?}", img_feat_shape, image_path);
    println!("\nNext: Compare with Python baseline:");
    println!("  Python: debug_output/image_features/vision_embeddings.npy");
    println!("  Rust:   debug_output/rust_image_features/vision_embeddings.npy");
    println!("  Python: debug_output/image_features/image_features.npy");
    println!("  Rust:   debug_output/rust_image_features/image_features.npy");

    Ok(())
}
