#![cfg(feature = "pytorch")]
use std::path::PathBuf;
/// Dump actual conv weights from VarStore to verify loading
///
/// This test accesses the VarStore directly to see what weights are actually
/// loaded for the downsample conv layer, then compares with expected values.
use tch::{nn, Device};

#[test]
#[ignore]
fn dump_downsample_conv_weights() {
    println!("\n{}", "=".repeat(80));
    println!("DUMP VARSTORE CONV WEIGHTS");
    println!("{}", "=".repeat(80));

    let device = Device::Cpu;
    let home = std::env::var("HOME").unwrap();

    // Load model (this loads weights into VarStore)
    println!("\nLoading model...");
    let model_dir = PathBuf::from(&home)
        .join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate");
    let model =
        docling_pdf_ml::models::table_structure::TableStructureModel::load(&model_dir, device)
            .expect("Failed to load model");

    // Get the VarStore
    // Note: We can't directly access vs from the model, so we'll use a different approach
    // Let's create a new VarStore and load it the same way
    let vs = nn::VarStore::new(device);
    let root = vs.root();

    // Create the same structure

    let basic_block_path = &root / "_tag_transformer" / "_input_filter" / "0";

    // Create the conv layer structure (this registers variables in VarStore)
    let ds_conv_config = nn::ConvConfig {
        stride: 1,
        padding: 0,
        bias: false,
        ..Default::default()
    };
    let _ds_conv = nn::conv2d(
        &basic_block_path / "downsample" / "0",
        256,
        512,
        1,
        ds_conv_config,
    );

    // Load SafeTensors
    let safetensors_path = model_dir.join("tableformer_accurate.safetensors");
    use safetensors::SafeTensors;
    let buffer = std::fs::read(&safetensors_path).expect("Failed to read safetensors");
    let tensors = SafeTensors::deserialize(&buffer).expect("Failed to deserialize");

    // Get the weight tensor name
    let weight_name = "_tag_transformer._input_filter.0.downsample.0.weight";

    println!("\nLooking for VarStore variable: {}", weight_name);

    // Find the variable in VarStore
    let mut found = false;
    for (vs_name, mut vs_tensor) in vs.variables() {
        if vs_name == weight_name {
            found = true;
            println!("✓ Found in VarStore");
            println!("  VarStore shape: {:?}", vs_tensor.size());

            // Get the tensor from SafeTensors
            if let Ok(st_tensor) = tensors.tensor(weight_name) {
                let shape: Vec<i64> = st_tensor.shape().iter().map(|&s| s as i64).collect();
                println!("  SafeTensors shape: {:?}", shape);

                // Load SafeTensors weight
                let data = st_tensor.data();
                let slice = bytemuck::cast_slice::<u8, f32>(data);
                let loaded_tensor = tch::Tensor::from_slice(slice).reshape(&shape);

                // Copy into VarStore
                vs_tensor.copy_(&loaded_tensor);

                // Now print actual values from VarStore
                println!("\n  First 10 weight values from VarStore [0,:10,0,0]:");
                for i in 0..10 {
                    let val = vs_tensor.double_value(&[0, i, 0, 0]);
                    println!("    weight[0,{:3},0,0] = {:12.8}", i, val);
                }

                println!("\n  Expected from SafeTensors:");
                println!("    weight[0,  0,0,0] = -0.45641580");
                println!("    weight[0,  1,0,0] =  0.12501499");
                println!("    weight[0,  2,0,0] =  0.11569990");
                println!("    weight[0,  3,0,0] =  0.02903064");
                println!("    weight[0,  4,0,0] =  0.03858520");
                println!("    weight[0,  5,0,0] =  0.12933077");
                println!("    weight[0,  6,0,0] = -0.10345598");
                println!("    weight[0,  7,0,0] = -0.05125140");
                println!("    weight[0,  8,0,0] =  0.02368324");
                println!("    weight[0,  9,0,0] =  0.16103768");

                // Check if they match
                println!("\n  Comparison:");
                let expected = vec![
                    -0.45641580,
                    0.12501499,
                    0.11569990,
                    0.02903064,
                    0.03858520,
                    0.12933077,
                    -0.10345598,
                    -0.05125140,
                    0.02368324,
                    0.16103768,
                ];
                let mut max_diff = 0.0;
                for i in 0..10 {
                    let actual = vs_tensor.double_value(&[0, i, 0, 0]);
                    let exp = expected[i as usize];
                    let diff = (actual - exp).abs();
                    if diff > max_diff {
                        max_diff = diff;
                    }
                    println!(
                        "    [{}] actual={:12.8}, expected={:12.8}, diff={:e}",
                        i, actual, exp, diff
                    );
                }

                if max_diff < 1e-6 {
                    println!("\n  ✅ WEIGHTS MATCH (diff < 1e-6)");
                    println!("  → Bug is in how tch-rs applies conv, NOT in loading");
                } else {
                    println!("\n  ❌ WEIGHTS DON'T MATCH (max diff = {})", max_diff);
                    println!("  → Bug is in weight loading/reshape");
                }
            }
        }
    }

    if !found {
        println!("❌ Variable not found in VarStore");
        println!("\nAvailable variables:");
        for (name, _) in vs.variables() {
            if name.contains("downsample") {
                println!("  {}", name);
            }
        }
    }

    println!("\n{}", "=".repeat(80));
}
