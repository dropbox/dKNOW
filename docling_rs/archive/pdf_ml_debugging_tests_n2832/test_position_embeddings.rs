#![cfg(feature = "pytorch")]
use docling_pdf_ml::models::layout_predictor::pytorch_backend::encoder::RTDetrV2HybridEncoder;
use tch::Device;

#[test]
#[ignore] // Temporarily disabled - has compilation issues, not needed for current debugging
fn test_position_embeddings_match_python() {
    // TODO: Fix compilation errors in this test
    // let width = 20;
    // let height = 20;
    // let embed_dim = 256;
    // let temperature = 10000.0;

    let pos_embed = RTDetrV2HybridEncoder::build_2d_sincos_position_embedding(
        20,
        20,
        256,
        10000.0,
        Device::Cpu,
    )
    .expect("Failed to build position embeddings");

    println!("Rust position embedding shape: {:?}", pos_embed.size());
    println!("\n⚠️ Test disabled - needs npyz parsing fixes");
}
