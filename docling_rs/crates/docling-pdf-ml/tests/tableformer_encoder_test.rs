#![cfg(feature = "pytorch")]
/// Test transformer encoder shape
///
/// This test verifies that the transformer encoder implementation:
/// 1. Loads weights correctly
/// 2. Produces the expected output shape
/// 3. Runs without errors
///
/// This is a sanity check before implementing the full Phase 1 validation.
use tch::{nn, Device, Kind, Tensor};

#[test]
#[ignore = "Requires model weights - run manually with --ignored"]
fn test_transformer_encoder_shape() {
    println!("\n=== Transformer Encoder Shape Test ===\n");

    // Load model weights
    let weights_path = std::path::PathBuf::from(
        std::env::var("HOME").unwrap()
    ).join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate/tableformer_accurate.pt");

    if !weights_path.exists() {
        panic!(
            "Model weights not found at: {}\n\
            Please ensure docling models are downloaded.",
            weights_path.display()
        );
    }

    println!("Loading model weights from: {}", weights_path.display());

    // Create VarStore
    let mut vs = nn::VarStore::new(Device::Cpu);
    let root = vs.root();

    // Create transformer encoder structure
    // Configuration from Python: 6 layers, 512 hidden_dim, 8 heads, 1024 FFN
    let encoder = docling_pdf_ml::models::table_structure::components::TransformerEncoder::new(
        &(&root / "_tag_transformer" / "_encoder"),
        6,    // num_layers
        512,  // d_model
        8,    // nhead
        1024, // dim_feedforward
    );

    // Load weights
    println!("Loading weights into VarStore...");
    vs.load(&weights_path)
        .expect("Failed to load model weights");
    println!("✓ Weights loaded successfully");

    // Create dummy input tensor
    // Shape: (seq_len, batch, d_model) = (784, 1, 512)
    // where seq_len = 28*28 = 784 (flattened spatial features)
    let seq_len = 784;
    let batch = 1;
    let d_model = 512;

    println!("\nCreating dummy input tensor...");
    let input = Tensor::randn([seq_len, batch, d_model], (Kind::Float, Device::Cpu));
    println!("Input shape: [{}, {}, {}]", seq_len, batch, d_model);

    // Run encoder forward pass
    println!("\nRunning encoder forward pass...");
    let output = encoder.forward(&input, None);

    println!("Output shape: {:?}", output.size());

    // Verify output shape
    let expected_shape = vec![seq_len, batch, d_model];
    assert_eq!(
        output.size(),
        expected_shape,
        "Encoder output shape mismatch!\n\
        Expected: {:?}\n\
        Got: {:?}",
        expected_shape,
        output.size()
    );

    println!("\n✅ SUCCESS: Encoder output shape matches!");
    println!("   Expected: [{}, {}, {}]", seq_len, batch, d_model);
    println!("   Got:      {:?}", output.size());
}
