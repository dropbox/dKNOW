#![cfg(feature = "pytorch")]
mod common;

use ndarray::ArrayD;
use std::path::PathBuf;
/// Debug test to isolate TableFormer predict() crash
///
/// This test incrementally tests each step of the predict() method
/// to find exactly where the crash occurs.
use tch::{Device, Tensor};

// Helper function to convert ndarray to tch::Tensor
fn numpy_to_tensor(arr: &ArrayD<f32>, device: Device) -> Tensor {
    let shape: Vec<i64> = arr.shape().iter().map(|&x| x as i64).collect();
    let data: Vec<f32> = arr.iter().copied().collect();
    Tensor::from_slice(&data).to(device).reshape(&shape)
}

#[test]
#[ignore]
fn test_predict_step_by_step() {
    println!("\n=== TableFormer Predict() Debug Test ===\n");

    let device = Device::Cpu;
    let home = std::env::var("HOME").unwrap();

    // Load preprocessed input
    let input_path = PathBuf::from(&home).join(
        "docling_debug_pdf_parsing/ml_model_inputs/tableformer/table_0_preprocessed_input.npy",
    );
    let preprocessed_arr = common::baseline_loaders::load_numpy(&input_path)
        .expect("Failed to load preprocessed input");
    let preprocessed_tensor = numpy_to_tensor(&preprocessed_arr, device);
    println!("✓ Input tensor loaded: {:?}", preprocessed_tensor.size());

    // Load model
    let model_dir = PathBuf::from(&home)
        .join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate");

    println!("\n✓ Loading model...");
    let model =
        docling_pdf_ml::models::table_structure::TableStructureModel::load(&model_dir, device)
            .expect("Failed to load model");
    println!("✓ Model loaded");

    // Step 1: Test encoder forward
    println!("\nStep 1: Testing encoder forward...");
    let encoder_out = model.encoder.forward(&preprocessed_tensor);
    println!("✓ Encoder output shape: {:?}", encoder_out.size());

    // Step 2: Test input_filter
    println!("\nStep 2: Testing input_filter (256→512 projection)...");
    let encoder_bchw = encoder_out.permute([0, 3, 1, 2]);
    println!("  Permuted to BCHW: {:?}", encoder_bchw.size());

    let filtered = model.tag_transformer.input_filter.forward(&encoder_bchw);
    println!("✓ Input filter output shape: {:?}", filtered.size());

    let filtered_bhwc = filtered.permute([0, 2, 3, 1]);
    println!("✓ Permuted back to BHWC: {:?}", filtered_bhwc.size());

    // Step 3: Test reshape for transformer
    println!("\nStep 3: Testing reshape for transformer...");
    let (_batch, height, width, channels) = filtered_bhwc.size4().unwrap();
    println!(
        "  Filtered dimensions: batch={}, h={}, w={}, c={}",
        _batch, height, width, channels
    );

    let spatial_len = height * width;
    println!("  Spatial length: {}", spatial_len);

    let encoder_flat = filtered_bhwc.view([1, spatial_len, channels]);
    println!("✓ Flattened shape: {:?}", encoder_flat.size());

    let encoder_for_transformer = encoder_flat.permute([1, 0, 2]);
    println!("✓ Permuted shape: {:?}", encoder_for_transformer.size());

    // Step 4: Test transformer encoder
    println!("\nStep 4: Testing transformer encoder...");
    let memory = model
        .tag_transformer
        .encoder
        .forward(&encoder_for_transformer, None);
    println!("✓ Transformer encoder output shape: {:?}", memory.size());

    // Step 5: Test autoregressive generation
    println!("\nStep 5: Testing autoregressive tag generation...");
    let (tag_sequence, tag_h, bboxes_to_merge) = model
        .tag_transformer
        .generate_tag_sequence(&memory, model.config.max_steps);
    println!("✓ Generated {} tags", tag_sequence.len());
    println!("✓ Generated {} cell decoder outputs", tag_h.len());
    println!("✓ BBox merge map size: {}", bboxes_to_merge.len());
    println!(
        "  First 20 tags: {:?}",
        &tag_sequence[..20.min(tag_sequence.len())]
    );

    println!("\n✅ All steps completed successfully!");
}
