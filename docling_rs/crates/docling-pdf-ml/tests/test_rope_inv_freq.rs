#![cfg(feature = "pytorch")]
/// Test to verify RoPE inv_freq computation matches Python formula
///
/// Formula: inv_freq[i] = 1 / (base^(2*i/dim)) = exp(-2*i/dim * ln(base))
use docling_pdf_ml::models::code_formula::text_decoder::RotaryEmbedding;
use tch::{nn, Device, Kind, Tensor};

#[test]
fn test_rope_inv_freq_formula() {
    let device = Device::Cpu;
    let vs = nn::VarStore::new(device);
    let root = vs.root();

    // Standard LLaMA RoPE parameters
    let dim = 128;
    let max_position_embeddings = 8192;
    let base = 10000.0_f64;

    // Create RoPE
    let rope = RotaryEmbedding::new(&root, dim, max_position_embeddings, base);

    // Get inv_freq tensor (this is private, so we'll test via forward pass)
    // Instead, manually compute expected values
    let half_dim = dim / 2;
    let indices = Tensor::arange(half_dim, (Kind::Float, device));
    let exponents = (indices * 2.0) / (dim as f64);
    let expected_inv_freq = (exponents.neg() * base.ln()).exp();

    // Extract via RoPE forward pass
    let (cos, sin) = rope.forward(1, device);

    // For position 0: cos should be all 1s, sin should be all 0s
    let cos_pos0 = cos.get(0).get(0);
    let sin_pos0 = sin.get(0).get(0);

    let cos_values: Vec<f64> = cos_pos0.try_into().unwrap();
    let sin_values: Vec<f64> = sin_pos0.try_into().unwrap();

    // Check position 0
    for &val in &cos_values {
        assert!((val - 1.0).abs() < 1e-5, "cos at position 0 should be 1.0");
    }
    for &val in &sin_values {
        assert!(val.abs() < 1e-5, "sin at position 0 should be 0.0");
    }

    // Now test position 1: cos(inv_freq), sin(inv_freq)
    let (cos, sin) = rope.forward(2, device);
    let cos_pos1 = cos.get(0).get(1);
    let sin_pos1 = sin.get(0).get(1);

    let expected_inv_freq_vec: Vec<f64> = expected_inv_freq.try_into().unwrap();

    // Concatenate expected_inv_freq with itself (RoPE does this)
    let mut expected_freqs = expected_inv_freq_vec.clone();
    expected_freqs.extend_from_slice(&expected_inv_freq_vec);

    let expected_cos: Vec<f64> = expected_freqs.iter().map(|f| f.cos()).collect();
    let expected_sin: Vec<f64> = expected_freqs.iter().map(|f| f.sin()).collect();

    let cos_values: Vec<f64> = cos_pos1.try_into().unwrap();
    let sin_values: Vec<f64> = sin_pos1.try_into().unwrap();

    // Compare first 10 values
    println!("\n=== RoPE inv_freq Verification ===");
    println!("Position 1 (first 10 values):");
    println!("Expected cos: {:?}", &expected_cos[..10]);
    println!("Actual cos:   {:?}", &cos_values[..10]);
    println!("Expected sin: {:?}", &expected_sin[..10]);
    println!("Actual sin:   {:?}", &sin_values[..10]);

    // Check tolerance
    for (i, (&expected, &actual)) in expected_cos.iter().zip(&cos_values).enumerate() {
        let diff = (expected - actual).abs();
        assert!(
            diff < 1e-5,
            "cos[{}] mismatch: expected {}, got {}, diff {}",
            i,
            expected,
            actual,
            diff
        );
    }

    for (i, (&expected, &actual)) in expected_sin.iter().zip(&sin_values).enumerate() {
        let diff = (expected - actual).abs();
        assert!(
            diff < 1e-5,
            "sin[{}] mismatch: expected {}, got {}, diff {}",
            i,
            expected,
            actual,
            diff
        );
    }

    println!("✅ RoPE inv_freq formula verified!");
}

#[test]
fn test_rope_inv_freq_values() {
    // Test specific known values from Python
    let device = Device::Cpu;
    let vs = nn::VarStore::new(device);
    let root = vs.root();

    let dim = 128;
    let base = 10000.0;

    let rope = RotaryEmbedding::new(&root, dim, 8192, base);

    // Generate position 1 embeddings
    let (cos, sin) = rope.forward(2, device);
    let cos_pos1 = cos.get(0).get(1);
    let sin_pos1 = sin.get(0).get(1);

    // Expected values from Python (position 1, first few dimensions)
    // These are based on: inv_freq = [1.0, 0.8659643, 0.7498942, ...]
    // freqs at position 1 = [1.0, 0.8659643, ...] (concatenated twice)
    let expected_cos_samples = vec![
        (0, 1.0_f64.cos()),
        (1, 0.8659643530845642_f64.cos()),
        (2, 0.7498942017555237_f64.cos()),
    ];

    let expected_sin_samples = vec![
        (0, 1.0_f64.sin()),
        (1, 0.8659643530845642_f64.sin()),
        (2, 0.7498942017555237_f64.sin()),
    ];

    let cos_values: Vec<f64> = cos_pos1.try_into().unwrap();
    let sin_values: Vec<f64> = sin_pos1.try_into().unwrap();

    println!("\n=== RoPE Known Values Verification ===");
    for (idx, expected) in &expected_cos_samples {
        let actual = cos_values[*idx];
        let diff = (expected - actual).abs();
        println!(
            "cos[{}]: expected {:.6}, actual {:.6}, diff {:.6}",
            idx, expected, actual, diff
        );
        assert!(
            diff < 1e-5,
            "cos[{}] mismatch: expected {}, got {}, diff {}",
            idx,
            expected,
            actual,
            diff
        );
    }

    for (idx, expected) in &expected_sin_samples {
        let actual = sin_values[*idx];
        let diff = (expected - actual).abs();
        println!(
            "sin[{}]: expected {:.6}, actual {:.6}, diff {:.6}",
            idx, expected, actual, diff
        );
        assert!(
            diff < 1e-5,
            "sin[{}] mismatch: expected {}, got {}, diff {}",
            idx,
            expected,
            actual,
            diff
        );
    }

    println!("✅ RoPE known values verified!");
}
