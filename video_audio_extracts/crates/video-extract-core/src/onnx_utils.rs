//! ONNX Runtime utilities for optimized model loading and inference
//!
//! This module provides helper functions for creating optimized ONNX Runtime sessions
//! with graph optimizations, execution providers, and performance tuning.

use ort::execution_providers::{
    CPUExecutionProvider, CUDAExecutionProvider, CoreMLExecutionProvider,
};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use std::path::Path;
use std::time::Instant;

/// Error type for ONNX operations
#[derive(Debug, thiserror::Error)]
pub enum OnnxError {
    #[error("Failed to create session builder: {0}")]
    SessionBuilderError(String),

    #[error("Failed to load ONNX model from {path}: {error}")]
    ModelLoadError { path: String, error: String },

    #[error("Model file not found: {0}")]
    ModelNotFound(String),
}

/// Get the cache directory for compiled CoreML models
///
/// Returns a path to a persistent cache directory where CoreML compiled models
/// will be stored to avoid re-compilation overhead (1.0-1.2s per process).
///
/// Cache location priority:
/// 1. Environment variable `VIDEO_EXTRACT_COREML_CACHE_DIR` (if set)
/// 2. `$HOME/.cache/video-extract/coreml` (XDG standard on Unix)
/// 3. `$TMPDIR/video-extract-coreml` (fallback if HOME not set)
///
/// The cache directory is created if it doesn't exist.
fn get_coreml_cache_dir() -> String {
    // Allow override via environment variable
    if let Ok(dir) = std::env::var("VIDEO_EXTRACT_COREML_CACHE_DIR") {
        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("Warning: Failed to create cache directory {}: {}", dir, e);
        }
        return dir;
    }

    // Use XDG standard: $HOME/.cache/video-extract/coreml
    if let Ok(home) = std::env::var("HOME") {
        let cache_dir = format!("{}/.cache/video-extract/coreml", home);
        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            eprintln!(
                "Warning: Failed to create cache directory {}: {}",
                cache_dir, e
            );
        }
        return cache_dir;
    }

    // Fallback to temp directory
    let temp_dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let cache_dir = format!("{}/video-extract-coreml", temp_dir);
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        eprintln!(
            "Warning: Failed to create cache directory {}: {}",
            cache_dir, e
        );
    }
    cache_dir
}

/// Create an optimized ONNX Runtime session with performance tuning
///
/// This function configures ONNX Runtime with:
/// - Maximum graph optimizations (GraphOptimizationLevel::All)
/// - Optimal intra-op parallelism (physical CPU cores)
/// - Platform-specific execution providers (CoreML on macOS, CUDA on NVIDIA GPUs, CPU fallback)
/// - Memory pattern optimization
///
/// Execution providers are tried in order of performance:
/// 1. **CoreML** (macOS/iOS): Apple Neural Engine + GPU acceleration (3-10x speedup)
/// 2. **CUDA** (NVIDIA GPUs): GPU acceleration (5-20x speedup)
/// 3. **CPU** (fallback): Always available, uses multi-threading
///
/// If CoreML fails to compile the model (unsupported operations), automatically falls back
/// to CUDA or CPU.
///
/// # Arguments
/// * `model_path` - Path to the ONNX model file
///
/// # Returns
/// * `Ok(Session)` - Optimized ONNX Runtime session ready for inference
/// * `Err(OnnxError)` - If model loading or session creation fails
///
/// # Example
/// ```no_run
/// use video_extract_core::onnx_utils::create_optimized_session;
/// use std::path::Path;
///
/// let session = create_optimized_session(Path::new("models/yolo/yolov8n.onnx"))?;
/// // Use session for inference...
/// # Ok::<(), video_extract_core::onnx_utils::OnnxError>(())
/// ```
pub fn create_optimized_session(model_path: &Path) -> Result<Session, OnnxError> {
    // Verify model file exists
    if !model_path.exists() {
        return Err(OnnxError::ModelNotFound(model_path.display().to_string()));
    }

    // Get physical CPU count for optimal parallelism
    // Allow override via environment variable (useful for testing to avoid thread contention)
    let num_threads = std::env::var("VIDEO_EXTRACT_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or_else(num_cpus::get_physical);

    // Get cache directory for compiled CoreML models
    // This eliminates 1.0-1.2s compilation overhead per process
    let cache_dir = get_coreml_cache_dir();

    // Debug: measure session creation time (includes CoreML compilation)
    let debug_enabled = std::env::var("VIDEO_EXTRACT_DEBUG_ONNX")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    let start_time = if debug_enabled {
        Some(Instant::now())
    } else {
        None
    };

    // Try with CoreML first (best performance on macOS)
    let session = Session::builder()
        .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
        .with_intra_threads(num_threads)
        .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
        .with_memory_pattern(true)
        .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
        .with_execution_providers([
            CoreMLExecutionProvider::default()
                .with_subgraphs(true)
                .with_model_cache_dir(cache_dir.clone())
                .build(),
            CUDAExecutionProvider::default().build(),
            CPUExecutionProvider::default().build(),
        ])
        .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
        .commit_from_file(model_path);

    // If CoreML fails, fall back to CUDA/CPU only
    match session {
        Ok(s) => {
            // Log which execution providers are actually being used
            // This helps verify if CoreML is working or falling back to CPU
            if debug_enabled {
                let elapsed = start_time.map(|t| t.elapsed()).unwrap();
                eprintln!(
                    "[ONNX DEBUG] Session created for model: {} in {:.3}s",
                    model_path.display(),
                    elapsed.as_secs_f64()
                );
                eprintln!(
                    "[ONNX DEBUG] Execution providers attempted: CoreML (with cache), CUDA, CPU"
                );
                eprintln!("[ONNX DEBUG] Cache directory: {}", cache_dir);
                eprintln!(
                    "[ONNX DEBUG] Note: ONNX Runtime selects execution provider per-node, not per-session"
                );
            }
            Ok(s)
        }
        Err(e) => {
            let error_msg = e.to_string();
            // Check if this is a CoreML compilation error
            if error_msg.contains("CoreML") || error_msg.contains("MLModel") {
                if debug_enabled {
                    eprintln!(
                        "[ONNX DEBUG] CoreML failed for {}: {}",
                        model_path.display(),
                        error_msg
                    );
                    eprintln!("[ONNX DEBUG] Retrying with CUDA/CPU only...");
                }

                // Retry without CoreML (just CUDA/CPU)
                let fallback_start = if debug_enabled {
                    Some(Instant::now())
                } else {
                    None
                };

                let fallback_session = Session::builder()
                    .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
                    .with_optimization_level(GraphOptimizationLevel::Level3)
                    .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
                    .with_intra_threads(num_threads)
                    .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
                    .with_memory_pattern(true)
                    .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
                    .with_execution_providers([
                        CUDAExecutionProvider::default().build(),
                        CPUExecutionProvider::default().build(),
                    ])
                    .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
                    .commit_from_file(model_path)
                    .map_err(|e| OnnxError::ModelLoadError {
                        path: model_path.display().to_string(),
                        error: format!("CoreML failed, CPU/CUDA also failed: {}", e),
                    })?;

                if debug_enabled {
                    if let Some(start) = fallback_start {
                        eprintln!(
                            "[ONNX DEBUG] Fallback session created in {:.3}s",
                            start.elapsed().as_secs_f64()
                        );
                    }
                }

                Ok(fallback_session)
            } else {
                // Not a CoreML error, return original error
                Err(OnnxError::ModelLoadError {
                    path: model_path.display().to_string(),
                    error: error_msg,
                })
            }
        }
    }
}

/// Create an optimized ONNX Runtime session with CPU-only execution
///
/// This function creates a session without CoreML or CUDA, using only CPU execution.
/// Useful for models that are incompatible with hardware acceleration (e.g., CLAP audio model).
///
/// # Arguments
/// * `model_path` - Path to the ONNX model file
///
/// # Returns
/// * `Ok(Session)` - Optimized ONNX Runtime session ready for inference (CPU-only)
/// * `Err(OnnxError)` - If model loading or session creation fails
pub fn create_cpu_only_session(model_path: &Path) -> Result<Session, OnnxError> {
    // Verify model file exists
    if !model_path.exists() {
        return Err(OnnxError::ModelNotFound(model_path.display().to_string()));
    }

    // Get physical CPU count for optimal parallelism
    // Allow override via environment variable (useful for testing to avoid thread contention)
    let num_threads = std::env::var("VIDEO_EXTRACT_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or_else(num_cpus::get_physical);

    // Create session with CPU-only execution provider
    Session::builder()
        .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
        .with_intra_threads(num_threads)
        .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
        .with_memory_pattern(true)
        .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
        .with_execution_providers([CPUExecutionProvider::default().build()])
        .map_err(|e| OnnxError::SessionBuilderError(e.to_string()))?
        .commit_from_file(model_path)
        .map_err(|e| OnnxError::ModelLoadError {
            path: model_path.display().to_string(),
            error: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_not_found() {
        let result = create_optimized_session(Path::new("nonexistent_model.onnx"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OnnxError::ModelNotFound(_)));
    }

    #[test]
    fn test_error_display() {
        let err = OnnxError::ModelNotFound("test.onnx".to_string());
        assert_eq!(err.to_string(), "Model file not found: test.onnx");

        let err = OnnxError::ModelLoadError {
            path: "test.onnx".to_string(),
            error: "invalid format".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Failed to load ONNX model from test.onnx: invalid format"
        );
    }
}
