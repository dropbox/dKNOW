#![cfg(feature = "pytorch")]
// Weight verification test: Compare Rust VarStore weights with Python HF model
//
// This test loads the CodeFormula model and extracts sample weights to verify
// that SafeTensors loading is working correctly.

use docling_pdf_ml::models::code_formula::CodeFormulaModel;
use tch::Device;

#[test]
#[ignore]
fn test_weight_verification() {
    let sep = "=".repeat(80);
    println!("\n{}", sep);
    println!("RUST WEIGHT VERIFICATION");
    println!("{}", sep);

    // Load model
    let home_dir = std::env::var("HOME").expect("HOME not set");
    let model_slug = "ds4sd--CodeFormulaV2";
    let cache_path = format!("{home_dir}/.cache/huggingface/hub/models--{model_slug}/snapshots");

    let model_dir = std::fs::read_dir(&cache_path)
        .expect("Failed to read snapshots directory")
        .next()
        .expect("No snapshot found")
        .unwrap()
        .path();

    println!("\nLoading model from: {:?}", model_dir);

    let device = Device::Cpu;
    let model =
        CodeFormulaModel::from_pretrained(&model_dir, device).expect("Failed to load model");

    println!("Model loaded successfully");

    // Extract weights from VarStore
    let vs = model.inner_model().var_store();

    println!("\n{}", sep);
    println!("WEIGHT TENSORS");
    println!("{}", sep);

    // Weight paths to check (matching Python script)
    let weights_to_check = vec![
        "model.text_model.embed_tokens.weight",
        "model.vision_model.encoder.layers.0.self_attn.q_proj.weight",
        "model.vision_model.encoder.layers.0.self_attn.k_proj.weight",
        "model.vision_model.encoder.layers.0.self_attn.v_proj.weight",
    ];

    for weight_path in weights_to_check {
        println!("\n{}:", weight_path);

        match vs.variables().get(weight_path) {
            Some(tensor) => {
                let shape = tensor.size();
                let dtype = tensor.kind();

                println!("  Shape: {:?}", shape);
                println!("  Dtype: {:?}", dtype);

                // Get statistics
                let min = tensor.min();
                let max = tensor.max();
                let mean = tensor.mean(tch::Kind::Float);
                let std = tensor.std(false);

                println!(
                    "  Min: {:.6}, Max: {:.6}",
                    f64::try_from(&min).unwrap(),
                    f64::try_from(&max).unwrap()
                );
                println!(
                    "  Mean: {:.6}, Std: {:.6}",
                    f64::try_from(&mean).unwrap(),
                    f64::try_from(&std).unwrap()
                );

                // Get first and last 5 elements
                let flat = tensor.flatten(0, -1);
                let first_5: Vec<f32> = (0..5.min(flat.size()[0]))
                    .map(|i| f32::try_from(&flat.get(i)).unwrap())
                    .collect();
                let last_start = (flat.size()[0] - 5).max(0);
                let last_5: Vec<f32> = (last_start..flat.size()[0])
                    .map(|i| f32::try_from(&flat.get(i)).unwrap())
                    .collect();

                println!("  First 5 elements: {:?}", first_5);
                println!("  Last 5 elements: {:?}", last_5);
            }
            None => {
                println!("  ERROR: Weight not found in VarStore");
                println!("  Available keys (first 20):");
                for (i, (key, _)) in vs.variables().iter().enumerate() {
                    if i < 20 {
                        println!("    {}", key);
                    }
                }
                break;
            }
        }
    }

    let sep2 = "=".repeat(80);
    println!("\n{}", sep2);
    println!("COMPARISON INSTRUCTIONS");
    println!("{}", sep2);
    println!(
        "
1. Run: python3 debug_scripts/extract_model_weights.py
2. Compare output above with Python output
3. Check that:
   - Shapes match exactly
   - Dtypes match (float32)
   - Min/Max/Mean/Std are close (within 1e-6)
   - First 5 and last 5 elements match (within 1e-6)

If values differ significantly → SafeTensors loading bug
If values match → Bug is elsewhere (image features, forward pass, etc.)
"
    );
}
