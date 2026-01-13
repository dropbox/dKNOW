#![cfg(feature = "pytorch")]
/// Test if tch-rs BatchNorm uses running stats in eval mode
use tch::{nn, Device, Kind, Tensor};

#[test]
#[ignore]
fn test_batch_norm_eval_mode() {
    println!("\n{}", "=".repeat(80));
    println!("Test BatchNorm Eval Mode - Does it use running stats?");
    println!("{}\n", "=".repeat(80));

    let device = Device::Cpu;
    let vs = nn::VarStore::new(device);
    let root = vs.root();

    // Create batch norm
    let bn = nn::batch_norm2d(&root / "bn", 512, Default::default());

    // Set custom running stats to verify they're being used
    println!("Setting custom running stats...");
    let running_mean = [-15.414, -4.369, 6.170, -9.220, -6.957];
    let running_var = [265.716, 132.469, 137.781, 142.144, 193.560];

    // We need to manually set these via the VarStore
    // Actually, tch-rs doesn't expose running_mean/running_var directly
    // They should be loaded from the weights file

    println!("Creating test input...");
    let mut input = Tensor::zeros([1, 512, 28, 28], (Kind::Float, device));
    // Set position [0,:5,0,0] to known values
    input = input.narrow(1, 0, 5).fill_(1.0);
    println!("  Input [0,:5,0,0]: all 1.0");

    println!("\nApplying batch norm with train=true...");
    let output_train = input.apply_t(&bn, true);
    let val_train: Vec<f32> = output_train
        .narrow(1, 0, 5)
        .narrow(2, 0, 1)
        .narrow(3, 0, 1)
        .squeeze()
        .try_into()
        .unwrap();
    println!("  Output [0,:5,0,0]: {:?}", val_train);

    println!("\nApplying batch norm with train=false...");
    let output_eval = input.apply_t(&bn, false);
    let val_eval: Vec<f32> = output_eval
        .narrow(1, 0, 5)
        .narrow(2, 0, 1)
        .narrow(3, 0, 1)
        .squeeze()
        .try_into()
        .unwrap();
    println!("  Output [0,:5,0,0]: {:?}", val_eval);

    println!("\nDifference between train and eval:");
    println!("  train: {:?}", val_train);
    println!("  eval:  {:?}", val_eval);

    if val_train
        .iter()
        .zip(val_eval.iter())
        .all(|(a, b)| (a - b).abs() < 1e-6)
    {
        println!("\n❌ TRAIN AND EVAL PRODUCE SAME OUTPUT!");
        println!("   This suggests running stats are NOT being used in eval mode.");
    } else {
        println!("\n✓ Train and eval produce different outputs (expected)");
    }

    println!("\n{}", "=".repeat(80));
}
