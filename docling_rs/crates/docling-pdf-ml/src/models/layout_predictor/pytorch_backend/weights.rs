// Weight loading utilities for RT-DETR v2 PyTorch backend
// Handles loading model.safetensors and mapping to Rust module structure

use safetensors::SafeTensors;
use std::collections::HashSet;
use std::path::Path;
use tch::{nn, Kind, Tensor};

/// Convert HuggingFace weight key to tch-rs format
///
/// HuggingFace uses "." for both module hierarchy AND array indices (e.g., "layers.0.weight")
/// tch-rs ALSO uses "." when you use the "/" path operator (e.g., vs / "layers" / 0 creates "layers.0")
///
/// Therefore, no conversion is needed - HF keys match tch-rs keys directly!
///
/// Examples:
///   HF: "model.decoder.layers.0.self_attn.weight" → same in tch
///   HF: "model.backbone.embedder.embedder.0.convolution.weight" → same in tch
fn convert_hf_key_to_tch(hf_key: &str) -> String {
    // No conversion needed - HF keys match tch-rs format when using "/" operator
    hf_key.to_string()
}

/// Fuse batch normalization into convolution weights
///
/// This function eliminates batch norm as a separate operation by folding its parameters
/// into the convolution weights and bias. This is mathematically exact for inference mode
/// (eval mode with frozen batch norm statistics).
///
/// # Formula
/// ```text
/// W_fused = γ * W / sqrt(σ² + ε)
/// b_fused = γ * (b - μ) / sqrt(σ² + ε) + β
/// ```
///
/// Where:
/// - W, b = convolution weight and bias
/// - γ, β = batch norm weight and bias
/// - μ, σ² = batch norm running_mean and running_var
/// - ε = batch norm epsilon (typically 1e-5)
///
/// # Arguments
/// * `conv_weight` - Convolution weight tensor [out_channels, in_channels, kh, kw]
/// * `conv_bias` - Optional convolution bias tensor [out_channels] (None if bias=false)
/// * `bn_weight` - Batch norm weight (γ) tensor [out_channels]
/// * `bn_bias` - Batch norm bias (β) tensor [out_channels]
/// * `bn_running_mean` - Batch norm running mean (μ) tensor [out_channels]
/// * `bn_running_var` - Batch norm running variance (σ²) tensor [out_channels]
/// * `bn_eps` - Batch norm epsilon (typically 1e-5)
///
/// # Returns
/// * `(Tensor, Tensor)` - Fused (weight, bias) tensors
///
/// # Performance Impact
/// Eliminates batch norm operation which consumes 64.1% of ConvNorm time (N=499 profiling).
/// Expected: 63.8% ConvNorm speedup → 0.4% pipeline speedup.
fn fuse_conv_bn(
    conv_weight: &Tensor,
    conv_bias: Option<&Tensor>,
    bn_weight: &Tensor,
    bn_bias: &Tensor,
    bn_running_mean: &Tensor,
    bn_running_var: &Tensor,
    bn_eps: f64,
) -> Result<(Tensor, Tensor), String> {
    // Validate shapes
    let conv_shape = conv_weight.size();
    if conv_shape.len() != 4 {
        return Err(format!(
            "Expected 4D conv weight [out_ch, in_ch, kh, kw], got {:?}",
            conv_shape
        ));
    }

    let out_channels = conv_shape[0];

    // Ensure all batch norm tensors have correct shape [out_channels]
    for (name, tensor) in [
        ("bn_weight", bn_weight),
        ("bn_bias", bn_bias),
        ("bn_running_mean", bn_running_mean),
        ("bn_running_var", bn_running_var),
    ] {
        let size = tensor.size();
        if size.len() != 1 || size[0] != out_channels {
            return Err(format!(
                "{} shape mismatch: expected [{}], got {:?}",
                name, out_channels, size
            ));
        }
    }

    // Check conv_bias if present
    if let Some(bias) = conv_bias {
        let size = bias.size();
        if size.len() != 1 || size[0] != out_channels {
            return Err(format!(
                "conv_bias shape mismatch: expected [{}], got {:?}",
                out_channels, size
            ));
        }
    }

    // Compute scale = γ / sqrt(σ² + ε)
    // Shape: [out_channels]
    let std = (bn_running_var + bn_eps).sqrt();
    let scale = bn_weight / &std;

    // Fuse into convolution weight: W_fused = γ * W / sqrt(σ² + ε)
    // Broadcast scale from [out_channels] to [out_channels, 1, 1, 1]
    let scale_broadcast = scale.view([out_channels, 1, 1, 1]);
    let w_fused = conv_weight * &scale_broadcast;

    // Fuse into bias: b_fused = γ * (b - μ) / sqrt(σ² + ε) + β
    // If conv has no bias, treat it as zeros
    let conv_bias_or_zeros = if let Some(bias) = conv_bias {
        bias.shallow_clone()
    } else {
        Tensor::zeros([out_channels], (Kind::Float, conv_weight.device()))
    };

    let b_fused = &scale * (&conv_bias_or_zeros - bn_running_mean) + bn_bias;

    Ok((w_fused, b_fused))
}

/// Load weights from safetensors file into existing VarStore
///
/// # IMPORTANT
/// This function loads weights into an EXISTING VarStore that already has
/// variables registered. You must create the model FIRST (which registers
/// variables), then call this to populate them.
///
/// HuggingFace models use "." as separators with numeric indices: "layers.0.weight"
/// tch-rs also uses "." but requires indices use "_": "layers_0.weight"
/// This function automatically converts HuggingFace format to tch-rs format.
///
/// # Arguments
/// * `vs` - VarStore with variables already registered
/// * `path` - Path to model.safetensors file
///
/// # Returns
/// * `Result<()>` - Ok if weights loaded successfully
///
/// # Example
/// ```no_run
/// use docling_pdf_ml::models::layout_predictor::pytorch_backend::weights::load_weights_into;
/// use docling_pdf_ml::models::layout_predictor::pytorch_backend::model::{RTDetrV2Config, RTDetrV2ForObjectDetection};
/// use tch::{nn, Device};
/// use std::path::Path;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut vs = nn::VarStore::new(Device::Cpu);
/// let config = RTDetrV2Config::default();
/// let model = RTDetrV2ForObjectDetection::new(&vs.root(), config)?;
/// load_weights_into(&mut vs, Path::new("model.safetensors"))?;
/// # Ok(())
/// # }
/// ```
pub fn load_weights_into(vs: &mut nn::VarStore, path: &Path) -> Result<(), String> {
    // Load safetensors file manually
    // We can't use vs.load() because HuggingFace uses "." separators
    // but tch-rs requires "/" separators

    use std::collections::HashMap;
    use tch::{Kind, Tensor};

    // Load safetensors file
    let buffer = std::fs::read(path)
        .map_err(|e| format!("Failed to read safetensors file {:?}: {}", path, e))?;

    let tensors = SafeTensors::deserialize(&buffer)
        .map_err(|e| format!("Failed to deserialize safetensors: {}", e))?;

    log::debug!("Loaded {} tensors from safetensors", tensors.len());

    // Convert safetensors to tch tensors and convert keys
    // HuggingFace uses "." as separator everywhere, including for array indices: "layers.0.weight"
    // tch-rs also uses "." as separator, but doesn't allow "." within segment names
    // We need to detect numeric array indices and replace ".<num>." with "_<num>."
    //
    // Examples:
    //   HF: "model.decoder.layers.0.self_attn.weight"
    //   tch: "model.decoder.layers_0.self_attn.weight"
    //
    //   HF: "model.decoder.class_embed.0.weight"
    //   tch: "model.decoder.class_embed_0.weight"

    let mut converted_weights: HashMap<String, Tensor> = HashMap::new();
    for (idx, name) in tensors.names().into_iter().enumerate() {
        log::debug!("Loading tensor {}/{}: {}", idx + 1, tensors.len(), name);

        let tensor_view = tensors
            .tensor(name)
            .map_err(|e| format!("Failed to get tensor {}: {}", name, e))?;

        // Convert safetensors tensor to tch Tensor
        let shape: Vec<i64> = tensor_view.shape().iter().map(|&x| x as i64).collect();
        let data = tensor_view.data();

        // Convert bytes to typed slice and create tensor
        // CRITICAL: Must copy to owned Vec to avoid C++ exception from borrowed safetensors data
        let tensor = match tensor_view.dtype() {
            safetensors::Dtype::F32 => {
                let slice: &[f32] = bytemuck::cast_slice(data);
                let owned: Vec<f32> = slice.to_vec(); // Copy to owned memory
                Tensor::from_slice(&owned).reshape(&shape)
            }
            safetensors::Dtype::F16 => {
                let slice: &[half::f16] = bytemuck::cast_slice(data);
                let f32_vec: Vec<f32> = slice.iter().map(|x| x.to_f32()).collect();
                Tensor::from_slice(&f32_vec)
                    .reshape(&shape)
                    .to_kind(Kind::Half)
            }
            safetensors::Dtype::I64 => {
                let slice: &[i64] = bytemuck::cast_slice(data);
                let owned: Vec<i64> = slice.to_vec(); // Copy to owned memory
                Tensor::from_slice(&owned).reshape(&shape)
            }
            safetensors::Dtype::I32 => {
                let slice: &[i32] = bytemuck::cast_slice(data);
                let owned: Vec<i32> = slice.to_vec(); // Copy to owned memory
                Tensor::from_slice(&owned).reshape(&shape)
            }
            safetensors::Dtype::I16 => {
                let slice: &[i16] = bytemuck::cast_slice(data);
                let owned: Vec<i16> = slice.to_vec(); // Copy to owned memory
                Tensor::from_slice(&owned).reshape(&shape)
            }
            safetensors::Dtype::I8 => {
                let slice: &[i8] = bytemuck::cast_slice(data);
                let owned: Vec<i8> = slice.to_vec(); // Copy to owned memory
                Tensor::from_slice(&owned).reshape(&shape)
            }
            safetensors::Dtype::U8 => {
                let slice: &[u8] = data; // Already u8
                let owned: Vec<u8> = slice.to_vec(); // Copy to owned memory
                Tensor::from_slice(&owned).reshape(&shape)
            }
            _ => return Err(format!("Unsupported dtype: {:?}", tensor_view.dtype())),
        };

        // Convert key format
        let var_key = convert_hf_key_to_tch(name);
        converted_weights.insert(var_key, tensor);
    }

    // === BATCH NORM FUSION ===
    // Fuse batch norm into convolution weights for all ConvNormLayers
    // This eliminates batch norm as a separate operation (64.1% of ConvNorm time from N=499 profiling)
    //
    // Pattern: For each {prefix}.conv.weight, check if {prefix}.norm.{weight,bias,running_mean,running_var} exist
    // If yes, fuse them into {prefix}.conv.{weight,bias} using fuse_conv_bn()
    log::debug!("\n=== Batch Norm Fusion ===");

    let bn_eps = 1e-5; // Standard PyTorch batch norm epsilon
    let mut fused_count = 0;
    let keys_to_process: Vec<String> = converted_weights.keys().cloned().collect();

    for key in &keys_to_process {
        // Check if this is a conv.weight key
        if key.ends_with(".conv.weight") {
            // Extract prefix (everything before ".conv.weight")
            let prefix = &key[..key.len() - ".conv.weight".len()];

            // Build expected batch norm keys
            let bn_weight_key = format!("{}.norm.weight", prefix);
            let bn_bias_key = format!("{}.norm.bias", prefix);
            let bn_mean_key = format!("{}.norm.running_mean", prefix);
            let bn_var_key = format!("{}.norm.running_var", prefix);
            let conv_bias_key = format!("{}.conv.bias", prefix);

            // Check if all batch norm parameters exist
            let has_bn = converted_weights.contains_key(&bn_weight_key)
                && converted_weights.contains_key(&bn_bias_key)
                && converted_weights.contains_key(&bn_mean_key)
                && converted_weights.contains_key(&bn_var_key);

            if has_bn {
                // Get tensors
                let conv_weight = &converted_weights[key];
                let conv_bias = converted_weights.get(&conv_bias_key);
                let bn_weight = &converted_weights[&bn_weight_key];
                let bn_bias = &converted_weights[&bn_bias_key];
                let bn_mean = &converted_weights[&bn_mean_key];
                let bn_var = &converted_weights[&bn_var_key];

                // Fuse
                match fuse_conv_bn(
                    conv_weight,
                    conv_bias,
                    bn_weight,
                    bn_bias,
                    bn_mean,
                    bn_var,
                    bn_eps,
                ) {
                    Ok((fused_weight, fused_bias)) => {
                        // Debug: log first fused layer details (before moving)
                        if fused_count == 0 {
                            log::debug!("  🔍 First fused layer debug:");
                            log::debug!("     Prefix: {}", prefix);
                            log::debug!("     Conv weight key: {}", key);
                            log::debug!("     Conv bias key: {}", conv_bias_key);
                            log::debug!("     Fused weight shape: {:?}", fused_weight.size());
                            log::debug!("     Fused bias shape: {:?}", fused_bias.size());
                            let sample_values = Vec::<f32>::try_from(fused_bias.slice(
                                0,
                                0,
                                5.min(fused_bias.size()[0]),
                                1,
                            ))
                            .unwrap();
                            log::debug!("     Fused bias sample (first 5): {:?}", sample_values);
                            // Store for verification
                            std::env::set_var("FIRST_FUSED_BIAS_KEY", &conv_bias_key);
                            std::env::set_var(
                                "FIRST_FUSED_BIAS_SAMPLE",
                                format!("{:?}", sample_values),
                            );
                        }

                        // Replace conv.weight and conv.bias with fused versions
                        converted_weights.insert(key.clone(), fused_weight);
                        converted_weights.insert(conv_bias_key.clone(), fused_bias);

                        log::debug!("  ✅ Fused: {}", prefix);
                        fused_count += 1;
                    }
                    Err(e) => {
                        log::debug!("  ⚠️  Failed to fuse {}: {}", prefix, e);
                    }
                }
            }
        }
    }

    log::debug!("✅ Fused {} conv+bn pairs\n", fused_count);

    // Manually copy tensors into VarStore variables
    // We can't use vs.load() because we need custom key conversion
    let mut variables = vs.variables_.lock().unwrap();

    let mut matched = 0;
    let mut missing_in_model = Vec::new();
    let mut missing_in_file = Vec::new();

    // Debug: log conv.bias keys in VarStore
    log::debug!("\n=== VarStore Conv Bias Keys (first 5) ===");
    let mut bias_count = 0;
    for (model_key, _) in variables.named_variables.iter() {
        if model_key.contains(".conv.bias") && bias_count < 5 {
            log::debug!("  VarStore key: {}", model_key);
            bias_count += 1;
        }
    }

    log::debug!("\n=== Converted Weights Conv Bias Keys (first 5) ===");
    let mut bias_count = 0;
    for key in converted_weights.keys() {
        if key.contains(".conv.bias") && bias_count < 5 {
            log::debug!("  Converted key: {}", key);
            bias_count += 1;
        }
    }
    log::debug!("");

    for (model_key, model_var) in variables.named_variables.iter_mut() {
        if let Some(file_tensor) = converted_weights.get(model_key) {
            // Check shape compatibility before copying
            if model_var.size() != file_tensor.size() {
                missing_in_file.push(format!("{} (shape mismatch)", model_key));
                continue;
            }

            // Copy data from file tensor to model variable
            tch::no_grad(|| {
                model_var.copy_(file_tensor);
            });

            // Debug: verify first fused bias was actually loaded
            if let Ok(expected_key) = std::env::var("FIRST_FUSED_BIAS_KEY") {
                if model_key == &expected_key {
                    log::debug!("\n🔍 Verifying first fused bias was loaded:");
                    log::debug!("   Key: {}", model_key);
                    log::debug!("   Shape: {:?}", model_var.size());
                    let bias_values: Vec<f32> = model_var
                        .slice(0, 0, 5.min(model_var.size()[0]), 1)
                        .try_into()
                        .unwrap();
                    log::debug!("   Loaded values (first 5): {:?}", bias_values);
                    if let Ok(expected_sample) = std::env::var("FIRST_FUSED_BIAS_SAMPLE") {
                        log::debug!("   Expected (from fusion): {}", expected_sample);
                        log::debug!(
                            "   Match: {}",
                            format!("{:?}", bias_values) == expected_sample
                        );
                    }
                }
            }

            matched += 1;
        } else {
            missing_in_file.push(model_key.clone());
        }
    }

    // Check for extra weights in file not used by model
    for file_key in converted_weights.keys() {
        if !variables.named_variables.contains_key(file_key) {
            missing_in_model.push(file_key.clone());
        }
    }

    // Release lock
    drop(variables);

    // Report stats
    log::debug!("✅ Loaded {} weights into model", matched);

    if !missing_in_file.is_empty() {
        log::debug!(
            "⚠️  {} weights missing in file (first 10):",
            missing_in_file.len()
        );
        for key in missing_in_file.iter().take(10) {
            log::debug!("    {}", key);
        }
    }

    if !missing_in_model.is_empty() {
        log::debug!(
            "⚠️  {} extra weights in file not used by model (first 10):",
            missing_in_model.len()
        );
        for key in missing_in_model.iter().take(10) {
            log::debug!("    {}", key);
        }
    }

    if matched == 0 {
        return Err("No weights matched! Check model structure.".to_string());
    }

    Ok(())
}

/// Verify that all expected weights are present in VarStore
///
/// # Arguments
/// * `vs` - VarStore to verify
/// * `config` - Model configuration (for layer counts)
///
/// # Returns
/// * `Result<()>` - Ok if all weights present, Err with details otherwise
pub fn verify_weights(
    vs: &nn::VarStore,
    num_decoder_layers: i64,
    num_feature_levels: i64,
) -> Result<(), String> {
    let variables = vs.variables();
    let loaded_keys: HashSet<String> = variables.keys().cloned().collect();

    let mut missing_keys: Vec<String> = Vec::new();
    let _unexpected_keys: Vec<String> = Vec::new();

    // Expected key patterns (partial list - full verification would be exhaustive)
    let expected_patterns = vec![
        // Backbone patterns
        "model/backbone",
        // Encoder patterns
        "model/encoder_input_proj",
        "model/encoder",
        "model/enc_output",
        "model/enc_score_head",
        "model/enc_bbox_head",
        // Decoder patterns
        "model/decoder_input_proj",
        "model/decoder",
        // Detection heads (decoder.class_embed.0, decoder.class_embed.1, ...)
        // Note: Python uses "model.decoder.class_embed.0" but Rust path is "class_embed/0"
    ];

    // Check for presence of key patterns
    for pattern in &expected_patterns {
        let found = loaded_keys.iter().any(|k| k.contains(pattern));
        if !found {
            missing_keys.push(pattern.to_string());
        }
    }

    // Check decoder layer heads exist (class_embed and bbox_embed for each layer)
    for i in 0..num_decoder_layers {
        // Python keys: "model.decoder.class_embed.{i}.weight"
        // Rust path: "class_embed/{i}/weight"
        let class_key_pattern = format!("class_embed/{}", i);
        let bbox_key_pattern = format!("bbox_embed/{}", i);

        let class_found = loaded_keys.iter().any(|k| k.contains(&class_key_pattern));
        let bbox_found = loaded_keys.iter().any(|k| k.contains(&bbox_key_pattern));

        if !class_found {
            missing_keys.push(format!("class_embed layer {}", i));
        }
        if !bbox_found {
            missing_keys.push(format!("bbox_embed layer {}", i));
        }
    }

    // Report results
    if !missing_keys.is_empty() {
        return Err(format!(
            "Missing expected weight patterns: {:?}\nLoaded {} keys total",
            missing_keys,
            loaded_keys.len()
        ));
    }

    Ok(())
}

/// Print weight statistics for debugging
///
/// # Arguments
/// * `vs` - VarStore to inspect
pub fn print_weight_stats(vs: &nn::VarStore) {
    let variables = vs.variables();
    let named_vars = &variables;

    log::debug!("=== Weight Statistics ===");
    log::debug!("Total variables: {}", named_vars.len());

    // Group by module
    let mut backbone_count = 0;
    let mut encoder_count = 0;
    let mut decoder_count = 0;
    let mut head_count = 0;
    let mut other_count = 0;

    for key in named_vars.keys() {
        // Check detection heads first (they contain "decoder" so must come before decoder check)
        if key.contains("class_embed") || key.contains("bbox_embed") {
            head_count += 1;
        } else if key.contains("backbone") {
            backbone_count += 1;
        } else if key.contains("encoder") {
            encoder_count += 1;
        } else if key.contains("decoder") {
            decoder_count += 1;
        } else {
            other_count += 1;
        }
    }

    log::debug!("  Backbone: {}", backbone_count);
    log::debug!("  Encoder: {}", encoder_count);
    log::debug!("  Decoder: {}", decoder_count);
    log::debug!("  Detection heads: {}", head_count);
    log::debug!("  Other: {}", other_count);

    // Print first 10 keys for debugging
    log::debug!("\nFirst 10 weight keys:");
    for (i, key) in named_vars.keys().take(10).enumerate() {
        let tensor = &named_vars[key];
        log::debug!("  {:2}: {:60} shape={:?}", i, key, tensor.size());
    }
}

/// Get HuggingFace model path for docling-layout-heron
///
/// # Returns
/// * `PathBuf` - Path to model directory in HuggingFace cache
pub fn get_model_path() -> Result<std::path::PathBuf, String> {
    // Standard HuggingFace cache location
    let cache_dir = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?
        .join(".cache")
        .join("huggingface")
        .join("hub");

    // Model repo: docling-project/docling-layout-heron
    let model_dir = cache_dir
        .join("models--docling-project--docling-layout-heron")
        .join("snapshots");

    // Find the snapshot directory (there should be exactly one)
    let entries = std::fs::read_dir(&model_dir).map_err(|e| {
        format!("Model not found at {:?}. Please run: huggingface-cli download docling-project/docling-layout-heron\nError: {}", model_dir, e)
    })?;

    let mut snapshot_dirs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        if entry
            .file_type()
            .map_err(|e| format!("Failed to get file type: {}", e))?
            .is_dir()
        {
            snapshot_dirs.push(entry.path());
        }
    }

    if snapshot_dirs.is_empty() {
        return Err(format!(
            "No snapshot found in {:?}. Please run: huggingface-cli download docling-project/docling-layout-heron",
            model_dir
        ));
    }

    if snapshot_dirs.len() > 1 {
        log::warn!(
            "Warning: Multiple snapshots found, using: {:?}",
            snapshot_dirs[0]
        );
    }

    Ok(snapshot_dirs[0].join("model.safetensors"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use log;

    #[test]
    fn test_get_model_path() {
        // This test just checks that the function doesn't panic
        // Actual model download is required for real testing
        match get_model_path() {
            Ok(path) => {
                log::debug!("Model path: {:?}", path);
                assert!(path.to_str().unwrap().contains("docling-layout-heron"));
            }
            Err(e) => {
                log::debug!("Model not found (expected if not downloaded): {}", e);
            }
        }
    }
}
