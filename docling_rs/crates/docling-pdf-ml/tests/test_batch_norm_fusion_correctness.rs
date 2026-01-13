#![cfg(feature = "pytorch")]
// Test batch norm fusion correctness
// Verify that fused conv+bn produces same output as separate conv+bn
use tch::{nn, Device, Kind, Tensor};

#[test]
fn test_batch_norm_fusion_simple() {
    // Set environment variables
    std::env::set_var(
        "DYLD_LIBRARY_PATH",
        "/opt/homebrew/lib/python3.14/site-packages/torch/lib",
    );
    std::env::set_var("DYLD_FALLBACK_LIBRARY_PATH", "/opt/homebrew/opt/llvm/lib");
    std::env::set_var("LIBTORCH_USE_PYTORCH", "1");
    std::env::set_var("LIBTORCH_BYPASS_VERSION_CHECK", "1");

    // Create a simple conv+bn layer
    let vs = nn::VarStore::new(Device::Cpu);
    let root = vs.root();

    // Conv2d: 3 input channels, 16 output channels, 3x3 kernel
    let conv_config = nn::ConvConfig {
        stride: 1,
        padding: 1,
        bias: false, // No bias, will be in batch norm
        ..Default::default()
    };

    let conv = nn::conv2d(&root / "conv", 3, 16, 3, conv_config);
    let bn = nn::batch_norm2d(&root / "bn", 16, Default::default());

    // Note: tch-rs initializes weights automatically:
    // - conv.weight: kaiming_uniform
    // - bn.weight: ones
    // - bn.bias: zeros
    // - bn.running_mean: zeros
    // - bn.running_var: ones
    // We'll use these default initializations

    // Create test input
    let input = Tensor::randn([1, 3, 8, 8], (Kind::Float, Device::Cpu));

    // Forward pass: conv -> bn
    let output_unfused = tch::no_grad(|| {
        let x = input.apply(&conv);
        x.apply_t(&bn, false) // eval mode
    });

    println!("Unfused output shape: {:?}", output_unfused.size());
    println!(
        "Unfused output mean: {:.6}",
        output_unfused.mean(Kind::Float).double_value(&[])
    );
    println!(
        "Unfused output std: {:.6}",
        output_unfused.std(false).double_value(&[])
    );

    // Now fuse the weights
    let (fused_weight, fused_bias) = {
        let variables = vs.variables();
        let conv_weight = variables.get("conv.weight").unwrap();
        let bn_weight = variables.get("bn.weight").unwrap();
        let bn_bias = variables.get("bn.bias").unwrap();
        let bn_mean = variables.get("bn.running_mean").unwrap();
        let bn_var = variables.get("bn.running_var").unwrap();

        // Apply fusion formula
        let bn_eps = 1e-5;
        let std = (bn_var + bn_eps).sqrt();
        let scale = bn_weight / &std;

        // W_fused = γ * W / sqrt(σ² + ε)
        let scale_broadcast = scale.view([16, 1, 1, 1]);
        let w_fused = conv_weight * &scale_broadcast;

        // b_fused = γ * (0 - μ) / sqrt(σ² + ε) + β  (conv has no bias)
        let b_fused = &scale * (0.0 - bn_mean) + bn_bias;

        (w_fused, b_fused)
    };

    // Create fused conv layer
    let vs_fused = nn::VarStore::new(Device::Cpu);
    let root_fused = vs_fused.root();

    let conv_fused_config = nn::ConvConfig {
        stride: 1,
        padding: 1,
        bias: true, // Now has bias
        ..Default::default()
    };

    let conv_fused = nn::conv2d(&root_fused / "conv", 3, 16, 3, conv_fused_config);

    // Load fused weights
    tch::no_grad(|| {
        let mut variables = vs_fused.variables_.lock().unwrap();
        if let Some(w) = variables.named_variables.get_mut("conv.weight") {
            w.copy_(&fused_weight);
        }
        if let Some(b) = variables.named_variables.get_mut("conv.bias") {
            b.copy_(&fused_bias);
        }
    });

    // Forward pass: fused conv only
    let output_fused = tch::no_grad(|| input.apply(&conv_fused));

    println!("Fused output shape: {:?}", output_fused.size());
    println!(
        "Fused output mean: {:.6}",
        output_fused.mean(Kind::Float).double_value(&[])
    );
    println!(
        "Fused output std: {:.6}",
        output_fused.std(false).double_value(&[])
    );

    // Compare outputs
    let diff = (&output_fused - &output_unfused).abs();
    let max_diff = diff.max().double_value(&[]);
    let mean_diff = diff.mean(Kind::Float).double_value(&[]);

    println!("\nDifference:");
    println!("  Max diff: {:.6e}", max_diff);
    println!("  Mean diff: {:.6e}", mean_diff);

    // Check if outputs match within tolerance
    assert!(
        max_diff < 1e-5,
        "Fused and unfused outputs differ! Max diff: {:.6e}",
        max_diff
    );

    println!("✅ Batch norm fusion test passed!");
}
