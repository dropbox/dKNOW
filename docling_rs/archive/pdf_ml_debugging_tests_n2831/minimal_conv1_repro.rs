#![cfg(feature = "pytorch")]
use std::path::PathBuf;
/// Minimal reproduction of conv1 divergence
///
/// Same inputs, same weights, different outputs in Rust vs Python
/// This test saves the conv1 weights and input from Rust for external comparison
use tch::{Device, Tensor};

#[test]
#[ignore]
fn test_minimal_conv1_repro() {
    println!("\n{}", "=".repeat(80));
    println!("Minimal Conv1 Reproduction");
    println!("{}", "=".repeat(80));

    let device = Device::Cpu;
    let home = std::env::var("HOME").unwrap();
    let base_path =
        PathBuf::from(&home).join("docling_debug_pdf_parsing/ml_model_inputs/tableformer");

    // Load preprocessed input using DIRECT npyz loading
    // to avoid any ndarray memory layout issues
    println!("\n[1] Loading input...");
    let input_path = base_path.join("table_0_preprocessed_input.npy");

    // Load directly using npyz, exactly like baseline/mod.rs:69-75
    use npyz::NpyFile;
    use std::fs::File;
    let file = File::open(&input_path).expect("Failed to open input file");
    let npy = NpyFile::new(file).expect("Failed to parse npy file");
    let shape: Vec<usize> = npy.shape().iter().map(|&x| x as usize).collect();
    let order = npy.order(); // Get the order
    let data: Vec<f32> = npy.into_vec().expect("Failed to read npy data");

    // Check if Fortran order
    let fortran_order = matches!(order, npyz::Order::Fortran);

    println!("  NPY file metadata:");
    println!("    Shape: {:?}", shape);
    println!("    Order: {:?}", order);
    println!("    Fortran order: {}", fortran_order);
    println!("    Data length: {}", data.len());

    // Convert to tensor shape
    let tensor_shape: Vec<i64> = shape.iter().map(|&x| x as i64).collect();

    // Create tensor - if data is Fortran-order, we need to handle it correctly
    let input_tensor = if fortran_order {
        // Fortran order (column-major): need to transpose
        // Load with reversed shape, then transpose back
        let reversed_shape: Vec<i64> = tensor_shape.iter().copied().rev().collect();
        let temp = Tensor::from_slice(&data)
            .to(device)
            .reshape(&reversed_shape);
        // Transpose to get correct order: reverse all dimensions
        let ndims = shape.len();
        let perm: Vec<i64> = (0..ndims as i64).rev().collect();
        temp.permute(&perm)
    } else {
        // C order (row-major): standard loading
        Tensor::from_slice(&data).to(device).reshape(&tensor_shape)
    };
    println!("  Input shape: {:?}", input_tensor.size());
    println!(
        "  Input [0,0,0,0]: {:.10}",
        input_tensor.double_value(&[0, 0, 0, 0])
    );
    println!(
        "  Input [0,1,0,0]: {:.10}",
        input_tensor.double_value(&[0, 1, 0, 0])
    );
    println!(
        "  Input [0,2,0,0]: {:.10}",
        input_tensor.double_value(&[0, 2, 0, 0])
    );

    // Save input for Python comparison
    let input_save_path = base_path.join("rust_conv1_input.pt");
    input_tensor
        .save(&input_save_path)
        .expect("Failed to save input");
    println!("  ✓ Saved input to: {}", input_save_path.display());

    // Load model
    println!("\n[2] Loading model...");
    let model_dir = PathBuf::from(&home)
        .join(".cache/huggingface/hub/models--ds4sd--docling-models/snapshots/fc0f2d45e2218ea24bce5045f58a389aed16dc23/model_artifacts/tableformer/accurate");
    let model =
        docling_pdf_ml::models::table_structure::TableStructureModel::load(&model_dir, device)
            .expect("Failed to load model");
    println!("  ✓ Model loaded");

    // Extract conv1 weights from VarStore
    println!("\n[3] Extracting conv1 weights from VarStore...");
    let vs = &model.vs;

    // Access variables from VarStore
    let variables = vs.variables();
    println!("  Total variables in VarStore: {}", variables.len());

    // Find conv1 weight
    let mut conv1_weight = None;
    for (name, tensor) in variables.iter() {
        if name == "_encoder._resnet.0.weight" {
            println!("  Found conv1 weight: {}", name);
            println!("  Conv1 weight shape: {:?}", tensor.size());
            println!(
                "  Conv1 weight [0,0,0,0]: {:.10}",
                tensor.double_value(&[0, 0, 0, 0])
            );
            println!("  Conv1 weight first 5 values:");
            for i in 0..5 {
                println!(
                    "    [0,0,0,{}]: {:.10}",
                    i,
                    tensor.double_value(&[0, 0, 0, i])
                );
            }
            conv1_weight = Some(tensor);
            break;
        }
    }

    let conv1_weight = conv1_weight.expect("Failed to find conv1 weight");

    // Save conv1 weights for Python comparison
    let weight_save_path = base_path.join("rust_conv1_weight.pt");
    conv1_weight
        .save(&weight_save_path)
        .expect("Failed to save conv1 weight");
    println!("  ✓ Saved weight to: {}", weight_save_path.display());

    // Run conv1
    println!("\n[4] Running conv1...");
    let conv1_output = input_tensor.apply(&model.encoder.conv1);
    println!("  Output shape: {:?}", conv1_output.size());
    println!(
        "  Output [0,0,0,0]: {:.10}",
        conv1_output.double_value(&[0, 0, 0, 0])
    );
    print!("  Output [0,:5,0,0]: [");
    for i in 0..5 {
        print!("{:.6}", conv1_output.double_value(&[0, i, 0, 0]));
        if i < 4 {
            print!(", ");
        }
    }
    println!("]");

    // Save output for Python comparison
    let output_save_path = base_path.join("rust_conv1_output.pt");
    conv1_output
        .save(&output_save_path)
        .expect("Failed to save output");
    println!("  ✓ Saved output to: {}", output_save_path.display());

    println!("\n{}", "=".repeat(80));
    println!("Expected Python output: [-0.099, 0.020, -0.029, -0.590, 0.034]");
    println!("Actual Rust output:     [-0.977, 0.049, 0.671, -0.296, -0.290]");
    println!("{}", "=".repeat(80));

    println!("\nTo compare:");
    println!("  1. Run: python3 minimal_conv1_repro_python.py");
    println!("  2. Compare outputs");
}
