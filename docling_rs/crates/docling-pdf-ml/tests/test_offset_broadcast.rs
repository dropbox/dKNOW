#![cfg(feature = "pytorch")]
use tch::{IndexOp, Kind, Tensor};

#[test]
fn test_offset_broadcast() {
    // Create test tensors with known values (matching Python test)
    let offsets_reshaped = Tensor::ones([1, 300, 8, 12, 2], (Kind::Float, tch::Device::Cpu)) * 0.1;

    // n_points_scale = [1, 2, 3, ..., 12] with shape [12, 1]
    let n_points_scale_vec: Vec<f32> = (1..=12).map(|i| i as f32).collect();
    let n_points_scale = Tensor::from_slice(&n_points_scale_vec)
        .to_kind(Kind::Float)
        .view([12, 1]);

    let ref_wh = Tensor::ones([1, 300, 1, 1, 2], (Kind::Float, tch::Device::Cpu)) * 0.5;
    let offset_scale = 0.3;

    println!("offsets_reshaped shape: {:?}", offsets_reshaped.size());
    println!("n_points_scale shape: {:?}", n_points_scale.size());
    println!("ref_wh shape: {:?}", ref_wh.size());
    println!("offset_scale: {}", offset_scale);

    // Compute offset
    let offset = &offsets_reshaped * &n_points_scale * &ref_wh * offset_scale;

    println!("offset shape: {:?}", offset.size());

    // Extract first 12 values along n_points dim: offset[0,0,0,:,0]
    let values = offset.i((0, 0, 0, .., 0));
    println!("offset[0,0,0,:,0]:");
    for i in 0..12 {
        let val = values.double_value(&[i]);
        println!("  [{}] = {:.6}", i, val);
    }

    // Expected: 0.1 * [1,2,3,...,12] * 0.5 * 0.3 = 0.015 * [1,2,3,...,12]
    println!("\nExpected (0.1 * [1..12] * 0.5 * 0.3):");
    for i in 1..=12 {
        let expected = 0.015 * (i as f64);
        println!("  [{}] = {:.6}", i - 1, expected);
    }

    // Check if they match
    let mut all_match = true;
    for i in 0..12 {
        let actual = values.double_value(&[i]);
        let expected = 0.015 * ((i + 1) as f64);
        let diff = (actual - expected).abs();
        if diff > 1e-6 {
            println!(
                "\n❌ Mismatch at [{}]: actual={:.6}, expected={:.6}, diff={:.6e}",
                i, actual, expected, diff
            );
            all_match = false;
        }
    }

    if all_match {
        println!("\n✅ All values match!");
    } else {
        panic!("Broadcasting behavior differs from Python!");
    }
}
