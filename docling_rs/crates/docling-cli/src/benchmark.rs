// Benchmark code involves many numeric operations for statistics.
// These are safe because:
// - Durations/counts are well within representable ranges
// - Statistical calculations use f64 which handles all cases
#![allow(
    clippy::cast_possible_truncation,  // duration/count conversions safe
    clippy::cast_precision_loss,       // f64 sufficient for statistics
    clippy::cast_sign_loss,            // counts always non-negative
    clippy::must_use_candidate,        // benchmark results often unused
)]

//! Benchmark infrastructure for CLI
//!
//! Provides timing and performance measurement for document conversions.
//! Integrated via the `docling benchmark` CLI command.
//!
//! # Usage
//! ```bash
//! # Basic benchmark
//! docling benchmark file.pdf
//!
//! # Multiple iterations with warmup
//! docling benchmark file.pdf -n 5 -w 2
//!
//! # JSON output
//! docling benchmark file.pdf -f json -o results.json
//! ```

use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Configuration for benchmark runs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Number of timed iterations per file
    pub iterations: usize,
    /// Number of warmup iterations (results discarded)
    pub warmup_iterations: usize,
}

impl Default for BenchmarkConfig {
    #[inline]
    fn default() -> Self {
        Self {
            iterations: 3,
            warmup_iterations: 1,
        }
    }
}

/// Result of benchmarking a single file
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Path to the benchmarked file
    pub file: PathBuf,
    /// Individual timing measurements
    pub times: Vec<Duration>,
    /// Mean duration
    pub mean: Duration,
    /// Minimum duration
    pub min: Duration,
    /// Maximum duration
    pub max: Duration,
    /// Standard deviation in microseconds
    pub std_dev_us: u64,
    /// File size in bytes
    pub file_size: u64,
    /// Whether the conversion succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl BenchmarkResult {
    /// Calculate statistics from timing measurements
    #[must_use = "benchmark result is created but not used"]
    pub fn from_times(file: PathBuf, times: Vec<Duration>, file_size: u64) -> Self {
        if times.is_empty() {
            return Self {
                file,
                times: vec![],
                mean: Duration::ZERO,
                min: Duration::ZERO,
                max: Duration::ZERO,
                std_dev_us: 0,
                file_size,
                success: false,
                error: Some("No timing data".to_string()),
            };
        }

        let sum: Duration = times.iter().sum();
        let mean = sum / times.len() as u32;
        let min = *times.iter().min().unwrap();
        let max = *times.iter().max().unwrap();

        // Calculate standard deviation
        let mean_us = mean.as_micros() as f64;
        let variance: f64 = times
            .iter()
            .map(|t| {
                let diff = t.as_micros() as f64 - mean_us;
                diff * diff
            })
            .sum::<f64>()
            / times.len() as f64;
        let std_dev_us = variance.sqrt() as u64;

        Self {
            file,
            times,
            mean,
            min,
            max,
            std_dev_us,
            file_size,
            success: true,
            error: None,
        }
    }

    /// Create a failed result
    #[inline]
    #[must_use = "failed benchmark result is created but not used"]
    pub const fn failed(file: PathBuf, error: String, file_size: u64) -> Self {
        Self {
            file,
            times: vec![],
            mean: Duration::ZERO,
            min: Duration::ZERO,
            max: Duration::ZERO,
            std_dev_us: 0,
            file_size,
            success: false,
            error: Some(error),
        }
    }
}

/// Benchmark runner for document conversions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner with the given configuration
    #[inline]
    #[must_use = "benchmark runner is created but not used"]
    pub const fn new(config: BenchmarkConfig) -> Self {
        Self { config }
    }

    /// Run benchmarks on the given input files
    #[must_use = "benchmark results are returned but not used"]
    pub fn run_benchmarks(&self, inputs: &[PathBuf]) -> Vec<BenchmarkResult> {
        let mut results = Vec::with_capacity(inputs.len());

        for input in inputs {
            let result = self.benchmark_file(input);
            results.push(result);
        }

        results
    }

    /// Benchmark a single file
    fn benchmark_file(&self, path: &Path) -> BenchmarkResult {
        use docling_backend::DocumentConverter;

        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        // Create converter
        let converter = match DocumentConverter::new() {
            Ok(c) => c,
            Err(e) => {
                return BenchmarkResult::failed(
                    path.to_path_buf(),
                    format!("Failed to create converter: {e}"),
                    file_size,
                );
            }
        };

        // Warmup iterations
        for _ in 0..self.config.warmup_iterations {
            let _ = converter.convert(path);
        }

        // Timed iterations
        let mut times = Vec::with_capacity(self.config.iterations);
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let result = converter.convert(path);
            let elapsed = start.elapsed();

            if result.is_err() {
                return BenchmarkResult::failed(
                    path.to_path_buf(),
                    format!("Conversion failed: {}", result.unwrap_err()),
                    file_size,
                );
            }

            times.push(elapsed);
        }

        BenchmarkResult::from_times(path.to_path_buf(), times, file_size)
    }

    /// Format results as a human-readable text table
    #[must_use = "formatted text is returned but not used"]
    pub fn format_as_text(results: &[BenchmarkResult]) -> String {
        let mut output = String::new();
        output.push_str("Benchmark Results\n");
        output.push_str("=================\n\n");

        for result in results {
            let filename = result.file.file_name().map_or_else(
                || result.file.display().to_string(),
                |s| s.to_string_lossy().to_string(),
            );

            if result.success {
                let _ = write!(
                    output,
                    "{}\n  Mean: {:?}\n  Min:  {:?}\n  Max:  {:?}\n  Std:  {}µs\n  Size: {} bytes\n  Runs: {}\n\n",
                    filename,
                    result.mean,
                    result.min,
                    result.max,
                    result.std_dev_us,
                    result.file_size,
                    result.times.len(),
                );
            } else {
                let _ = write!(
                    output,
                    "{}\n  FAILED: {}\n\n",
                    filename,
                    result.error.as_deref().unwrap_or("Unknown error"),
                );
            }
        }

        output
    }

    /// Format results as JSON
    #[must_use = "formatted JSON is returned but not used"]
    pub fn format_as_json(results: &[BenchmarkResult]) -> String {
        let mut output = String::from("[\n");
        for (i, r) in results.iter().enumerate() {
            let error_str = r.error.as_ref().map_or_else(
                || "null".to_string(),
                |e| format!("\"{}\"", e.replace('"', "\\\"")),
            );
            let _ = write!(
                output,
                "  {{\n    \"file\": \"{}\",\n    \"success\": {},\n    \"mean_us\": {},\n    \"min_us\": {},\n    \"max_us\": {},\n    \"std_dev_us\": {},\n    \"file_size\": {},\n    \"iterations\": {},\n    \"error\": {}\n  }}",
                r.file.display().to_string().replace('\\', "\\\\").replace('"', "\\\""),
                r.success,
                r.mean.as_micros(),
                r.min.as_micros(),
                r.max.as_micros(),
                r.std_dev_us,
                r.file_size,
                r.times.len(),
                error_str,
            );
            if i < results.len() - 1 {
                output.push_str(",\n");
            } else {
                output.push('\n');
            }
        }
        output.push(']');
        output
    }

    /// Format results as CSV
    #[must_use = "formatted CSV is returned but not used"]
    pub fn format_as_csv(results: &[BenchmarkResult]) -> String {
        let mut output = String::from(
            "file,success,mean_us,min_us,max_us,std_dev_us,file_size,iterations,error\n",
        );

        for r in results {
            let _ = writeln!(
                output,
                "{},{},{},{},{},{},{},{},{}",
                r.file.display(),
                r.success,
                r.mean.as_micros(),
                r.min.as_micros(),
                r.max.as_micros(),
                r.std_dev_us,
                r.file_size,
                r.times.len(),
                r.error.as_deref().unwrap_or(""),
            );
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_config_default() {
        let config = BenchmarkConfig::default();
        assert_eq!(config.iterations, 3);
        assert_eq!(config.warmup_iterations, 1);
    }

    #[test]
    fn test_benchmark_result_from_times() {
        let times = vec![
            Duration::from_millis(100),
            Duration::from_millis(110),
            Duration::from_millis(90),
        ];
        let result = BenchmarkResult::from_times(PathBuf::from("test.pdf"), times, 1024);

        assert!(result.success);
        assert_eq!(result.times.len(), 3);
        assert_eq!(result.min, Duration::from_millis(90));
        assert_eq!(result.max, Duration::from_millis(110));
        assert_eq!(result.file_size, 1024);
    }

    #[test]
    fn test_benchmark_result_failed() {
        let result =
            BenchmarkResult::failed(PathBuf::from("test.pdf"), "Test error".to_string(), 512);

        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "Test error");
    }

    #[test]
    fn test_benchmark_runner_creation() {
        let config = BenchmarkConfig::default();
        let runner = BenchmarkRunner::new(config);
        assert_eq!(runner.config.iterations, 3);
    }

    #[test]
    fn test_format_as_text() {
        let results = vec![BenchmarkResult {
            file: PathBuf::from("test.pdf"),
            times: vec![Duration::from_millis(100)],
            mean: Duration::from_millis(100),
            min: Duration::from_millis(100),
            max: Duration::from_millis(100),
            std_dev_us: 0,
            file_size: 1024,
            success: true,
            error: None,
        }];

        let text = BenchmarkRunner::format_as_text(&results);
        assert!(text.contains("test.pdf"));
        assert!(text.contains("Mean:"));
    }

    #[test]
    fn test_format_as_csv() {
        let results = vec![BenchmarkResult {
            file: PathBuf::from("test.pdf"),
            times: vec![Duration::from_millis(100)],
            mean: Duration::from_millis(100),
            min: Duration::from_millis(100),
            max: Duration::from_millis(100),
            std_dev_us: 0,
            file_size: 1024,
            success: true,
            error: None,
        }];

        let csv = BenchmarkRunner::format_as_csv(&results);
        assert!(csv.contains("file,success,mean_us"));
        // 100ms = 100,000us
        assert!(csv.contains("test.pdf,true,100000"));
    }
}
