#![cfg(feature = "pytorch")]
/// Test FP16 capability in PyTorch tch-rs
///
/// This test verifies basic FP16 support before implementing full model conversion.
/// Tests tensor operations and precision differences between FP32 and FP16.
///
/// NOTE: FP16 optimization expected to provide 1.3-1.5x speedup on MPS
use tch::{Device, Kind, Tensor};

#[test]
fn test_fp16_tensor_operations() {
    // Set environment variables
    std::env::set_var(
        "DYLD_LIBRARY_PATH",
        "/opt/homebrew/lib/python3.14/site-packages/torch/lib",
    );
    std::env::set_var("DYLD_FALLBACK_LIBRARY_PATH", "/opt/homebrew/opt/llvm/lib");
    std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
    std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");

    let device = Device::Mps;

    // Create FP32 tensor
    let x_fp32 = Tensor::randn([100, 100], (Kind::Float, device));
    let y_fp32 = Tensor::randn([100, 100], (Kind::Float, device));

    // Convert to FP16
    let x_fp16 = x_fp32.to_kind(Kind::Half);
    let y_fp16 = y_fp32.to_kind(Kind::Half);

    // Perform operation in FP16
    let z_fp16 = x_fp16.matmul(&y_fp16);

    // Convert back to FP32 for comparison
    let z_fp32_from_fp16 = z_fp16.to_kind(Kind::Float);
    let z_fp32_direct = x_fp32.matmul(&y_fp32);

    // Check shapes match
    assert_eq!(z_fp32_from_fp16.size(), z_fp32_direct.size());

    // Check values are close (FP16 has lower precision)
    let diff = (z_fp32_from_fp16 - z_fp32_direct).abs().max();
    let diff_val: f32 = diff.try_into().unwrap();

    println!("FP16 vs FP32 max difference: {:.6}", diff_val);

    // FP16 precision: ~3 decimal digits, expect larger differences than FP32
    assert!(diff_val < 1.0, "FP16 error too large: {}", diff_val);
}
