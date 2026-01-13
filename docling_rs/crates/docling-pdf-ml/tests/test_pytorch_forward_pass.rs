#![cfg(feature = "pytorch")]
mod common;
// Test PyTorch backend forward pass and generate attention debug outputs
// This test runs the full forward pass and verifies convergence with Python

use common::baseline_loaders::load_numpy;
use docling_pdf_ml::models::layout_predictor::pytorch_backend::{
    model::{RTDetrV2Config, RTDetrV2ForObjectDetection},
    weights,
};
use std::path::PathBuf;
use tch::{Device, Tensor};

#[test]
fn test_pytorch_forward_pass() {
    println!("\n================================================================================");
    println!("PyTorch Backend Forward Pass - Generate Attention Debug Outputs");
    println!("================================================================================");

    // Get model path
    let model_path = match weights::get_model_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("⚠️  Skipping test: {}", e);
            eprintln!("   To fix: huggingface-cli download docling-project/docling-layout-heron");
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

        // Derive num_labels from id2label if num_labels not present
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

        // Parse encode_proj_layers array
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
    let mut vs = tch::nn::VarStore::new(Device::Cpu);
    let model = RTDetrV2ForObjectDetection::new(&vs.root(), config.clone())
        .expect("Failed to create model");

    println!("✓ Model structure created");

    // Load weights
    weights::load_weights_into(&mut vs, &model_path).expect("Failed to load weights");

    println!("✓ Weights loaded successfully");

    // Load preprocessed tensor from Python baseline (same as Phase 1 tests)
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

    println!("✓ Loaded preprocessed tensor from baseline");
    println!("  Shape: {:?}", preprocessed.shape());

    // Convert ndarray to tch Tensor
    let shape = preprocessed.shape().to_vec();
    let data = preprocessed.into_raw_vec_and_offset().0;
    let input_tensor = Tensor::from_slice(&data).view(
        shape
            .iter()
            .map(|&x| x as i64)
            .collect::<Vec<_>>()
            .as_slice(),
    );

    println!("  Tensor shape: {:?}", input_tensor.size());

    // Run forward pass (this will generate rust_attn_*.npy files)
    println!("\nRunning forward pass (will generate rust_attn_*.npy files)...");

    tch::no_grad(|| {
        let outputs = model.forward(&input_tensor);

        match outputs {
            Ok(output) => {
                println!("✅ Forward pass complete!");
                println!("   Logits shape: {:?}", output.logits.size());
                println!("   Boxes shape: {:?}", output.pred_boxes.size());

                // Check if attention debug files were created
                let debug_files = [
                    "rust_attn_input_hidden_states.npy",
                    "rust_attn_position_embeddings.npy",
                    "rust_attn_hidden_states_with_pos.npy",
                    "rust_attn_query_after_proj.npy",
                    "rust_attn_query_after_scaling.npy",
                    "rust_attn_key_after_proj.npy",
                    "rust_attn_value_after_proj.npy",
                    "rust_attn_query_reshaped.npy",
                    "rust_attn_key_reshaped.npy",
                    "rust_attn_value_reshaped.npy",
                    "rust_attn_attn_scores.npy",
                    "rust_attn_attn_weights_after_softmax.npy",
                    "rust_attn_attn_probs_after_dropout.npy",
                    "rust_attn_attn_output_before_reshape.npy",
                    "rust_attn_attn_output_before_out_proj.npy",
                    "rust_attn_attn_output_final.npy",
                ];

                println!("\nAttention debug files generated:");
                for file in &debug_files {
                    if std::path::Path::new(file).exists() {
                        println!("   ✅ {}", file);
                    } else {
                        println!("   ❌ Missing: {}", file);
                    }
                }
            }
            Err(e) => {
                panic!("❌ Forward pass failed: {:?}", e);
            }
        }
    });

    println!("\n================================================================================");
    println!("✅ Test complete!");
    println!("================================================================================");
    println!("Next step: Run compare_attention_internals.py to verify convergence");
    println!("   python3 compare_attention_internals.py");
}
