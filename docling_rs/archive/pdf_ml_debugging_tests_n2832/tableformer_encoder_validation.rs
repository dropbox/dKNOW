#![cfg(feature = "pytorch")]
mod common;

use ndarray::ArrayD;
use std::path::PathBuf;
/// Validate TableFormer encoder output matches Python baseline
///
/// This test isolates the encoder to verify it produces identical outputs
/// to the Python implementation, following Phase 1 validation methodology.
use tch::{Device, Tensor};

// Helper function to convert ndarray to tch::Tensor
fn numpy_to_tensor(arr: &ArrayD<f32>, device: Device) -> Tensor {
    let shape: Vec<i64> = arr.shape().iter().map(|&x| x as i64).collect();
    let data: Vec<f32> = arr.iter().copied().collect();
    Tensor::from_slice(&data).to(device).reshape(&shape)
}

// Helper function to convert tch::Tensor to Vec<f32>
fn tensor_to_vec(tensor: &Tensor) -> Vec<f32> {
    Vec::<f32>::try_from(tensor.view(-1)).expect("Failed to convert tensor to vec")
}

// Helper function to compute max absolute difference
fn max_diff(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Arrays must have same length");
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

#[test]
#[ignore]
fn test_encoder_output_matches_python() {
    println!("\n{}", "=".repeat(80));
    println!("TableFormer Encoder Validation - Phase 1");
    println!("{}\n", "=".repeat(80));

    let device = Device::Cpu;
    let home = std::env::var("HOME").unwrap();
    let base_path =
        PathBuf::from(&home).join("docling_debug_pdf_parsing/ml_model_inputs/tableformer");

    // Load preprocessed input
    println!("Loading preprocessed input...");
    let input_path = base_path.join("table_0_preprocessed_input.npy");
    let preprocessed_arr = common::baseline_loaders::load_numpy(&input_path)
        .expect("Failed to load preprocessed input");
    let preprocessed_tensor = numpy_to_tensor(&preprocessed_arr, device);
    println!("  ✓ Input shape: {:?}", preprocessed_tensor.size());

    // Debug: Print input tensor details
    let input_vec = tensor_to_vec(&preprocessed_tensor);
    println!("  Input first 10: {:?}", &input_vec[..10]);
    let input_min = input_vec.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let input_max = input_vec.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    println!("  Input range: min={:.6}, max={:.6}", input_min, input_max);

    // Load model
    println!("\nLoading TableFormer model...");
    let model_dir = PathBuf::from(&home)
        .join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate");
    let model =
        docling_pdf_ml::models::table_structure::TableStructureModel::load(&model_dir, device)
            .expect("Failed to load model");
    println!("  ✓ Model loaded");

    // Debug: Verify encoder weights and running stats are loaded correctly
    println!("\n{}", "=".repeat(80));
    println!("Verifying Encoder Weights and Running Stats");
    println!("{}", "=".repeat(80));

    // Check first conv weight (should match Python)
    // Python first 10: [-0.0107, -0.0062, 0.0425, 0.0287, -0.0318, -0.0062, 0.0087, -0.0513, -0.0126, 0.1623]
    // Access the weights directly from the model (we need to access the VarStore)
    // For now, we'll verify by checking intermediate outputs

    // Run Rust encoder + transformer encoder pipeline
    println!("\n{}", "=".repeat(80));
    println!("Running Rust Encoder Pipeline");
    println!("{}", "=".repeat(80));

    // Step 1: ResNet18 encoder
    println!("\nStep 1: ResNet18 encoder...");
    let encoder_out = model.encoder.forward(&preprocessed_tensor);
    println!("  ✓ Shape: {:?}", encoder_out.size());

    // Debug: Check encoder output values
    print!("  Encoder output first 10 (BHWC): [");
    for i in 0..10 {
        let val: f64 = encoder_out.double_value(&[0, 0, 0, i]);
        print!("{:.6}", val);
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");
    let encoder_max: f64 = encoder_out.max().double_value(&[]);
    let encoder_min: f64 = encoder_out.min().double_value(&[]);
    println!(
        "  Encoder output range: min={:.6}, max={:.6}",
        encoder_min, encoder_max
    );

    // Step 2: Input filter (256 → 512 projection)
    println!("\nStep 2: Input filter (256 → 512)...");
    let encoder_bchw = encoder_out.permute([0, 3, 1, 2]);
    println!("  After BHWC→BCHW permute: {:?}", encoder_bchw.size());
    let filtered = model.tag_transformer.input_filter.forward(&encoder_bchw);
    println!("  After input_filter (BCHW): {:?}", filtered.size());
    let filtered_bhwc = filtered.permute([0, 2, 3, 1]);
    println!("  ✓ After BCHW→BHWC permute: {:?}", filtered_bhwc.size());

    // Debug: Check filtered output values at different positions
    print!("  Filtered [0,0,0,0:10]: [");
    for i in 0..10 {
        let val: f64 = filtered_bhwc.double_value(&[0, 0, 0, i]);
        print!("{:.6}", val);
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");
    print!("  Filtered [0,5,5,0:10]: [");
    for i in 0..10 {
        let val: f64 = filtered_bhwc.double_value(&[0, 5, 5, i]);
        print!("{:.6}", val);
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");
    let filtered_max: f64 = filtered_bhwc.max().double_value(&[]);
    let filtered_min: f64 = filtered_bhwc.min().double_value(&[]);
    println!(
        "  Filtered output range: min={:.6}, max={:.6}",
        filtered_min, filtered_max
    );

    // Step 3: Reshape for transformer
    println!("\nStep 3: Reshape for transformer...");
    let (batch, height, width, channels) = filtered_bhwc.size4().unwrap();
    let spatial_len = height * width;
    println!(
        "  Batch: {}, Height: {}, Width: {}, Channels: {}",
        batch, height, width, channels
    );
    println!("  Spatial length: {}", spatial_len);

    let encoder_flat = filtered_bhwc.view([batch, spatial_len, channels]);
    println!(
        "  After view [batch, spatial, channels]: {:?}",
        encoder_flat.size()
    );
    print!("  encoder_flat [0, 0, 0:10]: [");
    for i in 0..10 {
        let val: f64 = encoder_flat.double_value(&[0, 0, i]);
        print!("{:.6}", val);
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");

    let encoder_for_transformer = encoder_flat.permute([1, 0, 2]);
    println!(
        "  After permute [spatial, batch, channels]: {:?}",
        encoder_for_transformer.size()
    );

    // Save Rust encoder input for external comparison (can't load Python .npy file due to tch-rs crash)
    println!("\n{}", "=".repeat(80));
    println!("Saving Rust Encoder Input for External Comparison");
    println!("{}", "=".repeat(80));
    println!(
        "  Rust encoder input shape: {:?}",
        encoder_for_transformer.size()
    );

    // Print first few values using direct indexing (safer than tensor_to_vec)
    print!("  Rust encoder input first 10: [");
    for i in 0..10 {
        let val: f64 = encoder_for_transformer.double_value(&[0, 0, i]);
        print!("{:.6}", val);
        if i < 9 {
            print!(", ");
        }
    }
    println!("]");

    // Save using tch's built-in save (PyTorch format, not .npy)
    let rust_encoder_input_pt_path = base_path.join("rust_encoder_input_before_transformer.pt");
    encoder_for_transformer
        .save(&rust_encoder_input_pt_path)
        .expect("Failed to save Rust encoder input");
    println!(
        "  ✓ Saved Rust encoder input to: {}",
        rust_encoder_input_pt_path.display()
    );
    println!(
        "  (Can be loaded in Python with: torch.load('rust_encoder_input_before_transformer.pt'))"
    );

    // Step 4: Transformer encoder
    println!("\n{}", "=".repeat(80));
    println!("Step 4: Transformer encoder...");
    println!("{}", "=".repeat(80));
    let rust_memory = model
        .tag_transformer
        .encoder
        .forward(&encoder_for_transformer, None);
    println!("  ✓ Rust memory shape: {:?}", rust_memory.size());

    let rust_memory_vec = tensor_to_vec(&rust_memory);
    println!("  ✓ Rust memory first 10: {:?}", &rust_memory_vec[..10]);

    // Load Python baseline encoder output
    println!("\n{}", "=".repeat(80));
    println!("Loading Python Baseline Encoder Output");
    println!("{}", "=".repeat(80));

    let python_encoder_path = base_path.join("encoder_memory_output.npy");
    let python_memory_arr = common::baseline_loaders::load_numpy(&python_encoder_path)
        .expect("Failed to load Python encoder output");
    let python_memory_tensor = numpy_to_tensor(&python_memory_arr, device);
    println!("  ✓ Python memory shape: {:?}", python_memory_tensor.size());

    let python_memory_vec = tensor_to_vec(&python_memory_tensor);
    println!("  ✓ Python memory first 10: {:?}", &python_memory_vec[..10]);

    // Compare outputs
    println!("\n{}", "=".repeat(80));
    println!("Comparison: Rust vs Python Encoder Output");
    println!("{}", "=".repeat(80));

    // Check shapes match
    assert_eq!(
        rust_memory.size(),
        python_memory_tensor.size(),
        "Encoder output shapes differ!"
    );
    println!("  ✓ Shapes match: {:?}", rust_memory.size());

    // Compute difference
    let max_difference = max_diff(&rust_memory_vec, &python_memory_vec);
    println!("\n  Max absolute difference: {:.10}", max_difference);

    // Compute statistics
    let mean_rust: f32 = rust_memory_vec.iter().sum::<f32>() / rust_memory_vec.len() as f32;
    let mean_python: f32 = python_memory_vec.iter().sum::<f32>() / python_memory_vec.len() as f32;
    println!("  Rust mean:   {:.6}", mean_rust);
    println!("  Python mean: {:.6}", mean_python);

    // Check threshold
    let threshold = 1e-5;
    if max_difference < threshold {
        println!("\n  ✅ ENCODER MATCHES! (diff < {})", threshold);
        println!("  → Encoder implementation is correct");
        println!("  → Cross-attention bug must be in decoder or attention mechanism");
    } else {
        println!(
            "\n  ❌ ENCODER DIFFERS! (diff = {} > {})",
            max_difference, threshold
        );
        println!("  → Encoder produces different outputs than Python");
        println!("  → Need to debug encoder pipeline");

        // Show first few differences
        println!("\n  First 20 value pairs:");
        for i in 0..20 {
            let diff = (rust_memory_vec[i] - python_memory_vec[i]).abs();
            println!(
                "    [{}] Rust: {:.6}, Python: {:.6}, Diff: {:.6}",
                i, rust_memory_vec[i], python_memory_vec[i], diff
            );
        }
    }

    println!("\n{}", "=".repeat(80));

    // Assert threshold for test pass/fail
    assert!(
        max_difference < threshold,
        "Encoder output differs by {} (threshold: {})",
        max_difference,
        threshold
    );
}
