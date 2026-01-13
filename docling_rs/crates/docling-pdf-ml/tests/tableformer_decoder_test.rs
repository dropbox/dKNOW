#![cfg(feature = "pytorch")]
use std::path::PathBuf;
/// Test: TableFormer Transformer Decoder Shape Validation
///
/// Purpose: Validate that the Rust decoder implementation produces the correct output shape
///
/// Test Strategy:
/// 1. Load PyTorch weights into Rust TransformerDecoder
/// 2. Create dummy inputs:
///    - tgt: (1, 1, 512) - one token, batch=1, d_model=512
///    - memory: (784, 1, 512) - encoder output (28*28=784 spatial positions)
/// 3. Run decoder forward pass
/// 4. Verify output shape matches expected
///
/// Expected Output:
/// - output: (1, 1, 512) - last token embedding
/// - cache: 6 layers, each with shape (1, 1, 512)
///
/// This test validates:
/// - Decoder loads weights correctly
/// - Forward pass runs without errors
/// - Output shapes are correct
///
/// Run with:
/// LIBTORCH_USE_PYTORCH=1 cargo test --release --test tableformer_decoder_test -- --ignored --nocapture
use tch::{nn, Device, Kind, Tensor};

#[test]
#[ignore = "Requires PyTorch weights"]
fn test_transformer_decoder_shape() {
    println!("\n=== TableFormer Transformer Decoder Shape Test ===\n");

    // Set device (use CPU for testing, same as Python baseline)
    let device = Device::Cpu;

    // Create VarStore
    let mut vs = nn::VarStore::new(device);
    let root = vs.root();

    // Create decoder structure BEFORE loading weights
    println!("Creating TransformerDecoder structure...");
    let decoder_path = root.sub("_tag_transformer").sub("_decoder");

    // Parameters from Python baseline
    let num_layers = 6;
    let d_model = 512;
    let nhead = 8;
    let dim_feedforward = 1024;

    let decoder = docling_pdf_ml::models::table_structure::components::TransformerDecoder::new(
        &decoder_path,
        num_layers,
        d_model,
        nhead,
        dim_feedforward,
    );
    println!("✓ Decoder structure created");

    // Now load weights (using the original .pt file from huggingface)
    let weights_path = PathBuf::from(
        std::env::var("HOME").unwrap()
    ).join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate/tableformer_accurate.pt");

    println!("\nLoading weights from: {}", weights_path.display());

    if !weights_path.exists() {
        panic!(
            "Weight file not found: {}. Please ensure docling models are downloaded.",
            weights_path.display()
        );
    }

    vs.load(&weights_path)
        .expect("Failed to load model weights");
    println!("✓ Weights loaded successfully");

    // Create dummy inputs
    println!("\nCreating dummy inputs...");

    // Target: one token, batch=1
    let tgt = Tensor::randn([1, 1, d_model], (Kind::Float, device));
    println!("  tgt shape: {:?}", tgt.size());

    // Memory: encoder output (28x28=784 positions from image)
    let enc_image_size = 28;
    let memory_seq_len = enc_image_size * enc_image_size; // 784
    let memory = Tensor::randn([memory_seq_len, 1, d_model], (Kind::Float, device));
    println!("  memory shape: {:?}", memory.size());

    // Run decoder forward pass
    println!("\nRunning decoder forward pass...");
    let (output, cache) = decoder.forward(&tgt, &memory, None);

    // Verify output shape
    println!("\nValidating output shapes...");
    let output_shape = output.size();
    println!("  Output shape: {:?}", output_shape);

    // Expected: (1, 1, 512) - last token only
    let expected_shape = vec![1, 1, d_model];

    if output_shape == expected_shape {
        println!("✓ SUCCESS: Decoder output shape matches expected");
        println!("     Expected: {:?}", expected_shape);
        println!("     Got:      {:?}", output_shape);
    } else {
        println!("✗ FAILED: Decoder output shape mismatch");
        println!("     Expected: {:?}", expected_shape);
        println!("     Got:      {:?}", output_shape);
        panic!("Shape mismatch");
    }

    // Validate cache
    println!("\nValidating cache...");
    for i in 0..num_layers as usize {
        if let Some(layer_cache) = cache.get(i) {
            let cache_shape = layer_cache.size();
            println!("  Layer {} cache shape: {:?}", i, cache_shape);

            // Expected: (1, 1, 512) for each layer after first step
            if cache_shape != expected_shape {
                println!("✗ FAILED: Layer {} cache shape mismatch", i);
                panic!("Cache shape mismatch at layer {}", i);
            }
        } else {
            println!("✗ FAILED: No cache for layer {}", i);
            panic!("Missing cache for layer {}", i);
        }
    }
    println!("✓ All layer caches have correct shape");

    // Test cache concatenation (second decoding step)
    println!("\nTesting cache concatenation (second step)...");
    let tgt2 = Tensor::randn([1, 1, d_model], (Kind::Float, device));
    let (output2, cache2) = decoder.forward(&tgt2, &memory, Some(&cache));

    let output2_shape = output2.size();
    println!("  Output shape after cache: {:?}", output2_shape);

    // Expected: (2, 1, 512) - now includes first token
    let expected_shape2 = vec![2, 1, d_model];
    if output2_shape == expected_shape2 {
        println!("✓ SUCCESS: Cached output shape correct");
        println!("     Expected: {:?}", expected_shape2);
        println!("     Got:      {:?}", output2_shape);
    } else {
        println!("✗ FAILED: Cached output shape mismatch");
        println!("     Expected: {:?}", expected_shape2);
        println!("     Got:      {:?}", output2_shape);
        panic!("Cached shape mismatch");
    }

    // Validate cache after second step
    for i in 0..num_layers as usize {
        if let Some(layer_cache) = cache2.get(i) {
            let cache_shape = layer_cache.size();
            println!(
                "  Layer {} cache shape after 2nd step: {:?}",
                i, cache_shape
            );

            // Expected: (2, 1, 512) - accumulated outputs
            if cache_shape != expected_shape2 {
                println!("✗ FAILED: Layer {} cache accumulation incorrect", i);
                panic!("Cache accumulation failed at layer {}", i);
            }
        }
    }
    println!("✓ Cache accumulation working correctly");

    println!("\n=== All decoder shape tests PASSED ===\n");
}
