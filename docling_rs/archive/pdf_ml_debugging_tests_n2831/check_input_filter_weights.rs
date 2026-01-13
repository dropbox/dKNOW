#![cfg(feature = "pytorch")]
use std::path::PathBuf;
/// Check if input_filter weights are loaded correctly
use tch::Device;

#[test]
#[ignore]
fn test_check_input_filter_weights() {
    println!("\n{}", "=".repeat(80));
    println!("Check Input Filter Weights");
    println!("{}", "=".repeat(80));

    let device = Device::Cpu;
    let home = std::env::var("HOME").unwrap();

    // Load model
    println!("\nLoading model...");
    let model_dir = PathBuf::from(&home)
        .join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate");
    let model =
        docling_pdf_ml::models::table_structure::TableStructureModel::load(&model_dir, device)
            .expect("Failed to load model");

    // Access VarStore
    let vs = &model.vs;
    let variables = vs.variables();

    println!("\nChecking input_filter.0.bn1 running_mean:");
    for (name, tensor) in variables.iter() {
        if name.contains("_tag_transformer._input_filter.0.bn1.running_mean") {
            println!("  Found: {}", name);
            println!("  Shape: {:?}", tensor.size());
            print!("  First 5 values: [");
            for i in 0..5 {
                print!("{:.6}", tensor.double_value(&[i]));
                if i < 4 {
                    print!(", ");
                }
            }
            println!("]");
            println!("  Expected: [-15.41, -4.37, 6.17, -9.22, -6.96]");
        }
    }

    println!("\nChecking input_filter.0.bn1.running_var:");
    for (name, tensor) in variables.iter() {
        if name.contains("_tag_transformer._input_filter.0.bn1.running_var") {
            println!("  Found: {}", name);
            println!("  Shape: {:?}", tensor.size());
            print!("  First 5 values: [");
            for i in 0..5 {
                print!("{:.6}", tensor.double_value(&[i]));
                if i < 4 {
                    print!(", ");
                }
            }
            println!("]");
            println!("  Expected: [265.72, 132.47, 137.78, 142.14, 193.56]");
        }
    }

    println!("\nChecking input_filter.0.conv1.weight:");
    for (name, tensor) in variables.iter() {
        if name == "_tag_transformer._input_filter.0.conv1.weight" {
            println!("  Found: {}", name);
            println!("  Shape: {:?}", tensor.size());
            println!(
                "  First value [0,0,0,0]: {:.6}",
                tensor.double_value(&[0, 0, 0, 0])
            );
        }
    }

    println!("\n{}", "=".repeat(80));
}
