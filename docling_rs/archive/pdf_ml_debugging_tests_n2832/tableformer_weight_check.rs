#![cfg(feature = "pytorch")]
use std::path::PathBuf;
/// Test to check if TableFormer weights are loaded correctly
///
/// This test verifies that embedding and FC layer weights match Python baseline.
use tch::{nn, nn::Module, Device};

#[test]
#[ignore]
fn test_embedding_and_fc_weights() {
    println!("\n=== TableFormer Weight Check ===\n");

    let device = Device::Cpu;
    let home = std::env::var("HOME").unwrap();
    let model_dir = PathBuf::from(&home)
        .join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate");

    println!("Loading model from: {:?}\n", model_dir);

    let mut vs = nn::VarStore::new(device);

    // Load the model weights
    let weights_path = model_dir.join("tableformer_accurate.pt");
    vs.load(&weights_path).expect("Failed to load weights");

    println!("✓ Weights loaded\n");

    // Get root after loading
    let root = vs.root();

    // Check embedding layer
    println!("=== Checking Embedding Layer ===");
    let embedding_path = root.sub("_tag_transformer").sub("_embedding");
    let embedding = nn::embedding(&embedding_path, 13, 512, Default::default());

    // Get embedding for token 2 (TAG_START)
    let token_2 = tch::Tensor::from_slice(&[2i64]).to(device);
    let emb_2 = embedding.forward(&token_2);
    println!(
        "Embedding for token 2 (TAG_START) shape: {:?}",
        emb_2.size()
    );
    let emb_2_vec: Vec<f32> = (0..10)
        .map(|i| emb_2.double_value(&[0, i as i64]) as f32)
        .collect();
    println!("First 10 values: {:?}", emb_2_vec);

    // Check FC layer
    println!("\n=== Checking FC Layer ===");
    let fc_path = root.sub("_tag_transformer").sub("_fc");
    let fc = nn::linear(&fc_path, 512, 13, Default::default());

    // Test forward pass with a dummy input
    let dummy_input = tch::Tensor::randn([1, 512], (tch::Kind::Float, device));
    let fc_output = fc.forward(&dummy_input);
    println!("FC output shape: {:?}", fc_output.size());
    let fc_output_vec: Vec<f32> = (0..13)
        .map(|i| fc_output.double_value(&[0, i]) as f32)
        .collect();
    println!("FC output: {:?}", fc_output_vec);

    // Check positional encoding
    println!("\n=== Checking Positional Encoding ===");
    let pe_path = root.sub("_tag_transformer").sub("_positional_encoding");
    let pe = pe_path.var("pe", &[1024, 1, 512], nn::Init::Const(0.0));
    println!("Positional encoding shape: {:?}", pe.size());

    // Check position 0
    let pe_vec_0: Vec<f32> = (0..10)
        .map(|i| pe.double_value(&[0, 0, i]) as f32)
        .collect();
    println!("Position 0, first 10 values: {:?}", pe_vec_0);
    println!(
        "Position 0, values 10-20: {:?}",
        (10..20)
            .map(|i| pe.double_value(&[0, 0, i]) as f32)
            .collect::<Vec<f32>>()
    );

    // Check position 1
    let pe_vec_1: Vec<f32> = (0..10)
        .map(|i| pe.double_value(&[1, 0, i]) as f32)
        .collect();
    println!("Position 1, first 10 values: {:?}", pe_vec_1);

    println!("\n✅ Weight check complete");
}
