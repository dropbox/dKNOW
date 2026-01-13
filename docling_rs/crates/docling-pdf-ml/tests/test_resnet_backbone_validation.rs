#![cfg(feature = "pytorch")]
mod common;
/// Phase 1 Systematic Validation: ResNet Backbone
///
/// This test validates that the Rust ResNet backbone produces identical stage outputs
/// to the Python PyTorch model. This is the first step in bottom-up module validation.
///
/// Success Criteria: Each stage output < 1e-4 difference from Python baseline
///
/// Stages tested:
/// - stage0: [1, 256, 160, 160] - After first ResNet stage
/// - stage1: [1, 512, 80, 80] - After second ResNet stage
/// - stage2: [1, 1024, 40, 40] - After third ResNet stage
/// - stage3: [1, 2048, 20, 20] - After fourth ResNet stage
use common::baseline_loaders::load_numpy;
use docling_pdf_ml::models::layout_predictor::pytorch_backend::{
    model::{RTDetrV2Config, RTDetrV2ForObjectDetection},
    weights,
};
use std::path::PathBuf;
use tch::{Device, Tensor};

const TOLERANCE: f64 = 1e-4; // Tight tolerance for early stages

#[test]
fn test_resnet_stage_validation() {
    println!("\n{}", "=".repeat(80));
    println!("Phase 1: ResNet Backbone Validation (Systematic Bottom-Up Approach)");
    println!("{}\n", "=".repeat(80));
    println!("Goal: Prove Rust ResNet backbone = Python ResNet backbone (< 1e-4 diff per stage)");
    println!("Method: Same preprocessed tensor → compare stage outputs (stage0-3)");
    println!();

    // Set environment for PyTorch
    std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
    std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");

    let device = Device::Cpu;

    // Get model path
    let model_path = match weights::get_model_path() {
        Ok(path) => path,
        Err(e) => {
            println!("⚠️  Skipping test: {}", e);
            println!("   To fix: huggingface-cli download docling-project/docling-layout-heron");
            return;
        }
    };

    println!("✓ Model path: {:?}", model_path);

    // Read config from HuggingFace
    let config_path = model_path.parent().unwrap().join("config.json");
    let config = if config_path.exists() {
        let config_str = std::fs::read_to_string(&config_path).expect("Failed to read config.json");
        let config_json: serde_json::Value =
            serde_json::from_str(&config_str).expect("Failed to parse config.json");

        // Create config from HuggingFace config.json
        let mut cfg = RTDetrV2Config::default();

        cfg.num_labels = config_json["num_labels"]
            .as_i64()
            .or_else(|| {
                config_json
                    .get("id2label")
                    .and_then(|v| v.as_object())
                    .map(|obj| obj.len() as i64)
            })
            .unwrap_or(cfg.num_labels);

        cfg.d_model = config_json["d_model"].as_i64().unwrap_or(cfg.d_model);
        cfg.encoder_hidden_dim = config_json["encoder_hidden_dim"]
            .as_i64()
            .unwrap_or(cfg.encoder_hidden_dim);
        cfg.encoder_layers = config_json["encoder_layers"]
            .as_i64()
            .unwrap_or(cfg.encoder_layers);
        cfg.decoder_layers = config_json["decoder_layers"]
            .as_i64()
            .unwrap_or(cfg.decoder_layers);
        cfg.num_queries = config_json["num_queries"]
            .as_i64()
            .unwrap_or(cfg.num_queries);

        if let Some(encode_proj) = config_json.get("encode_proj_layers") {
            if let Some(arr) = encode_proj.as_array() {
                cfg.encode_proj_layers = arr.iter().filter_map(|v| v.as_i64()).collect();
            }
        }
        cfg
    } else {
        RTDetrV2Config::default()
    };

    println!(
        "✓ Config loaded (num_labels={}, d_model={}, encoder_layers={}, decoder_layers={})",
        config.num_labels, config.d_model, config.encoder_layers, config.decoder_layers
    );

    // Create VarStore and model
    let mut vs = tch::nn::VarStore::new(device);
    let model = RTDetrV2ForObjectDetection::new(&vs.root(), config.clone())
        .expect("Failed to create model");

    println!("✓ Model structure created");

    // Load weights
    weights::load_weights_into(&mut vs, &model_path).expect("Failed to load weights");
    vs.freeze();

    println!("✓ Weights loaded successfully");

    // Load preprocessed tensor from Python baseline
    let preprocessed_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("ml_model_inputs/layout_predictor/page_0_preprocessed_input.npy");

    if !preprocessed_path.exists() {
        println!(
            "⚠️  Skipping test - preprocessed tensor not found at {:?}",
            preprocessed_path
        );
        println!("   Run: python3 extract_layout_phase1_inputs.py");
        return;
    }

    let preprocessed = load_numpy(&preprocessed_path).expect("Failed to load preprocessed tensor");

    println!("\n✓ Loaded preprocessed tensor from Python");
    println!("  Shape: {:?}", preprocessed.shape());

    // Convert ndarray to tch Tensor
    let shape = preprocessed.shape().to_vec();
    let data = preprocessed.into_raw_vec_and_offset().0;
    let pixel_values = Tensor::from_slice(&data).view(
        shape
            .iter()
            .map(|&x| x as i64)
            .collect::<Vec<_>>()
            .as_slice(),
    );

    println!("  Converted to PyTorch tensor: {:?}", pixel_values.size());

    println!("\n🔬 Running Rust ResNet backbone forward pass...");
    let (_final_features, hidden_states) = model.model.backbone.forward(&pixel_values);

    println!("  Captured {} stage outputs", hidden_states.len());

    // Load Python ResNet stage outputs
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("baseline_data/arxiv_2206.01062/page_0/layout/pytorch_intermediate/resnet_stages");

    let stage_paths = [
        (0, base_path.join("stage0.npy")),
        (1, base_path.join("stage1.npy")),
        (2, base_path.join("stage2.npy")),
        (3, base_path.join("stage3.npy")),
    ];

    // Check all baseline files exist
    for (stage_num, path) in &stage_paths {
        if !path.exists() {
            println!(
                "⚠️  Skipping test - Python ResNet stage{} output not found",
                stage_num
            );
            println!("   Expected: {:?}", path);
            println!("   Run: python3 extract_resnet_stage_outputs.py");
            return;
        }
    }

    println!("\n✓ All Python ResNet stage outputs found");

    // Validate each stage
    println!("\n{}", "=".repeat(80));
    println!("Stage-by-Stage Validation");
    println!("{}\n", "=".repeat(80));

    let mut all_passed = true;

    for (stage_num, path) in &stage_paths {
        println!("--- Stage {} ---", stage_num);

        // Load Python baseline
        let python_output = load_numpy(path)
            .unwrap_or_else(|_| panic!("Failed to load stage{} baseline", stage_num));
        println!("  Python shape: {:?}", python_output.shape());

        // Get Rust output for this stage
        // hidden_states: [embeddings, stage0, stage1, stage2, stage3]
        // So stage N is at index N+1
        let rust_output = &hidden_states[*stage_num + 1];
        println!("  Rust shape: {:?}", rust_output.size());

        // Convert Python ndarray to tch Tensor
        let python_shape = python_output.shape().to_vec();
        let python_data = python_output.into_raw_vec_and_offset().0;
        let python_tensor = Tensor::from_slice(&python_data)
            .view(
                python_shape
                    .iter()
                    .map(|&x| x as i64)
                    .collect::<Vec<_>>()
                    .as_slice(),
            )
            .to(device);

        // Check shape match
        if rust_output.size() != python_tensor.size() {
            println!("  ❌ FAIL: Shape mismatch");
            println!("    Rust: {:?}", rust_output.size());
            println!("    Python: {:?}", python_tensor.size());
            all_passed = false;
            continue;
        }

        // Compute difference
        let diff = (rust_output - &python_tensor).abs();
        let max_diff: f64 = diff.max().double_value(&[]);
        let mean_diff: f64 = diff.mean(tch::Kind::Float).double_value(&[]);

        // Print sample values
        println!(
            "  Rust   [0, :5, 0, 0]: [{:.5}, {:.5}, {:.5}, {:.5}, {:.5}]",
            rust_output.double_value(&[0, 0, 0, 0]),
            rust_output.double_value(&[0, 1, 0, 0]),
            rust_output.double_value(&[0, 2, 0, 0]),
            rust_output.double_value(&[0, 3, 0, 0]),
            rust_output.double_value(&[0, 4, 0, 0]),
        );

        println!(
            "  Python [0, :5, 0, 0]: [{:.5}, {:.5}, {:.5}, {:.5}, {:.5}]",
            python_tensor.double_value(&[0, 0, 0, 0]),
            python_tensor.double_value(&[0, 1, 0, 0]),
            python_tensor.double_value(&[0, 2, 0, 0]),
            python_tensor.double_value(&[0, 3, 0, 0]),
            python_tensor.double_value(&[0, 4, 0, 0]),
        );

        // Check tolerance
        let passed = max_diff < TOLERANCE;

        if passed {
            println!("  ✅ PASS (max diff {:.6e} < {:.0e})", max_diff, TOLERANCE);
        } else {
            println!("  ❌ FAIL (max diff {:.6e} >= {:.0e})", max_diff, TOLERANCE);
            println!("  Mean diff: {:.6e}", mean_diff);

            // Find location of max diff
            let diff_flat = diff.flatten(0, -1);
            let max_idx_flat: i64 = diff_flat.argmax(0, false).int64_value(&[]);
            let total_elements: i64 = rust_output.size().iter().product();
            println!(
                "  Max diff location (flat index): {} / {}",
                max_idx_flat, total_elements
            );

            all_passed = false;
        }

        println!();
    }

    // Final summary
    println!("{}", "=".repeat(80));
    println!("Validation Summary");
    println!("{}\n", "=".repeat(80));

    if all_passed {
        println!("✅ ALL STAGES PASSED - ResNet backbone validated!");
        println!("   Next: Phase 2 - Encoder Validation (FPN/PAN)");
    } else {
        println!("❌ VALIDATION FAILED");
        println!("   Action: Debug ResNet implementation before proceeding to encoder");
        println!("   Check: Conv layers, BatchNorm, ReLU, shortcuts, stride/padding");
        panic!("ResNet backbone validation failed - see output above for details");
    }
}
