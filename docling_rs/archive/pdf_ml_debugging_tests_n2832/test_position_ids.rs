#![cfg(feature = "pytorch")]
// Test position IDs computation for vision embeddings
use tch::{Device, Tensor};

#[test]
#[ignore] // tch-rs 0.18 searchsorted segfaults - see N=472
fn test_bucketize_basic() {
    // Test basic bucketize functionality
    let boundaries = Tensor::from_slice(&[0.1_f64, 0.2, 0.3]).to_device(Device::Cpu);
    let input = Tensor::from_slice(&[0.05_f64, 0.15, 0.25, 0.35]).to_device(Device::Cpu);

    println!("Boundaries shape: {:?}", boundaries.size());
    println!("Input shape: {:?}", input.size());
    println!("Boundaries: {:?}", boundaries);
    println!("Input: {:?}", input);

    // torch.bucketize(input, boundaries, right=True)
    // Expected: [0, 1, 2, 3]
    // Note: tch binding appears backwards - need input.searchsorted(boundaries)
    let result = input.searchsorted(&boundaries, false, true, "right", Option::<&Tensor>::None);

    println!("Result shape: {:?}", result.size());
    println!("Result: {:?}", result);

    let result_vec: Vec<i64> = result.try_into().unwrap();
    assert_eq!(result_vec, vec![0, 1, 2, 3]);
}

#[test]
#[ignore] // tch-rs 0.18 searchsorted segfaults - see N=472
fn test_position_id_bucketize() {
    // Test the specific bucketing used for position IDs
    let num_patches_per_side = 32;
    let n = num_patches_per_side as f64;

    // Create boundaries: [1/32, 2/32, ..., 31/32]
    let boundaries: Vec<f64> = (1..num_patches_per_side).map(|i| i as f64 / n).collect();
    let boundaries_tensor = Tensor::from_slice(&boundaries).to_device(Device::Cpu);

    println!("Boundaries shape: {:?}", boundaries_tensor.size());
    println!("Boundaries (first 5): {:?}", &boundaries[..5]);

    // Compute fractional coordinates for 32 patches: [0, 1, 2, ..., 31]
    let nb_patches = 32;
    let indices = Tensor::arange(nb_patches, (tch::Kind::Float, Device::Cpu));
    let fractional_coords = &indices / (nb_patches as f64) * (1.0 - 1e-6);

    println!("Fractional coords shape: {:?}", fractional_coords.size());

    // Bucketize
    let bucket_coords = fractional_coords.searchsorted(
        &boundaries_tensor,
        false,
        true,
        "right",
        Option::<&Tensor>::None,
    );

    println!("Bucket coords shape: {:?}", bucket_coords.size());

    let bucket_vec: Vec<i64> = bucket_coords.try_into().unwrap();

    // Expected pattern: [0, 0, 1, 2, 3, ..., 30] (first two are 0)
    println!("Bucket coords (first 10): {:?}", &bucket_vec[..10]);

    // Verify first two are 0
    assert_eq!(bucket_vec[0], 0);
    assert_eq!(bucket_vec[1], 0);

    // Verify rest follows pattern
    for (i, &value) in bucket_vec.iter().enumerate().skip(2).take(8) {
        assert_eq!(value, (i - 1) as i64);
    }
}
