#![cfg(feature = "pytorch")]
// Test position embedding fix after N=417
// Verifies that position embeddings produce values in [-1, 1] range (not [0, 1])

use docling_pdf_ml::models::layout_predictor::pytorch_backend::encoder::RTDetrV2HybridEncoder;
use tch::Device;

#[test]
fn test_position_embedding_has_negative_values() {
    let width = 20;
    let height = 20;
    let embed_dim = 256;
    let temperature = 10000.0;
    let device = Device::Cpu;

    // Build position embeddings
    let pos_embed = RTDetrV2HybridEncoder::build_2d_sincos_position_embedding(
        width,
        height,
        embed_dim,
        temperature,
        device,
    )
    .unwrap();

    // Check shape
    assert_eq!(pos_embed.size(), vec![1, width * height, embed_dim]);

    // Get min/max values
    let min_val = pos_embed.min().double_value(&[]);
    let max_val = pos_embed.max().double_value(&[]);

    println!("Position embeddings:");
    println!("  Shape: {:?}", pos_embed.size());
    println!("  Min: {:.6}", min_val);
    println!("  Max: {:.6}", max_val);

    // CRITICAL: Position embeddings must have NEGATIVE values
    // Bug in N=416: Rust produced [0, 1] instead of [-1, 1]
    assert!(
        min_val < -0.9,
        "Position embeddings must have negative values! Got min={}",
        min_val
    );
    assert!(
        max_val > 0.9,
        "Position embeddings must have positive values! Got max={}",
        max_val
    );

    // Check range is approximately [-1, 1]
    assert!(
        (-1.1..=-0.9).contains(&min_val),
        "Min value should be close to -1.0, got {}",
        min_val
    );
    assert!(
        (0.9..=1.1).contains(&max_val),
        "Max value should be close to 1.0, got {}",
        max_val
    );

    // Count negative vs positive values
    let total_elements = width * height * embed_dim;
    let flattened = pos_embed.flatten(0, -1);
    let negative_mask = flattened.lt(0.0);
    let negative_count = negative_mask.sum(tch::Kind::Int64).int64_value(&[]);
    let negative_pct = (negative_count as f64 / total_elements as f64) * 100.0;

    println!(
        "  Negative values: {}/{} ({:.1}%)",
        negative_count, total_elements, negative_pct
    );

    // Should have roughly 10-50% negative values (depends on sin/cos distribution)
    assert!(
        negative_pct > 5.0,
        "Too few negative values! Only {:.1}%",
        negative_pct
    );
    assert!(
        negative_pct < 95.0,
        "Too many negative values! {:.1}%",
        negative_pct
    );

    println!("✅ Position embeddings correctly produce values in [-1, 1] range");
}
