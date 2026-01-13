// CLI tool has many numeric conversions for progress display and statistics.
// These are safe because:
// - File sizes, durations, and counts are well within representable ranges
// - Progress percentages use f64 which handles all cases
// - Byte counts are formatted for display, truncation is acceptable
#![allow(
    clippy::cast_possible_truncation,  // file sizes, progress bars - safe ranges
    clippy::cast_sign_loss,            // lengths/counts are always non-negative
    clippy::cast_precision_loss,       // f64 sufficient for display purposes
    clippy::cast_possible_wrap,        // lengths fit in signed types for iteration
    clippy::similar_names,             // args/opts patterns are CLI conventions
    clippy::too_many_lines,            // CLI main() is necessarily large
    clippy::needless_pass_by_value,    // clap requires owned strings
    clippy::trivially_copy_pass_by_ref,// consistent API for small types
    clippy::unreadable_literal,        // numeric constants (timeouts, sizes)
    clippy::unnecessary_wraps,         // consistent Result return for CLI handlers
    clippy::unnecessary_debug_formatting, // useful for error messages
    clippy::fn_params_excessive_bools, // CLI commands have many boolean flags
    clippy::must_use_candidate,        // CLI functions don't need must_use
)]

//! Docling CLI - Document conversion and benchmarking tool
//!
//! A command-line interface for converting documents and benchmarking performance.

mod benchmark;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use colored::Colorize;
use docling_backend::DocumentConverter;
use docling_core::{format::InputFormat, Document, JsonSerializer, YamlSerializer};

use indicatif::{ProgressBar, ProgressStyle};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

/// Format bytes as human-readable size (e.g., "1.5 MB")
fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} bytes")
    }
}

/// Generate smart output path from input file and output format.
///
/// Given an input file like "report.pdf" and format Markdown, returns "report.md".
/// The output file is created in the same directory as the input file.
fn smart_output_path(input: &Path, format: &OutputFormat) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default();
    let ext = match format {
        OutputFormat::Markdown => "md",
        OutputFormat::Json => "json",
        OutputFormat::Yaml => "yaml",
    };
    input.with_file_name(format!("{}.{}", stem.to_string_lossy(), ext))
}

/// Parse a human-readable file size string into bytes.
///
/// Supports formats:
/// - Plain numbers: "1048576" -> 1048576 bytes
/// - KB suffix: "100K", "100KB", "100k" -> 102400 bytes
/// - MB suffix: "10M", "10MB", "10m" -> 10485760 bytes
/// - GB suffix: "1G", "1GB", "1g" -> 1073741824 bytes
///
/// Decimal values are supported: "1.5M" -> 1572864 bytes
fn parse_file_size(s: &str) -> Result<usize, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty file size".to_string());
    }

    // Check for suffix (case-insensitive)
    let s_upper = s.to_uppercase();
    let (num_str, multiplier) = if s_upper.ends_with("GB") {
        (&s[..s.len() - 2], 1024 * 1024 * 1024)
    } else if s_upper.ends_with("MB") {
        (&s[..s.len() - 2], 1024 * 1024)
    } else if s_upper.ends_with("KB") {
        (&s[..s.len() - 2], 1024)
    } else if s_upper.ends_with('G') {
        (&s[..s.len() - 1], 1024 * 1024 * 1024)
    } else if s_upper.ends_with('M') {
        (&s[..s.len() - 1], 1024 * 1024)
    } else if s_upper.ends_with('K') {
        (&s[..s.len() - 1], 1024)
    } else if s_upper.ends_with('B') {
        // Just "B" suffix means bytes
        (&s[..s.len() - 1], 1)
    } else {
        // No suffix - assume bytes
        (s, 1)
    };

    let num_str = num_str.trim();
    if num_str.is_empty() {
        return Err("missing numeric value".to_string());
    }

    // Parse as float to support decimals like "1.5M"
    let value: f64 = num_str
        .parse()
        .map_err(|_| format!("invalid number: '{num_str}'"))?;

    if value < 0.0 {
        return Err("file size cannot be negative".to_string());
    }

    let bytes = (value * f64::from(multiplier)).round() as usize;
    Ok(bytes)
}

/// Verbosity level for output control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Verbosity {
    /// Suppress all output except errors
    Quiet,
    /// Normal output (default)
    Normal,
    /// Verbose output with extra details
    Verbose,
}

impl Verbosity {
    /// Create from CLI flags
    const fn from_flags(quiet: bool, verbose: bool) -> Self {
        if quiet {
            Self::Quiet
        } else if verbose {
            Self::Verbose
        } else {
            Self::Normal
        }
    }

    /// Check if output should be shown (not quiet)
    const fn should_show_output(self) -> bool {
        !matches!(self, Self::Quiet)
    }

    /// Check if verbose output is requested
    const fn is_verbose(self) -> bool {
        matches!(self, Self::Verbose)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
enum OutputFormat {
    /// Markdown output (default)
    Markdown,
    /// JSON output
    Json,
    /// YAML output
    Yaml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
enum Backend {
    /// Use Rust backend (supported formats only)
    Rust,
    /// Auto-select backend (default: Rust for all supported formats)
    Auto,
}

/// ML inference device selection (PDF only, requires pdf-ml feature)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Default)]
enum MlDevice {
    /// Auto-detect best device (MPS on Mac, CUDA on Linux with GPU, CPU fallback)
    #[default]
    Auto,
    /// Force CPU inference
    Cpu,
    /// Use CUDA GPU (Linux/Windows with NVIDIA GPU)
    Cuda,
    /// Use Metal Performance Shaders (macOS Apple Silicon)
    Mps,
}

/// ML model size/accuracy tradeoff (PDF only, requires pdf-ml feature)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Default)]
enum ModelSize {
    /// Fast inference with ResNet-18 backbone (~2x faster, slightly less accurate)
    Fast,
    /// Standard inference with ResNet-50 backbone (default)
    #[default]
    Standard,
    /// Accurate inference with ResNet-101 backbone (slower, most accurate)
    Accurate,
}

/// ML backend selection for PDF processing
///
/// `PyTorch` requires the `pdf-ml` feature to be enabled at compile time.
/// If `PyTorch` is requested but unavailable, falls back to ONNX with a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Default)]
enum MlBackend {
    /// `PyTorch` backend (GPU support: CUDA, MPS/Metal; requires pdf-ml feature)
    #[default]
    Pytorch,
    /// ONNX Runtime backend (CPU-optimized, portable, always available)
    Onnx,
}

/// Benchmark output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
enum BenchFormat {
    /// Human-readable text
    Text,
    /// JSON format
    Json,
    /// CSV format
    Csv,
}

/// Configuration file structure for .docling.toml
///
/// Configuration files can be placed in:
/// - User home directory: ~/.docling.toml (user defaults)
/// - Project directory: ./.docling.toml (project defaults)
/// - Custom location via --config flag (overrides both)
///
/// Precedence order (highest to lowest):
/// 1. Command-line arguments (--format, --backend, etc.)
/// 2. Project config (./.docling.toml)
/// 3. User config (~/.docling.toml)
/// 4. Built-in defaults
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
struct Config {
    /// Default settings for convert command
    #[serde(skip_serializing_if = "Option::is_none")]
    convert: Option<ConvertConfig>,

    /// Default settings for batch command
    #[serde(skip_serializing_if = "Option::is_none")]
    batch: Option<BatchConfig>,

    /// Default settings for benchmark command
    #[serde(skip_serializing_if = "Option::is_none")]
    benchmark: Option<BenchmarkConfigSettings>,

    /// Named profiles for convert command (e.g., "fast", "accurate")
    #[serde(skip_serializing_if = "Option::is_none")]
    profiles: Option<std::collections::HashMap<String, ConvertConfig>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(default)]
struct ConvertConfig {
    /// Default output format (markdown, json, yaml)
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,

    /// Default backend (rust or auto)
    #[serde(skip_serializing_if = "Option::is_none")]
    backend: Option<String>,

    /// Default compact JSON output
    #[serde(skip_serializing_if = "Option::is_none")]
    compact: Option<bool>,

    /// Default OCR enablement
    #[serde(skip_serializing_if = "Option::is_none")]
    ocr: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(default)]
struct BatchConfig {
    /// Default output format
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,

    /// Continue on errors
    #[serde(skip_serializing_if = "Option::is_none")]
    continue_on_error: Option<bool>,

    /// Maximum file size in bytes (files larger than this will be skipped)
    #[serde(skip_serializing_if = "Option::is_none")]
    max_file_size: Option<usize>,

    /// Default OCR enablement
    #[serde(skip_serializing_if = "Option::is_none")]
    ocr: Option<bool>,

    /// Default compact JSON output
    #[serde(skip_serializing_if = "Option::is_none")]
    compact: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(default)]
struct BenchmarkConfigSettings {
    /// Default number of iterations
    #[serde(skip_serializing_if = "Option::is_none")]
    iterations: Option<usize>,

    /// Default warmup iterations
    #[serde(skip_serializing_if = "Option::is_none")]
    warmup: Option<usize>,

    /// Default output format
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,

    /// Default OCR enablement
    #[serde(skip_serializing_if = "Option::is_none")]
    ocr: Option<bool>,
}

impl Config {
    /// Load configuration from file
    fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Self = toml::from_str(&content).map_err(|e| {
            // TOML errors include line/column information, preserve it
            eprintln!(
                "{} Failed to parse config file: {}",
                "Error:".red().bold(),
                path.display()
            );
            eprintln!("{} {}", "Parse error:".yellow().bold(), e);
            eprintln!();
            eprintln!("{} Configuration file syntax:", "Help:".cyan().bold());
            eprintln!("  [convert]");
            eprintln!("  format = \"json\"  # markdown, json, or yaml");
            eprintln!("  backend = \"auto\" # rust or auto");
            eprintln!("  ocr = false");
            eprintln!();
            eprintln!("  See examples/.docling.toml for a complete example");
            anyhow::anyhow!("Failed to parse config file: {e}")
        })?;

        Ok(config)
    }

    /// Find and load configuration files
    /// Returns (`user_config`, `project_config`)
    fn discover_configs() -> (Option<Self>, Option<Self>) {
        let user_config = Self::load_user_config();
        let project_config = Self::load_project_config();
        (user_config, project_config)
    }

    /// Load user config from ~/.docling.toml
    fn load_user_config() -> Option<Self> {
        let home_dir = dirs::home_dir()?;
        let config_path = home_dir.join(".docling.toml");

        if config_path.exists() {
            match Self::load_from_file(&config_path) {
                Ok(config) => Some(config),
                Err(e) => {
                    eprintln!(
                        "{} Failed to load user config from {}: {}",
                        "Warning:".yellow().bold(),
                        config_path.display(),
                        e
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    /// Load project config from ./.docling.toml
    fn load_project_config() -> Option<Self> {
        let config_path = PathBuf::from(".docling.toml");

        if config_path.exists() {
            match Self::load_from_file(&config_path) {
                Ok(config) => Some(config),
                Err(e) => {
                    eprintln!(
                        "{} Failed to load project config from {}: {}",
                        "Warning:".yellow().bold(),
                        config_path.display(),
                        e
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    /// Merge multiple configs with precedence
    /// CLI args > project config > user config > defaults
    fn merge(user_config: Option<Self>, project_config: Option<Self>) -> Self {
        let mut merged = Self::default();

        // Apply user config first (lowest precedence)
        if let Some(user) = user_config {
            if let Some(convert) = user.convert {
                merged.convert = Some(convert);
            }
            if let Some(batch) = user.batch {
                merged.batch = Some(batch);
            }
            if let Some(benchmark) = user.benchmark {
                merged.benchmark = Some(benchmark);
            }
        }

        // Apply project config (overrides user config)
        if let Some(project) = project_config {
            if let Some(convert) = project.convert {
                // Merge convert configs
                let mut merged_convert = merged.convert.unwrap_or_default();
                if let Some(format) = convert.format {
                    merged_convert.format = Some(format);
                }
                if let Some(backend) = convert.backend {
                    merged_convert.backend = Some(backend);
                }
                if let Some(compact) = convert.compact {
                    merged_convert.compact = Some(compact);
                }
                if let Some(ocr) = convert.ocr {
                    merged_convert.ocr = Some(ocr);
                }
                merged.convert = Some(merged_convert);
            }

            if let Some(batch) = project.batch {
                // Merge batch configs
                let mut merged_batch = merged.batch.unwrap_or_default();
                if let Some(format) = batch.format {
                    merged_batch.format = Some(format);
                }
                if let Some(continue_on_error) = batch.continue_on_error {
                    merged_batch.continue_on_error = Some(continue_on_error);
                }
                if let Some(max_file_size) = batch.max_file_size {
                    merged_batch.max_file_size = Some(max_file_size);
                }
                if let Some(ocr) = batch.ocr {
                    merged_batch.ocr = Some(ocr);
                }
                if let Some(compact) = batch.compact {
                    merged_batch.compact = Some(compact);
                }
                merged.batch = Some(merged_batch);
            }

            if let Some(benchmark) = project.benchmark {
                // Merge benchmark configs
                let mut merged_benchmark = merged.benchmark.unwrap_or_default();
                if let Some(iterations) = benchmark.iterations {
                    merged_benchmark.iterations = Some(iterations);
                }
                if let Some(warmup) = benchmark.warmup {
                    merged_benchmark.warmup = Some(warmup);
                }
                if let Some(format) = benchmark.format {
                    merged_benchmark.format = Some(format);
                }
                if let Some(ocr) = benchmark.ocr {
                    merged_benchmark.ocr = Some(ocr);
                }
                merged.benchmark = Some(merged_benchmark);
            }
        }

        merged
    }

    /// Resolve output format from CLI, config, or default
    fn resolve_output_format(
        cli_value: Option<OutputFormat>,
        config_value: Option<&str>,
    ) -> OutputFormat {
        if let Some(format) = cli_value {
            return format;
        }

        if let Some(format_str) = config_value {
            return match format_str.to_lowercase().as_str() {
                "json" => OutputFormat::Json,
                "yaml" => OutputFormat::Yaml,
                _ => OutputFormat::Markdown,
            };
        }

        OutputFormat::Markdown
    }

    /// Resolve backend from CLI, config, or default
    fn resolve_backend(cli_value: Option<Backend>, config_value: Option<&str>) -> Backend {
        if let Some(backend) = cli_value {
            return backend;
        }

        if let Some(backend_str) = config_value {
            return match backend_str.to_lowercase().as_str() {
                "rust" => Backend::Rust,
                _ => Backend::Auto,
            };
        }

        Backend::Auto
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "docling",
    about = "Convert documents and benchmark performance",
    long_about = "Convert documents to various formats and measure conversion performance.\n\
                  \n\
                  Supports 55+ formats including PDF, DOCX, XLSX, PPTX, HTML, and more.",
    version
)]
struct Args {
    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Show detailed processing information
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Convert a document to markdown, JSON, or YAML
    #[command(long_about = "Convert documents to markdown, JSON, or YAML.\n\
                      \n\
                      Supports 55+ formats including PDF, DOCX, XLSX, PPTX, HTML, and more.\n\
                      \n\
                      Pure Rust backend supports: PDF, archives (ZIP/TAR/7Z/RAR), subtitles (SRT/WEBVTT),\n\
                      email (EML/MBOX/MSG/VCF), e-books (EPUB/FB2/MOBI), OpenDocument (ODT/ODS/ODP),\n\
                      and many image/CAD/medical formats.\n\
                      \n\
                      Defaults can be set via .docling.toml configuration file.")]
    Convert {
        /// Input file path, or '-' to read from stdin
        #[arg(value_name = "INPUT")]
        input: String,

        /// Output file path (default: auto-generates from input, e.g., report.pdf → report.md)
        /// When reading from stdin, output defaults to stdout unless -o is specified
        #[arg(short, long, value_name = "OUTPUT")]
        output: Option<PathBuf>,

        /// Output format (default: markdown, or from config)
        #[arg(short, long, value_enum)]
        format: Option<OutputFormat>,

        /// Input format (required when reading from stdin, auto-detected otherwise)
        #[arg(long, value_name = "FORMAT")]
        input_format: Option<String>,

        /// Backend to use for conversion (default: auto, or from config)
        #[arg(short, long, value_enum)]
        backend: Option<Backend>,

        /// Compact JSON output (no pretty-printing, only affects JSON format)
        #[arg(long)]
        compact: bool,

        /// Enable OCR for scanned documents (PDF only, requires pdf-ml feature)
        #[arg(long)]
        ocr: bool,

        // --- ML Performance Options (PDF only, requires pdf-ml feature) ---
        /// ML inference device (default: auto-detect MPS/CUDA/CPU)
        #[arg(long, value_enum, default_value = "auto")]
        device: MlDevice,

        /// Batch size for ML inference (default: auto based on device)
        #[arg(long, value_name = "N")]
        batch_size: Option<usize>,

        /// Model size/accuracy tradeoff (default: standard)
        #[arg(long, value_enum, default_value = "standard")]
        model_size: ModelSize,

        /// Skip table structure recognition (faster, tables as text)
        #[arg(long)]
        no_tables: bool,

        /// ML backend for PDF processing (pytorch requires pdf-ml feature, falls back to onnx if unavailable)
        #[arg(long, value_enum, default_value = "pytorch")]
        ml_backend: MlBackend,

        /// Maximum number of pages to process (PDF only, processes all pages if not specified)
        #[arg(long, value_name = "N")]
        max_pages: Option<usize>,

        /// Overwrite existing output files without prompting
        #[arg(long)]
        force: bool,

        /// Never overwrite existing files (exit with error if output exists)
        #[arg(long, conflicts_with = "force")]
        no_clobber: bool,

        /// Show what would be converted without actually converting
        #[arg(long)]
        dry_run: bool,

        /// Watch input file for changes and re-convert automatically
        #[arg(long)]
        watch: bool,

        /// Use a named profile from config (e.g., "fast", "accurate")
        #[arg(long, value_name = "NAME")]
        profile: Option<String>,
    },

    /// Convert multiple documents in batch (streaming mode)
    #[command(
        long_about = "Convert multiple documents efficiently using streaming API.\n\
                      \n\
                      Processes documents one at a time with optional error recovery.\n\
                      \n\
                      Examples:\n\
                        docling batch *.pdf -o output/\n\
                        docling batch docs/*.pdf docs/*.docx --continue-on-error\n\
                      \n\
                      Defaults can be set via .docling.toml configuration file."
    )]
    Batch {
        /// Input file paths or glob patterns
        #[arg(value_name = "INPUTS", required_unless_present = "stdin")]
        inputs: Vec<PathBuf>,

        /// Output directory for converted files (required for batch mode)
        #[arg(short, long, value_name = "OUTPUT_DIR", required = true)]
        output: PathBuf,

        /// Output format (default: markdown, or from config)
        #[arg(short, long, value_enum)]
        format: Option<OutputFormat>,

        /// Continue processing on errors (default: stop on first error, or from config)
        #[arg(long)]
        continue_on_error: bool,

        /// Maximum file size (skip files larger than this)
        ///
        /// Accepts human-readable sizes: 10M, 1G, 500K, 1.5MB
        /// Or raw bytes: 10485760
        #[arg(long, value_name = "SIZE", value_parser = parse_file_size)]
        max_file_size: Option<usize>,

        /// Enable OCR for scanned documents (PDF only, requires pdf-ml feature)
        #[arg(long)]
        ocr: bool,

        /// Compact JSON output (no pretty-printing, only affects JSON format)
        #[arg(long)]
        compact: bool,

        /// Read file list from stdin (one file per line)
        ///
        /// Useful with find: find . -name "*.pdf" | docling batch --stdin -o out/
        #[arg(long)]
        stdin: bool,

        /// Number of parallel workers for processing files
        ///
        /// Default: number of CPU cores. Use 1 for sequential processing.
        /// Higher values can speed up batch processing but use more memory.
        #[arg(short = 'j', long, value_name = "N")]
        parallel: Option<usize>,
    },

    /// Benchmark document conversion performance
    #[command(long_about = "Measure and analyze document conversion performance.\n\
                      \n\
                      Runs multiple iterations and reports timing statistics.\n\
                      \n\
                      Defaults can be set via .docling.toml configuration file.")]
    Benchmark {
        /// Input file path(s)
        #[arg(value_name = "INPUT", required = true)]
        inputs: Vec<PathBuf>,

        /// Number of iterations to run for each file (default: 3, or from config)
        #[arg(short = 'n', long)]
        iterations: Option<usize>,

        /// Warmup iterations (results discarded, default: 1, or from config)
        #[arg(short = 'w', long)]
        warmup: Option<usize>,

        /// Output format (default: text, or from config)
        #[arg(short, long, value_enum)]
        format: Option<BenchFormat>,

        /// Output file path (default: stdout)
        #[arg(short, long, value_name = "OUTPUT")]
        output: Option<PathBuf>,

        /// Enable OCR for scanned documents (PDF only)
        #[arg(long)]
        ocr: bool,
    },
    /// Generate shell completion scripts
    #[command(long_about = "Generate shell completion scripts for docling.\n\
                      \n\
                      Supports bash, zsh, fish, and PowerShell.\n\
                      \n\
                      Examples:\n\
                        docling completion bash > /usr/local/etc/bash_completion.d/docling\n\
                        docling completion zsh > ~/.zsh/completions/_docling\n\
                        docling completion fish > ~/.config/fish/completions/docling.fish\n\
                        docling completion powershell > docling.ps1")]
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// List all supported input formats
    #[command(long_about = "List all input formats supported by docling.\n\
                      \n\
                      Displays format name, file extensions, and backend status.\n\
                      \n\
                      Examples:\n\
                        docling formats              # List all formats\n\
                        docling formats --json       # Output as JSON\n\
                        docling formats pdf          # Show info about PDF format")]
    Formats {
        /// Filter by format name (partial match)
        #[arg(value_name = "FILTER")]
        filter: Option<String>,

        /// Output as JSON instead of table
        #[arg(long)]
        json: bool,
    },

    /// Inspect document metadata and structure without converting
    #[command(
        long_about = "Inspect document metadata and structure without full conversion.\n\
                      \n\
                      Quickly extract file metadata, page count, and basic structure.\n\
                      Useful for checking a document before deciding how to process it.\n\
                      \n\
                      Examples:\n\
                        docling info report.pdf      # Show basic info\n\
                        docling info report.pdf --deep  # Deep analysis (slower)\n\
                        docling info report.pdf --json  # Output as JSON"
    )]
    Info {
        /// Input file to inspect
        #[arg(value_name = "INPUT")]
        input: PathBuf,

        /// Output as JSON instead of text
        #[arg(long)]
        json: bool,

        /// Perform deep analysis (slower, extracts more details)
        #[arg(long)]
        deep: bool,
    },

    /// Manage configuration settings
    #[command(long_about = "Manage docling configuration files and settings.\n\
                      \n\
                      Configuration files are loaded in this order (later overrides earlier):\n\
                        1. User config: ~/.docling.toml\n\
                        2. Project config: ./.docling.toml\n\
                        3. Command-line arguments\n\
                      \n\
                      Examples:\n\
                        docling config init          # Create .docling.toml with defaults\n\
                        docling config show          # Display current configuration\n\
                        docling config get convert.format  # Get a specific value\n\
                        docling config set convert.format json  # Set a value")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

/// Config subcommands
#[derive(Subcommand, Debug)]
enum ConfigAction {
    /// Create a new .docling.toml configuration file with sensible defaults
    Init {
        /// Create in user home directory (~/.docling.toml) instead of current directory
        #[arg(long)]
        global: bool,

        /// Overwrite existing configuration file
        #[arg(long)]
        force: bool,
    },

    /// Display the current effective configuration
    Show {
        /// Output as JSON instead of TOML
        #[arg(long)]
        json: bool,
    },

    /// Get a specific configuration value
    Get {
        /// Configuration key (e.g., convert.format, `batch.continue_on_error`)
        #[arg(value_name = "KEY")]
        key: String,
    },

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., convert.format, `batch.continue_on_error`)
        #[arg(value_name = "KEY")]
        key: String,

        /// Value to set
        #[arg(value_name = "VALUE")]
        value: String,

        /// Set in user config (~/.docling.toml) instead of project config
        #[arg(long)]
        global: bool,
    },

    /// Reset configuration to defaults
    Reset {
        /// Reset user config (~/.docling.toml) instead of project config
        #[arg(long)]
        global: bool,

        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Show the path(s) to configuration file(s)
    Path {
        /// Show all config file paths (user and project)
        #[arg(long)]
        all: bool,
    },
}

#[allow(clippy::option_if_let_else)] // Complex nested if-let patterns with error handling
fn main() -> Result<()> {
    // Load configuration files
    let (user_config, project_config) = Config::discover_configs();
    let config = Config::merge(user_config, project_config);

    let args = Args::parse();

    // Extract global verbosity settings
    let verbosity = Verbosity::from_flags(args.quiet, args.verbose);

    match args.command {
        Commands::Convert {
            input,
            output,
            format,
            input_format,
            backend,
            compact,
            ocr,
            device,
            batch_size,
            model_size,
            no_tables,
            ml_backend,
            max_pages,
            force,
            no_clobber,
            dry_run,
            watch,
            profile,
        } => {
            // Apply config defaults (CLI args override config)
            // Profile takes precedence over default convert config
            let default_config = if let Some(ref profile_name) = profile {
                // Look up profile in config
                if let Some(ref profiles) = config.profiles {
                    if let Some(profile_config) = profiles.get(profile_name) {
                        profile_config.clone()
                    } else {
                        eprintln!(
                            "{} Unknown profile: '{}'. Available profiles: {}",
                            "Error:".red().bold(),
                            profile_name,
                            profiles
                                .keys()
                                .map(String::as_str)
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        std::process::exit(1);
                    }
                } else {
                    eprintln!(
                        "{} No profiles defined in config. Add profiles to .docling.toml:",
                        "Error:".red().bold()
                    );
                    eprintln!("\n  [profiles.fast]");
                    eprintln!("  format = \"markdown\"");
                    eprintln!("  ocr = false");
                    eprintln!("\n  [profiles.accurate]");
                    eprintln!("  format = \"markdown\"");
                    eprintln!("  ocr = true");
                    std::process::exit(1);
                }
            } else {
                config.convert.unwrap_or_default()
            };

            if watch {
                // Watch mode - re-convert on file changes
                watch_and_convert(
                    input,
                    output,
                    format,
                    input_format,
                    backend,
                    compact,
                    ocr,
                    &default_config,
                    device,
                    batch_size,
                    model_size,
                    no_tables,
                    ml_backend,
                    max_pages,
                    force,
                    verbosity,
                )
            } else {
                convert_command(
                    input,
                    output,
                    format,
                    input_format,
                    backend,
                    compact,
                    ocr,
                    &default_config,
                    device,
                    batch_size,
                    model_size,
                    no_tables,
                    ml_backend,
                    max_pages,
                    force,
                    no_clobber,
                    dry_run,
                    verbosity,
                )
            }
        }

        Commands::Batch {
            inputs,
            output,
            format,
            continue_on_error,
            max_file_size,
            ocr,
            compact,
            stdin,
            parallel,
        } => {
            // Apply config defaults (CLI args override config)
            let default_config = config.batch.unwrap_or_default();
            batch_command(
                inputs,
                output,
                format,
                continue_on_error,
                max_file_size,
                ocr,
                compact,
                stdin,
                parallel,
                &default_config,
                verbosity,
            )
        }

        Commands::Benchmark {
            inputs,
            iterations,
            warmup,
            format,
            output,
            ocr,
        } => {
            // Apply config defaults (CLI args override config)
            let default_config = config.benchmark.unwrap_or_default();
            benchmark_command(
                inputs,
                iterations,
                warmup,
                format,
                output,
                ocr,
                &default_config,
                verbosity,
            )
        }
        Commands::Completion { shell } => completion_command(shell),
        Commands::Formats { filter, json } => formats_command(filter, json),
        Commands::Info { input, json, deep } => info_command(input, json, deep, verbosity),
        Commands::Config { action } => config_command(action, verbosity),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "CLI command handler - args mirror CLI options"
)]
fn convert_command(
    input: String,
    output: Option<PathBuf>,
    format: Option<OutputFormat>,
    cli_input_format: Option<String>,
    backend: Option<Backend>,
    compact: bool,
    ocr: bool,
    config: &ConvertConfig,
    device: MlDevice,
    batch_size: Option<usize>,
    model_size: ModelSize,
    no_tables: bool,
    ml_backend: MlBackend,
    max_pages: Option<usize>,
    force: bool,
    no_clobber: bool,
    dry_run: bool,
    verbosity: Verbosity,
) -> Result<()> {
    use std::io::Read;
    use tempfile::NamedTempFile;

    // Resolve final values with precedence: CLI > config > defaults
    let format = Config::resolve_output_format(format, config.format.as_deref());
    let backend = Config::resolve_backend(backend, config.backend.as_deref());
    let compact = compact || config.compact.unwrap_or(false);
    let ocr = ocr || config.ocr.unwrap_or(false);

    // Pass ML options to PDF backend
    // Note: device, batch_size, model_size, no_tables, max_pages are not yet wired through
    // These can be added to PdfMlConfig/DocumentConverter when needed
    let _ = (device, batch_size, model_size, no_tables, max_pages);
    // ml_backend is now passed through to the PDF backend

    // Handle stdin ("-") or regular file input
    let (input_path, _temp_file, is_stdin) = if input == "-" {
        // Reading from stdin - requires --input-format
        let Some(fmt) = &cli_input_format else {
            eprintln!(
                "{} --input-format is required when reading from stdin",
                "Error:".red().bold()
            );
            eprintln!(
                "{} Example: curl ... | docling convert - --input-format pdf",
                "Help:".cyan().bold()
            );
            std::process::exit(1);
        };
        let ext = fmt.to_lowercase();

        // Read stdin into a temporary file
        let mut buffer = Vec::new();
        io::stdin()
            .read_to_end(&mut buffer)
            .context("Failed to read from stdin")?;

        // Create temp file with proper extension for format detection
        let temp_file = NamedTempFile::with_suffix(format!(".{ext}"))
            .context("Failed to create temporary file")?;
        fs::write(temp_file.path(), &buffer)
            .context("Failed to write stdin content to temporary file")?;

        (temp_file.path().to_path_buf(), Some(temp_file), true)
    } else {
        let path = PathBuf::from(&input);

        // Verify input file exists
        if !path.exists() {
            eprintln!(
                "{} Input file not found: {}",
                "Error:".red().bold(),
                path.display()
            );
            eprintln!(
                "{} Check that the file path is correct and the file exists",
                "Help:".cyan().bold()
            );
            anyhow::bail!("Input file not found: {}", path.display());
        }

        (path, None, false)
    };

    // Detect input format from file extension (or use CLI-provided format)
    let detected_format = if let Some(ref fmt) = cli_input_format {
        // Parse the CLI-provided format string
        detect_format(&PathBuf::from(format!("dummy.{fmt}")))?
    } else {
        detect_format(&input_path)?
    };

    // Determine which backend to use (always Rust now that Python is removed)
    let _use_rust_backend = match backend {
        Backend::Rust => true,
        Backend::Auto => is_rust_backend_supported(detected_format),
    };

    // Determine output path: for stdin, default to stdout (None) unless -o specified
    let output_path = if is_stdin {
        output // For stdin, None means stdout
    } else {
        Some(output.unwrap_or_else(|| smart_output_path(&input_path, &format)))
    };

    // Handle --dry-run: show what would be converted without doing it
    if dry_run {
        let format_name = match format {
            OutputFormat::Markdown => "Markdown",
            OutputFormat::Json => "JSON",
            OutputFormat::Yaml => "YAML",
        };
        let input_display = if is_stdin {
            "stdin".to_string()
        } else {
            input_path.display().to_string()
        };
        let output_display = output_path
            .as_ref()
            .map_or_else(|| "stdout".to_string(), |p| p.display().to_string());
        println!("Would convert: {input_display} → {output_display} ({format_name})");
        return Ok(());
    }

    // Create spinner for conversion (only if not quiet)
    let spinner = if verbosity.should_show_output() {
        let s = ProgressBar::new_spinner();
        s.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("template is compile-time constant")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        let input_name = if is_stdin {
            "stdin".to_string()
        } else {
            input_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };
        s.set_message(format!(
            "Converting {} using {} backend...",
            input_name,
            "Rust".cyan()
        ));
        s.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(s)
    } else {
        None
    };

    // Verbose: show timing info
    let start_time = std::time::Instant::now();

    // Convert document using Rust backend
    let document = convert_with_rust_backend(&input_path, detected_format, ocr, ml_backend)?;

    let conversion_time = start_time.elapsed();
    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    // Verbose output
    if verbosity.is_verbose() {
        eprintln!(
            "{} Conversion completed in {:.2}s",
            "Info:".blue().bold(),
            conversion_time.as_secs_f64()
        );
    }

    // Serialize to requested format (with optional spinner)
    let serialize_spinner = if verbosity.should_show_output() {
        let s = ProgressBar::new_spinner();
        s.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} {msg}")
                .expect("template is compile-time constant")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        s.set_message(format!(
            "Serializing to {}...",
            match format {
                OutputFormat::Markdown => "Markdown".to_string(),
                OutputFormat::Json => "JSON".to_string(),
                OutputFormat::Yaml => "YAML".to_string(),
            }
        ));
        s.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(s)
    } else {
        None
    };

    let output_str = match format {
        OutputFormat::Markdown => {
            // Use the markdown field directly (already converted)
            // Trim trailing whitespace to match Python docling output
            document.to_markdown().trim_end().to_string()
        }
        OutputFormat::Json => {
            let serializer = if compact {
                JsonSerializer::with_options(docling_core::JsonOptions {
                    pretty: false,
                    indent: "  ".to_string(),
                })
            } else {
                JsonSerializer::new()
            };
            serializer
                .serialize_document(&document)
                .context("Failed to serialize to JSON")?
        }
        OutputFormat::Yaml => {
            let serializer = YamlSerializer::new();
            serializer
                .serialize_document(&document)
                .context("Failed to serialize to YAML")?
        }
    };

    if let Some(s) = serialize_spinner {
        s.finish_and_clear();
    }

    // Write output to file or stdout
    match output_path {
        Some(ref path) => {
            // Check for existing file and handle --force / --no-clobber flags
            if path.exists() && !force {
                if no_clobber {
                    eprintln!(
                        "{} Output file already exists: {} (--no-clobber specified)",
                        "Error:".red().bold(),
                        path.display()
                    );
                    std::process::exit(1);
                }
                eprintln!(
                    "{} Output file already exists: {}",
                    "Error:".red().bold(),
                    path.display()
                );
                eprintln!(
                    "{} Use --force to overwrite existing files",
                    "Help:".cyan().bold()
                );
                std::process::exit(1);
            }

            // Write output to file
            fs::write(path, &output_str)
                .with_context(|| format!("Failed to write output file: {}", path.display()))?;
            if verbosity.should_show_output() {
                eprintln!(
                    "{} Output written to: {}",
                    "✓".green().bold(),
                    path.display().to_string().bright_white()
                );
            }
            // Verbose: show additional details
            if verbosity.is_verbose() {
                eprintln!(
                    "{} Output size: {} ({} chars)",
                    "Info:".blue().bold(),
                    format_bytes(output_str.len()),
                    output_str.len()
                );
            }
        }
        None => {
            // Write to stdout (for stdin input without -o)
            print!("{output_str}");
        }
    }

    Ok(())
}

/// Watch a file for changes and re-convert automatically.
///
/// Uses the notify library with debouncing to detect file changes
/// and trigger reconversion. Runs until interrupted (Ctrl+C).
#[allow(
    clippy::too_many_arguments,
    reason = "wraps convert_command with same args"
)]
fn watch_and_convert(
    input: String,
    output: Option<PathBuf>,
    format: Option<OutputFormat>,
    input_format: Option<String>,
    backend: Option<Backend>,
    compact: bool,
    ocr: bool,
    config: &ConvertConfig,
    device: MlDevice,
    batch_size: Option<usize>,
    model_size: ModelSize,
    no_tables: bool,
    ml_backend: MlBackend,
    max_pages: Option<usize>,
    force: bool,
    verbosity: Verbosity,
) -> Result<()> {
    // Watch mode doesn't work with stdin
    if input == "-" {
        eprintln!(
            "{} --watch cannot be used with stdin input",
            "Error:".red().bold()
        );
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&input);

    // Verify input file exists
    if !input_path.exists() {
        eprintln!(
            "{} Input file not found: {}",
            "Error:".red().bold(),
            input_path.display()
        );
        std::process::exit(1);
    }

    // Get the directory to watch (parent of input file)
    let watch_dir = input_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Set up the channel for receiving file events
    let (tx, rx) = channel();

    // Create a debouncer with 500ms delay to avoid rapid re-triggers
    let mut debouncer =
        new_debouncer(Duration::from_millis(500), tx).context("Failed to create file watcher")?;

    // Start watching the directory
    debouncer
        .watcher()
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .context("Failed to watch directory")?;

    if verbosity.should_show_output() {
        eprintln!(
            "{} Watching {} for changes... (Ctrl+C to stop)",
            "Watch:".cyan().bold(),
            input_path.display()
        );
    }

    // Initial conversion
    let result = convert_command(
        input.clone(),
        output.clone(),
        format,
        input_format.clone(),
        backend,
        compact,
        ocr,
        config,
        device,
        batch_size,
        model_size,
        no_tables,
        ml_backend,
        max_pages,
        force, // In watch mode, always allow overwrite
        false, // no_clobber = false (we always want to overwrite in watch mode)
        false, // dry_run = false
        verbosity,
    );

    if let Err(e) = result {
        eprintln!("{} Initial conversion failed: {}", "Error:".red().bold(), e);
    }

    // Watch loop
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Check if any event is for our input file
                let input_canonical = input_path
                    .canonicalize()
                    .unwrap_or_else(|_| input_path.clone());
                let relevant_event = events.iter().any(|event| {
                    event.kind == DebouncedEventKind::Any && {
                        let event_canonical = event
                            .path
                            .canonicalize()
                            .unwrap_or_else(|_| event.path.clone());
                        event_canonical == input_canonical
                    }
                });

                if relevant_event {
                    if verbosity.should_show_output() {
                        eprintln!("\n{} File changed, reconverting...", "Watch:".cyan().bold());
                    }

                    let result = convert_command(
                        input.clone(),
                        output.clone(),
                        format,
                        input_format.clone(),
                        backend,
                        compact,
                        ocr,
                        config,
                        device,
                        batch_size,
                        model_size,
                        no_tables,
                        ml_backend,
                        max_pages,
                        true,  // force = true (always overwrite in watch mode)
                        false, // no_clobber = false
                        false, // dry_run = false
                        verbosity,
                    );

                    if let Err(e) = result {
                        eprintln!("{} Conversion failed: {}", "Error:".red().bold(), e);
                    }
                }
            }
            Ok(Err(error)) => {
                eprintln!("{} Watch error: {}", "Error:".red().bold(), error);
            }
            Err(_) => {
                // Channel disconnected, exit gracefully
                break;
            }
        }
    }

    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    clippy::option_if_let_else, // Complex conditional logic in parallel selection
    reason = "function mirrors CLI args structure"
)]
fn batch_command(
    inputs: Vec<PathBuf>,
    output_dir: PathBuf,
    format: Option<OutputFormat>,
    continue_on_error: bool,
    max_file_size: Option<usize>,
    ocr: bool,
    compact: bool,
    read_from_stdin: bool,
    parallel: Option<usize>,
    config: &BatchConfig,
    verbosity: Verbosity,
) -> Result<()> {
    use rayon::prelude::*;
    use std::io::BufRead;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    // Process result type
    struct ProcessResult {
        input_path: PathBuf,
        output_path: PathBuf,
        success: bool,
        error: Option<String>,
        skipped: bool,
        skip_reason: Option<String>,
        latency_secs: f64,
        output_len: usize,
    }

    // Resolve final values with precedence: CLI > config > defaults
    let format = Config::resolve_output_format(format, config.format.as_deref());
    let continue_on_error = continue_on_error || config.continue_on_error.unwrap_or(false);
    let max_file_size = max_file_size.or(config.max_file_size);
    let ocr = ocr || config.ocr.unwrap_or(false);
    let compact = compact || config.compact.unwrap_or(false);

    // Determine parallelism level
    // Default to sequential (1) for fail-fast behavior when not using continue_on_error
    // Use parallel (CPU cores) when continue_on_error is enabled
    let num_workers = match parallel {
        Some(n) => n.max(1), // Explicit parallel count
        None => {
            if continue_on_error {
                rayon::current_num_threads() // Parallel by default with continue_on_error
            } else {
                1 // Sequential by default for fail-fast behavior
            }
        }
    };

    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir).with_context(|| {
            format!(
                "Failed to create output directory: {}",
                output_dir.display()
            )
        })?;
    }

    // Verify output directory is actually a directory
    if !output_dir.is_dir() {
        eprintln!(
            "{} Output path is not a directory: {}",
            "Error:".red().bold(),
            output_dir.display()
        );
        anyhow::bail!("Output path is not a directory: {}", output_dir.display());
    }

    // Gather input files from stdin if requested
    let inputs = if read_from_stdin {
        let stdin = io::stdin();
        let lines: Vec<PathBuf> = stdin
            .lock()
            .lines()
            .map_while(Result::ok)
            .filter(|line| !line.trim().is_empty())
            .map(PathBuf::from)
            .collect();
        lines
    } else {
        inputs
    };

    // Expand glob patterns if present (shell may not expand them in all cases)
    let expanded_inputs = expand_glob_patterns(&inputs)?;

    if expanded_inputs.is_empty() {
        eprintln!("{} No input files found", "Error:".red().bold());
        anyhow::bail!("No input files found");
    }

    let total_files = expanded_inputs.len();
    if verbosity.should_show_output() {
        let parallel_msg = if num_workers > 1 {
            format!(" with {} workers", num_workers.to_string().cyan())
        } else {
            " (sequential)".to_string()
        };
        eprintln!(
            "{} Processing {} files{}...",
            "Info:".blue().bold(),
            total_files.to_string().cyan(),
            parallel_msg
        );
    }

    // Configure rayon thread pool if parallel count specified
    if let Some(n) = parallel {
        if n > 0 {
            rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build_global()
                .ok(); // Ignore error if pool already built
        }
    }

    // Create progress bar (hidden in quiet mode)
    let progress = if verbosity.should_show_output() {
        let pb = ProgressBar::new(total_files as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                .expect("template is compile-time constant")
                .progress_chars("█▓▒░  "),
        );
        pb
    } else {
        ProgressBar::hidden()
    };

    // Track statistics with atomic counters for thread safety
    let succeeded = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let skipped = Arc::new(AtomicUsize::new(0));
    let start_time = std::time::Instant::now();

    // Early termination flag for fail-fast behavior
    let should_stop = Arc::new(AtomicBool::new(false));

    // Clone should_stop for the closure
    let should_stop_clone = Arc::clone(&should_stop);

    // Helper closure for processing a single file
    let process_file = |input_path: &PathBuf| -> ProcessResult {
        // Check for early termination
        if !continue_on_error && should_stop_clone.load(Ordering::SeqCst) {
            return ProcessResult {
                input_path: input_path.clone(),
                output_path: PathBuf::new(),
                success: false,
                error: Some("Skipped due to previous error".to_string()),
                skipped: true,
                skip_reason: Some("previous error".to_string()),
                latency_secs: 0.0,
                output_len: 0,
            };
        }

        // Check file size limit before processing
        if let Some(max_size) = max_file_size {
            if let Ok(metadata) = fs::metadata(input_path) {
                let file_size = metadata.len() as usize;
                if file_size > max_size {
                    return ProcessResult {
                        input_path: input_path.clone(),
                        output_path: PathBuf::new(),
                        success: false,
                        error: None,
                        skipped: true,
                        skip_reason: Some(format!(
                            "{} exceeds {} limit",
                            format_bytes(file_size),
                            format_bytes(max_size)
                        )),
                        latency_secs: 0.0,
                        output_len: 0,
                    };
                }
            }
        }

        // Create converter for this thread
        let converter = match if ocr {
            DocumentConverter::with_ocr(true)
        } else {
            DocumentConverter::new()
        } {
            Ok(c) => c,
            Err(e) => {
                return ProcessResult {
                    input_path: input_path.clone(),
                    output_path: PathBuf::new(),
                    success: false,
                    error: Some(format!("Failed to create converter: {e}")),
                    skipped: false,
                    skip_reason: None,
                    latency_secs: 0.0,
                    output_len: 0,
                };
            }
        };

        let start = std::time::Instant::now();
        let result = converter.convert(input_path);
        let latency = start.elapsed().as_secs_f64();

        match result {
            Ok(conv_result) => {
                // Generate output filename (same name, different extension)
                let output_filename = input_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let extension = match format {
                    OutputFormat::Markdown => "md",
                    OutputFormat::Json => "json",
                    OutputFormat::Yaml => "yaml",
                };

                let output_path = output_dir.join(format!("{output_filename}.{extension}"));

                // Serialize document
                let output_str = match format {
                    OutputFormat::Markdown => {
                        conv_result.document.to_markdown().trim_end().to_string()
                    }
                    OutputFormat::Json => {
                        let serializer = if compact {
                            JsonSerializer::with_options(docling_core::JsonOptions {
                                pretty: false,
                                indent: "  ".to_string(),
                            })
                        } else {
                            JsonSerializer::new()
                        };
                        match serializer.serialize_document(&conv_result.document) {
                            Ok(s) => s,
                            Err(e) => {
                                return ProcessResult {
                                    input_path: input_path.clone(),
                                    output_path,
                                    success: false,
                                    error: Some(format!("Failed to serialize to JSON: {e}")),
                                    skipped: false,
                                    skip_reason: None,
                                    latency_secs: latency,
                                    output_len: 0,
                                };
                            }
                        }
                    }
                    OutputFormat::Yaml => {
                        let serializer = YamlSerializer::new();
                        match serializer.serialize_document(&conv_result.document) {
                            Ok(s) => s,
                            Err(e) => {
                                return ProcessResult {
                                    input_path: input_path.clone(),
                                    output_path,
                                    success: false,
                                    error: Some(format!("Failed to serialize to YAML: {e}")),
                                    skipped: false,
                                    skip_reason: None,
                                    latency_secs: latency,
                                    output_len: 0,
                                };
                            }
                        }
                    }
                };

                let output_len = output_str.len();

                // Write output file
                if let Err(e) = fs::write(&output_path, output_str) {
                    return ProcessResult {
                        input_path: input_path.clone(),
                        output_path,
                        success: false,
                        error: Some(format!("Failed to write output: {e}")),
                        skipped: false,
                        skip_reason: None,
                        latency_secs: latency,
                        output_len: 0,
                    };
                }

                ProcessResult {
                    input_path: input_path.clone(),
                    output_path,
                    success: true,
                    error: None,
                    skipped: false,
                    skip_reason: None,
                    latency_secs: latency,
                    output_len,
                }
            }
            Err(e) => {
                // Set stop flag for fail-fast behavior
                if !continue_on_error {
                    should_stop_clone.store(true, Ordering::SeqCst);
                }
                ProcessResult {
                    input_path: input_path.clone(),
                    output_path: PathBuf::new(),
                    success: false,
                    error: Some(e.to_string()),
                    skipped: false,
                    skip_reason: None,
                    latency_secs: latency,
                    output_len: 0,
                }
            }
        }
    };

    // Process files: use parallel iterator for multi-threaded, sequential for single-threaded
    let results: Vec<ProcessResult> = if num_workers > 1 {
        expanded_inputs
            .par_iter()
            .map(|input_path| {
                let result = process_file(input_path);
                progress.inc(1);
                result
            })
            .collect()
    } else {
        // Sequential processing for fail-fast behavior
        expanded_inputs
            .iter()
            .map(|input_path| {
                let result = process_file(input_path);
                progress.inc(1);
                result
            })
            .collect()
    };

    progress.finish_and_clear();

    // Process results and update counters
    let mut first_error: Option<String> = None;
    for result in &results {
        if result.skipped {
            skipped.fetch_add(1, Ordering::SeqCst);
            if verbosity.should_show_output() {
                let filename = result
                    .input_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                eprintln!(
                    "{} Skipped {} ({})",
                    "Skip:".yellow().bold(),
                    filename,
                    result.skip_reason.as_deref().unwrap_or("unknown reason")
                );
            }
        } else if result.success {
            succeeded.fetch_add(1, Ordering::SeqCst);
            if verbosity.is_verbose() {
                let input_name = result
                    .input_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                let output_name = result
                    .output_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                eprintln!(
                    "{} {} -> {} ({:.2}s, {} chars)",
                    "✓".green().bold(),
                    input_name.bright_white(),
                    output_name.bright_black(),
                    result.latency_secs,
                    result.output_len
                );
            }
        } else {
            failed.fetch_add(1, Ordering::SeqCst);
            let input_name = result
                .input_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let error_msg = result.error.as_deref().unwrap_or("Unknown error");

            if !continue_on_error && first_error.is_none() {
                first_error = Some(format!("{input_name}: {error_msg}"));
            }

            if verbosity.should_show_output() || !continue_on_error {
                eprintln!(
                    "{} {} - {}",
                    "✗".red().bold(),
                    input_name.bright_white(),
                    error_msg.red()
                );
            }
        }
    }

    let elapsed = start_time.elapsed();
    let total = total_files;
    let succeeded_count = succeeded.load(Ordering::SeqCst);
    let failed_count = failed.load(Ordering::SeqCst);
    let skipped_count = skipped.load(Ordering::SeqCst);

    // Print summary with colors (skip in quiet mode)
    if verbosity.should_show_output() {
        eprintln!("\n{}", "=== Batch Conversion Summary ===".bold());
        eprintln!("{:<16} {}", "Total files:", total.to_string().cyan());
        eprintln!(
            "{:<16} {}",
            "Succeeded:",
            succeeded_count.to_string().green()
        );
        eprintln!(
            "{:<16} {}",
            "Failed:",
            if failed_count > 0 {
                failed_count.to_string().red()
            } else {
                failed_count.to_string().normal()
            }
        );
        eprintln!("{:<16} {}", "Skipped:", skipped_count.to_string().yellow());
        eprintln!("{:<16} {:.2}s", "Total time:", elapsed.as_secs_f64());
        eprintln!(
            "{:<16} {:.2}s per file",
            "Average time:",
            if total > 0 {
                elapsed.as_secs_f64() / total as f64
            } else {
                0.0
            }
        );
        if num_workers > 1 {
            eprintln!("{:<16} {}", "Workers:", num_workers.to_string().cyan());
        }
    }

    if let Some(err) = first_error {
        if !continue_on_error {
            anyhow::bail!("Batch conversion failed: {err}");
        }
    }

    if failed_count > 0 && !continue_on_error {
        anyhow::bail!("Batch conversion failed with {failed_count} errors");
    }

    Ok(())
}

/// Expand glob patterns to list of concrete file paths
/// Note: Does NOT validate file existence - allows converter to handle errors
fn expand_glob_patterns(patterns: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut expanded = Vec::new();

    for pattern in patterns {
        // If pattern contains glob characters, try to expand
        let pattern_str = pattern.to_string_lossy();
        if pattern_str.contains('*') || pattern_str.contains('?') || pattern_str.contains('[') {
            // Use glob crate to expand pattern
            use glob::glob;

            let matches = glob(&pattern_str)
                .with_context(|| format!("Invalid glob pattern: {pattern_str}"))?;

            for entry in matches {
                let path = entry.with_context(|| "Failed to read glob entry")?;
                if path.is_file() {
                    expanded.push(path);
                }
            }
        } else {
            // Not a glob pattern, add as-is (let converter handle validation)
            expanded.push(pattern.clone());
        }
    }

    Ok(expanded)
}

#[allow(
    clippy::too_many_arguments,
    reason = "CLI command handler - args mirror CLI options"
)]
fn benchmark_command(
    inputs: Vec<PathBuf>,
    iterations: Option<usize>,
    warmup: Option<usize>,
    format: Option<BenchFormat>,
    output: Option<PathBuf>,
    _ocr: bool,
    config: &BenchmarkConfigSettings,
    verbosity: Verbosity,
) -> Result<()> {
    use benchmark::{BenchmarkConfig, BenchmarkRunner};

    // Resolve final values with precedence: CLI > config > defaults
    let iterations = iterations.or(config.iterations).unwrap_or(3);
    let warmup = warmup.or(config.warmup).unwrap_or(1);
    let format = format.unwrap_or(BenchFormat::Text);

    // Verify all input files exist
    for input in &inputs {
        if !input.exists() {
            eprintln!(
                "{} Input file not found: {}",
                "Error:".red().bold(),
                input.display()
            );
            eprintln!(
                "{} Check that the file path is correct and the file exists",
                "Help:".cyan().bold()
            );
            anyhow::bail!("Input file not found: {}", input.display());
        }
    }

    // Create benchmark configuration
    let bench_config = BenchmarkConfig {
        iterations,
        warmup_iterations: warmup,
    };

    if verbosity.should_show_output() {
        eprintln!(
            "{} Running benchmark with {} iterations ({} warmup)...",
            "Info:".blue().bold(),
            iterations.to_string().cyan(),
            warmup.to_string().cyan()
        );
        eprintln!();
    }

    // Create progress spinner (hidden in quiet mode)
    let spinner = if verbosity.should_show_output() {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("template is compile-time constant")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        sp.set_message("Running benchmarks...".to_string());
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        sp
    } else {
        ProgressBar::hidden()
    };

    // Run benchmarks
    let runner = BenchmarkRunner::new(bench_config);
    let results = runner.run_benchmarks(&inputs);

    spinner.finish_and_clear();

    if results.is_empty() {
        if verbosity.should_show_output() {
            eprintln!("{} No successful benchmarks", "Warning:".yellow().bold());
        }
        return Ok(());
    }

    // Format output
    let output_str = match format {
        BenchFormat::Text => BenchmarkRunner::format_as_text(&results),
        BenchFormat::Json => BenchmarkRunner::format_as_json(&results),
        BenchFormat::Csv => BenchmarkRunner::format_as_csv(&results),
    };

    // Write output
    if let Some(output_path) = output {
        fs::write(&output_path, &output_str)
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
        if verbosity.should_show_output() {
            eprintln!(
                "{} Benchmark results written to: {}",
                "✓".green().bold(),
                output_path.display().to_string().bright_white()
            );
        }
    } else {
        // Write to stdout
        print!("{output_str}");
    }

    Ok(())
}

/// Detect input format from file extension
fn detect_format(path: &Path) -> Result<InputFormat> {
    // Get extension or provide helpful error
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        eprintln!(
            "{} File has no extension: {}",
            "Error:".red().bold(),
            path.display()
        );
        eprintln!(
            "{} Add a file extension (e.g., .pdf, .docx, .html) or specify format explicitly",
            "Help:".cyan().bold()
        );
        anyhow::bail!("File has no extension");
    };
    let extension = ext.to_lowercase();

    match extension.as_str() {
        "pdf" => Ok(InputFormat::Pdf),
        "docx" => Ok(InputFormat::Docx),
        "doc" => Ok(InputFormat::Doc),
        "pptx" => Ok(InputFormat::Pptx),
        "xlsx" => Ok(InputFormat::Xlsx),
        "html" | "htm" => Ok(InputFormat::Html),
        "csv" => Ok(InputFormat::Csv),
        "md" | "markdown" => Ok(InputFormat::Md),
        "asciidoc" | "adoc" => Ok(InputFormat::Asciidoc),
        "nxml" | "xml" => Ok(InputFormat::Jats),
        "vtt" | "webvtt" => Ok(InputFormat::Webvtt),
        "srt" => Ok(InputFormat::Srt),
        "png" => Ok(InputFormat::Png),
        "jpg" | "jpeg" => Ok(InputFormat::Jpeg),
        "tiff" | "tif" => Ok(InputFormat::Tiff),
        "webp" => Ok(InputFormat::Webp),
        "bmp" => Ok(InputFormat::Bmp),
        "gif" => Ok(InputFormat::Gif),
        "svg" => Ok(InputFormat::Svg),
        "avif" => Ok(InputFormat::Avif),
        "epub" => Ok(InputFormat::Epub),
        "fb2" => Ok(InputFormat::Fb2),
        "mobi" | "prc" | "azw" => Ok(InputFormat::Mobi),
        "eml" => Ok(InputFormat::Eml),
        "mbox" | "mbx" => Ok(InputFormat::Mbox),
        "vcf" | "vcard" => Ok(InputFormat::Vcf),
        "msg" => Ok(InputFormat::Msg),
        "zip" => Ok(InputFormat::Zip),
        "tar" | "tgz" | "gz" | "bz2" => Ok(InputFormat::Tar),
        "7z" => Ok(InputFormat::SevenZ),
        "rar" => Ok(InputFormat::Rar),
        "odt" => Ok(InputFormat::Odt),
        "ods" => Ok(InputFormat::Ods),
        "odp" => Ok(InputFormat::Odp),
        "tex" | "latex" => Ok(InputFormat::Tex),
        "pub" => Ok(InputFormat::Pub),
        "pages" => Ok(InputFormat::Pages),
        "numbers" => Ok(InputFormat::Numbers),
        "key" => Ok(InputFormat::Key),
        "vsdx" => Ok(InputFormat::Vsdx),
        "mpp" => Ok(InputFormat::Mpp),
        "one" => Ok(InputFormat::One),
        "mdb" | "accdb" => Ok(InputFormat::Mdb),
        // Document formats
        "xps" | "oxps" => Ok(InputFormat::Xps),
        "rtf" => Ok(InputFormat::Rtf),
        "idml" => Ok(InputFormat::Idml),
        // Data/scientific formats
        "ics" | "ical" => Ok(InputFormat::Ics),
        "gpx" => Ok(InputFormat::Gpx),
        "kml" => Ok(InputFormat::Kml),
        "kmz" => Ok(InputFormat::Kmz),
        "ipynb" => Ok(InputFormat::Ipynb),
        // Image formats (extended)
        "heif" | "heic" => Ok(InputFormat::Heif),
        "dicom" | "dcm" => Ok(InputFormat::Dicom),
        // CAD/3D formats
        "dxf" => Ok(InputFormat::Dxf),
        "stl" => Ok(InputFormat::Stl),
        "obj" => Ok(InputFormat::Obj),
        "gltf" => Ok(InputFormat::Gltf),
        "glb" => Ok(InputFormat::Glb),
        _ => {
            eprintln!(
                "{} Unsupported file extension: .{}",
                "Error:".red().bold(),
                extension
            );
            eprintln!();
            eprintln!("{} Supported document formats:", "Help:".cyan().bold());
            eprintln!(
                "  {} pdf, docx, doc, pptx, xlsx, html, htm, xps, oxps, rtf, idml",
                "Documents:".bright_white()
            );
            eprintln!(
                "  {} md, markdown, asciidoc, adoc, csv",
                "Text:".bright_white()
            );
            eprintln!("  {} odt, ods, odp", "OpenDocument:".bright_white());
            eprintln!(
                "  {} png, jpg, jpeg, tiff, tif, webp, bmp, gif, svg, avif, heif, heic, dicom, dcm",
                "Images:".bright_white()
            );
            eprintln!("  {} epub, fb2, mobi, prc, azw", "E-books:".bright_white());
            eprintln!(
                "  {} eml, mbox, mbx, msg, vcf, vcard",
                "Email:".bright_white()
            );
            eprintln!(
                "  {} zip, tar, tgz, gz, bz2, 7z, rar",
                "Archives:".bright_white()
            );
            eprintln!("  {} vtt, webvtt, srt", "Subtitles:".bright_white());
            eprintln!(
                "  {} nxml, xml, ics, ical, gpx, kml, kmz, ipynb",
                "Scientific/Data:".bright_white()
            );
            eprintln!("  {} dxf, stl, obj, gltf, glb", "CAD/3D:".bright_white());
            eprintln!();
            anyhow::bail!("Unsupported file extension: .{extension}")
        }
    }
}

/// Check if Rust backend supports this format
const fn is_rust_backend_supported(format: InputFormat) -> bool {
    matches!(
        format,
        InputFormat::Pdf
            | InputFormat::Zip
            | InputFormat::Tar
            | InputFormat::SevenZ
            | InputFormat::Rar
            | InputFormat::Srt
            | InputFormat::Webvtt
            | InputFormat::Eml
            | InputFormat::Mbox
            | InputFormat::Msg
            | InputFormat::Vcf
            | InputFormat::Epub
            | InputFormat::Fb2
            | InputFormat::Mobi
            | InputFormat::Odt
            | InputFormat::Ods
            | InputFormat::Odp
            | InputFormat::Svg
            | InputFormat::Bmp
            | InputFormat::Gif
            | InputFormat::Avif
            | InputFormat::Heif
            | InputFormat::Rtf
            | InputFormat::Xps
            | InputFormat::Idml
            | InputFormat::Ipynb
            | InputFormat::Gpx
            | InputFormat::Kml
            | InputFormat::Kmz
            | InputFormat::Ics
            | InputFormat::Dicom
            // CAD/3D formats
            | InputFormat::Dxf
            | InputFormat::Stl
            | InputFormat::Obj
            | InputFormat::Gltf
            | InputFormat::Glb
            // Academic/Scientific
            | InputFormat::Tex
            | InputFormat::Jats
            // Microsoft Extended (Pure Rust)
            | InputFormat::Vsdx
            // Office formats (Rust backends)
            | InputFormat::Docx
            | InputFormat::Doc
            | InputFormat::Pptx
            | InputFormat::Xlsx
            // Web/Text formats (Rust backends)
            | InputFormat::Html
            | InputFormat::Md
            | InputFormat::Asciidoc
            | InputFormat::Csv
            // Image formats (Rust backends)
            | InputFormat::Png
            | InputFormat::Jpeg
            | InputFormat::Tiff
            | InputFormat::Webp
    )
}

/// Convert document using Rust backend
fn convert_with_rust_backend(
    path: &Path,
    _format: InputFormat,
    enable_ocr: bool,
    ml_backend: MlBackend,
) -> Result<Document> {
    use docling_backend::{PdfMlBackend, PdfMlConfig, RustDocumentConverter};

    // Convert CLI MlBackend to backend PdfMlBackend
    let pdf_ml_backend = match ml_backend {
        MlBackend::Pytorch => PdfMlBackend::PyTorch,
        MlBackend::Onnx => PdfMlBackend::Onnx,
    };

    // Create PDF ML config (pure Rust pipeline)
    let pdf_ml_config = PdfMlConfig {
        backend: pdf_ml_backend,
        table_structure: true, // Enable table structure by default
    };

    let converter = RustDocumentConverter::with_config(enable_ocr, pdf_ml_config)
        .context("Failed to initialize Rust backend")?;

    let result = converter
        .convert(path)
        .with_context(|| format!("Failed to convert document: {}", path.display()))?;

    Ok(result.document)
}

/// Generate shell completion scripts
#[allow(
    clippy::unnecessary_wraps,
    reason = "consistent return type for CLI commands"
)]
fn completion_command(shell: Shell) -> Result<()> {
    let mut cmd = Args::command();
    let bin_name = cmd.get_name().to_string();

    generate(shell, &mut cmd, bin_name, &mut io::stdout());

    Ok(())
}

/// List supported input formats
#[allow(
    clippy::unnecessary_wraps,
    reason = "consistent return type for CLI commands"
)]
fn formats_command(filter: Option<String>, json_output: bool) -> Result<()> {
    // Define all supported formats with their info
    let formats: Vec<(&str, &str, Vec<&str>, bool, &str)> = vec![
        // Documents
        (
            "pdf",
            "PDF",
            vec!["pdf"],
            true,
            "PDF documents with ML-based layout detection",
        ),
        (
            "docx",
            "Word (DOCX)",
            vec!["docx"],
            true,
            "Microsoft Word 2007+ documents",
        ),
        (
            "doc",
            "Word (DOC)",
            vec!["doc"],
            true,
            "Legacy Microsoft Word documents",
        ),
        (
            "pptx",
            "PowerPoint",
            vec!["pptx"],
            true,
            "Microsoft PowerPoint presentations",
        ),
        (
            "xlsx",
            "Excel",
            vec!["xlsx"],
            true,
            "Microsoft Excel spreadsheets",
        ),
        (
            "html",
            "HTML",
            vec!["html", "htm"],
            true,
            "Web pages and HTML documents",
        ),
        (
            "xps",
            "XPS",
            vec!["xps", "oxps"],
            true,
            "XML Paper Specification",
        ),
        (
            "rtf",
            "RTF",
            vec!["rtf"],
            true,
            "Rich Text Format documents",
        ),
        (
            "idml",
            "InDesign",
            vec!["idml"],
            true,
            "Adobe InDesign Markup Language",
        ),
        // Text/Markup
        (
            "md",
            "Markdown",
            vec!["md", "markdown"],
            true,
            "Markdown documents",
        ),
        (
            "asciidoc",
            "AsciiDoc",
            vec!["asciidoc", "adoc"],
            true,
            "AsciiDoc documents",
        ),
        ("csv", "CSV", vec!["csv"], true, "Comma-separated values"),
        (
            "tex",
            "LaTeX",
            vec!["tex", "latex"],
            true,
            "LaTeX documents",
        ),
        (
            "jats",
            "JATS XML",
            vec!["nxml", "xml"],
            true,
            "Journal Article Tag Suite",
        ),
        // OpenDocument
        ("odt", "ODT", vec!["odt"], true, "OpenDocument Text"),
        ("ods", "ODS", vec!["ods"], true, "OpenDocument Spreadsheet"),
        ("odp", "ODP", vec!["odp"], true, "OpenDocument Presentation"),
        // Images
        ("png", "PNG", vec!["png"], true, "Portable Network Graphics"),
        ("jpeg", "JPEG", vec!["jpg", "jpeg"], true, "JPEG images"),
        (
            "tiff",
            "TIFF",
            vec!["tiff", "tif"],
            true,
            "Tagged Image File Format",
        ),
        ("webp", "WebP", vec!["webp"], true, "WebP images"),
        ("bmp", "BMP", vec!["bmp"], true, "Bitmap images"),
        (
            "gif",
            "GIF",
            vec!["gif"],
            true,
            "Graphics Interchange Format",
        ),
        ("svg", "SVG", vec!["svg"], true, "Scalable Vector Graphics"),
        ("avif", "AVIF", vec!["avif"], true, "AV1 Image File Format"),
        (
            "heif",
            "HEIF",
            vec!["heif", "heic"],
            true,
            "High Efficiency Image Format",
        ),
        (
            "dicom",
            "DICOM",
            vec!["dicom", "dcm"],
            true,
            "Medical imaging format",
        ),
        // E-books
        ("epub", "EPUB", vec!["epub"], true, "Electronic Publication"),
        ("fb2", "FictionBook", vec!["fb2"], true, "FictionBook 2.0"),
        (
            "mobi",
            "Kindle",
            vec!["mobi", "prc", "azw"],
            true,
            "Amazon Kindle formats",
        ),
        // Email
        ("eml", "Email", vec!["eml"], true, "Email message format"),
        (
            "mbox",
            "Mailbox",
            vec!["mbox", "mbx"],
            true,
            "Unix mailbox format",
        ),
        (
            "msg",
            "Outlook MSG",
            vec!["msg"],
            true,
            "Microsoft Outlook message",
        ),
        ("vcf", "vCard", vec!["vcf", "vcard"], true, "Contact cards"),
        // Archives
        ("zip", "ZIP", vec!["zip"], true, "ZIP archives"),
        (
            "tar",
            "TAR",
            vec!["tar", "tgz", "gz", "bz2"],
            true,
            "Tape archives",
        ),
        ("7z", "7-Zip", vec!["7z"], true, "7-Zip archives"),
        ("rar", "RAR", vec!["rar"], true, "RAR archives"),
        // Subtitles
        ("srt", "SRT", vec!["srt"], true, "SubRip subtitles"),
        (
            "webvtt",
            "WebVTT",
            vec!["vtt", "webvtt"],
            true,
            "Web Video Text Tracks",
        ),
        // Scientific/Data
        (
            "ics",
            "iCalendar",
            vec!["ics", "ical"],
            true,
            "Calendar events",
        ),
        ("gpx", "GPX", vec!["gpx"], true, "GPS Exchange Format"),
        ("kml", "KML", vec!["kml"], true, "Keyhole Markup Language"),
        ("kmz", "KMZ", vec!["kmz"], true, "Compressed KML"),
        ("ipynb", "Jupyter", vec!["ipynb"], true, "Jupyter Notebooks"),
        // CAD/3D
        ("dxf", "DXF", vec!["dxf"], true, "AutoCAD Drawing Exchange"),
        ("stl", "STL", vec!["stl"], true, "Stereolithography"),
        ("obj", "OBJ", vec!["obj"], true, "Wavefront OBJ"),
        ("gltf", "glTF", vec!["gltf"], true, "GL Transmission Format"),
        ("glb", "GLB", vec!["glb"], true, "Binary glTF"),
        // Microsoft Extended
        (
            "vsdx",
            "Visio",
            vec!["vsdx"],
            true,
            "Microsoft Visio diagrams",
        ),
        (
            "mpp",
            "MS Project",
            vec!["mpp"],
            true,
            "Microsoft Project files (via LibreOffice)",
        ),
        (
            "one",
            "OneNote",
            vec!["one"],
            false,
            "Microsoft OneNote (not supported)",
        ),
        (
            "pub",
            "Publisher",
            vec!["pub"],
            true,
            "Microsoft Publisher (via LibreOffice)",
        ),
        // Apple
        (
            "pages",
            "Pages",
            vec!["pages"],
            true,
            "Apple Pages documents",
        ),
        (
            "numbers",
            "Numbers",
            vec!["numbers"],
            true,
            "Apple Numbers spreadsheets",
        ),
        (
            "key",
            "Keynote",
            vec!["key"],
            true,
            "Apple Keynote presentations",
        ),
    ];

    // Filter formats if a filter is provided
    let filtered: Vec<_> = if let Some(ref f) = filter {
        let f_lower = f.to_lowercase();
        formats
            .into_iter()
            .filter(|(id, name, exts, _, _)| {
                id.contains(&f_lower)
                    || name.to_lowercase().contains(&f_lower)
                    || exts.iter().any(|e| e.contains(&f_lower))
            })
            .collect()
    } else {
        formats
    };

    if filtered.is_empty() {
        if let Some(f) = filter {
            eprintln!(
                "{} No formats matching '{}' found",
                "Error:".red().bold(),
                f
            );
            std::process::exit(1);
        }
    }

    if json_output {
        // Output as JSON
        let json_formats: Vec<_> = filtered
            .iter()
            .map(|(id, name, exts, supported, desc)| {
                serde_json::json!({
                    "id": id,
                    "name": name,
                    "extensions": exts,
                    "supported": supported,
                    "description": desc
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json_formats).unwrap_or_default()
        );
    } else {
        // Output as formatted table
        println!(
            "\n{} {} supported input formats:\n",
            "docling".cyan().bold(),
            filtered.iter().filter(|(_, _, _, s, _)| *s).count()
        );

        // Group by category for nicer output
        let categories = [
            (
                "Documents",
                vec![
                    "pdf", "docx", "doc", "pptx", "xlsx", "html", "xps", "rtf", "idml",
                ],
            ),
            ("Text/Markup", vec!["md", "asciidoc", "csv", "tex", "jats"]),
            ("OpenDocument", vec!["odt", "ods", "odp"]),
            (
                "Images",
                vec![
                    "png", "jpeg", "tiff", "webp", "bmp", "gif", "svg", "avif", "heif", "dicom",
                ],
            ),
            ("E-books", vec!["epub", "fb2", "mobi"]),
            ("Email", vec!["eml", "mbox", "msg", "vcf"]),
            ("Archives", vec!["zip", "tar", "7z", "rar"]),
            ("Subtitles", vec!["srt", "webvtt"]),
            ("Scientific/Data", vec!["ics", "gpx", "kml", "kmz", "ipynb"]),
            ("CAD/3D", vec!["dxf", "stl", "obj", "gltf", "glb"]),
            ("Microsoft", vec!["vsdx", "mpp", "one", "pub"]),
            ("Apple", vec!["pages", "numbers", "key"]),
        ];

        for (category, ids) in categories {
            let cat_formats: Vec<_> = filtered
                .iter()
                .filter(|(id, _, _, _, _)| ids.contains(id))
                .collect();

            if cat_formats.is_empty() {
                continue;
            }

            println!("  {}:", category.bright_white().bold());
            for (id, _name, exts, supported, desc) in cat_formats {
                let status = if *supported {
                    "✓".green()
                } else {
                    "✗".red()
                };
                let ext_str = exts.join(", ");
                println!(
                    "    {} {:12} {:24} {}",
                    status,
                    id.cyan(),
                    format!("[{ext_str}]").bright_black(),
                    desc
                );
            }
            println!();
        }
    }

    Ok(())
}

/// Document info structure for JSON output
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
struct DocumentInfo {
    file_name: String,
    file_path: String,
    file_size: usize,
    file_size_human: String,
    format: String,
    format_id: String,
    supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    word_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    char_count: Option<usize>,
    modified: Option<String>,
    created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deep_analysis: Option<DeepAnalysis>,
}

/// Deep analysis results
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
struct DeepAnalysis {
    sections: usize,
    tables: usize,
    images: usize,
    links: usize,
    headings: Vec<String>,
}

/// Inspect document metadata and structure
fn info_command(input: PathBuf, json_output: bool, deep: bool, verbosity: Verbosity) -> Result<()> {
    use std::time::UNIX_EPOCH;

    // Check file exists
    if !input.exists() {
        eprintln!(
            "{} File not found: {}",
            "Error:".red().bold(),
            input.display()
        );
        std::process::exit(1);
    }

    // Get basic file metadata
    let metadata = fs::metadata(&input)
        .with_context(|| format!("Failed to read file metadata: {}", input.display()))?;

    let file_size = metadata.len() as usize;
    let file_name = input
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_path = input.display().to_string();

    // Get modification and creation times
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| {
            let secs = d.as_secs();
            // Format as ISO 8601-like timestamp
            let datetime =
                chrono::DateTime::from_timestamp(secs as i64, 0).unwrap_or_else(chrono::Utc::now);
            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        });

    let created = metadata
        .created()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| {
            let secs = d.as_secs();
            let datetime =
                chrono::DateTime::from_timestamp(secs as i64, 0).unwrap_or_else(chrono::Utc::now);
            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        });

    // Detect format
    let detected_format = detect_format(&input)?;
    let format_id = format!("{detected_format:?}").to_lowercase();
    let format_name = get_format_name(detected_format);
    let supported = is_rust_backend_supported(detected_format);

    // Get format-specific info (page count for PDFs, etc.)
    let page_count = get_page_count(&input, detected_format);

    // Deep analysis if requested
    let (deep_analysis, word_count, char_count) = if deep {
        // Do full conversion to get detailed info
        // Show spinner only when not in quiet mode
        let spinner = if verbosity.should_show_output() {
            let sp = ProgressBar::new_spinner();
            sp.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .expect("template is compile-time constant")
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            sp.set_message("Performing deep analysis...");
            sp.enable_steady_tick(std::time::Duration::from_millis(80));
            sp
        } else {
            ProgressBar::hidden()
        };

        let analysis = perform_deep_analysis(&input, detected_format);
        spinner.finish_and_clear();

        match analysis {
            Ok((deep, words, chars)) => (Some(deep), Some(words), Some(chars)),
            Err(_) => (None, None, None),
        }
    } else {
        (None, None, None)
    };

    let info = DocumentInfo {
        file_name,
        file_path,
        file_size,
        file_size_human: format_bytes(file_size),
        format: format_name,
        format_id,
        supported,
        page_count,
        word_count,
        char_count,
        modified,
        created,
        deep_analysis,
    };

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&info).unwrap_or_default()
        );
    } else {
        print_info_text(&info);
    }

    Ok(())
}

/// Get human-readable format name
fn get_format_name(format: InputFormat) -> String {
    match format {
        InputFormat::Pdf => "PDF Document".to_string(),
        InputFormat::Docx => "Microsoft Word (DOCX)".to_string(),
        InputFormat::Doc => "Microsoft Word (DOC)".to_string(),
        InputFormat::Pptx => "Microsoft PowerPoint".to_string(),
        InputFormat::Xlsx => "Microsoft Excel".to_string(),
        InputFormat::Html => "HTML Document".to_string(),
        InputFormat::Md => "Markdown".to_string(),
        InputFormat::Csv => "CSV Spreadsheet".to_string(),
        InputFormat::Asciidoc => "AsciiDoc".to_string(),
        InputFormat::Jats => "JATS XML".to_string(),
        InputFormat::Png => "PNG Image".to_string(),
        InputFormat::Jpeg => "JPEG Image".to_string(),
        InputFormat::Tiff => "TIFF Image".to_string(),
        InputFormat::Webp => "WebP Image".to_string(),
        InputFormat::Bmp => "Bitmap Image".to_string(),
        InputFormat::Gif => "GIF Image".to_string(),
        InputFormat::Svg => "SVG Image".to_string(),
        InputFormat::Epub => "EPUB E-book".to_string(),
        InputFormat::Fb2 => "FictionBook".to_string(),
        InputFormat::Mobi => "Kindle E-book".to_string(),
        InputFormat::Eml => "Email Message".to_string(),
        InputFormat::Mbox => "Unix Mailbox".to_string(),
        InputFormat::Msg => "Outlook Message".to_string(),
        InputFormat::Vcf => "vCard Contact".to_string(),
        InputFormat::Zip => "ZIP Archive".to_string(),
        InputFormat::Tar => "TAR Archive".to_string(),
        InputFormat::SevenZ => "7-Zip Archive".to_string(),
        InputFormat::Rar => "RAR Archive".to_string(),
        InputFormat::Srt => "SRT Subtitles".to_string(),
        InputFormat::Webvtt => "WebVTT Subtitles".to_string(),
        InputFormat::Odt => "OpenDocument Text".to_string(),
        InputFormat::Ods => "OpenDocument Spreadsheet".to_string(),
        InputFormat::Odp => "OpenDocument Presentation".to_string(),
        InputFormat::Tex => "LaTeX Document".to_string(),
        InputFormat::Xps => "XPS Document".to_string(),
        InputFormat::Rtf => "Rich Text Format".to_string(),
        InputFormat::Idml => "InDesign Markup".to_string(),
        InputFormat::Ipynb => "Jupyter Notebook".to_string(),
        InputFormat::Ics => "iCalendar".to_string(),
        InputFormat::Gpx => "GPS Exchange".to_string(),
        InputFormat::Kml => "Keyhole Markup".to_string(),
        InputFormat::Kmz => "Compressed KML".to_string(),
        InputFormat::Avif => "AVIF Image".to_string(),
        InputFormat::Heif => "HEIF Image".to_string(),
        InputFormat::Dicom => "DICOM Medical".to_string(),
        InputFormat::Dxf => "AutoCAD DXF".to_string(),
        InputFormat::Stl => "STL 3D Model".to_string(),
        InputFormat::Obj => "Wavefront OBJ".to_string(),
        InputFormat::Gltf => "glTF 3D Model".to_string(),
        InputFormat::Glb => "Binary glTF".to_string(),
        InputFormat::Vsdx => "Visio Diagram".to_string(),
        InputFormat::Mpp => "Microsoft Project".to_string(),
        InputFormat::One => "OneNote".to_string(),
        InputFormat::Pub => "Publisher".to_string(),
        InputFormat::Pages => "Apple Pages".to_string(),
        InputFormat::Numbers => "Apple Numbers".to_string(),
        InputFormat::Key => "Apple Keynote".to_string(),
        InputFormat::Mdb => "Access Database".to_string(),
        // Audio/video (not supported - handled by separate system)
        InputFormat::Wav => "WAV Audio".to_string(),
        InputFormat::Mp3 => "MP3 Audio".to_string(),
        InputFormat::Mp4 => "MP4 Video".to_string(),
        InputFormat::Mkv => "MKV Video".to_string(),
        InputFormat::Mov => "QuickTime Video".to_string(),
        InputFormat::Avi => "AVI Video".to_string(),
        // Internal format
        InputFormat::JsonDocling => "Docling JSON".to_string(),
    }
}

/// Get page count for supported formats
fn get_page_count(path: &Path, format: InputFormat) -> Option<usize> {
    match format {
        InputFormat::Pdf => {
            // Use lopdf to get page count (no external library needed)
            lopdf::Document::load(path)
                .ok()
                .map(|doc| doc.get_pages().len())
        }
        InputFormat::Epub => {
            // EPUB chapter count can be approximated
            None
        }
        _ => None,
    }
}

/// Perform deep analysis of document
fn perform_deep_analysis(path: &Path, format: InputFormat) -> Result<(DeepAnalysis, usize, usize)> {
    use docling_backend::RustDocumentConverter;

    // Only perform deep analysis on supported formats
    if !is_rust_backend_supported(format) {
        anyhow::bail!("Format not supported for deep analysis");
    }

    let converter = RustDocumentConverter::new()?;
    let result = converter.convert(path)?;

    let markdown = result.document.to_markdown();

    // Count words and characters
    let char_count = markdown.len();
    let word_count = markdown.split_whitespace().count();

    // Extract headings from markdown
    let headings: Vec<String> = markdown
        .lines()
        .filter(|line| line.starts_with('#'))
        .take(10) // Limit to first 10 headings
        .map(|line| line.trim_start_matches('#').trim().to_string())
        .collect();

    // Count structural elements (approximate from markdown)
    let tables = markdown.matches('|').count() / 4; // Rough estimate
    let images = markdown.matches("![").count();
    let links = markdown.matches("](").count().saturating_sub(images);
    let sections = headings.len();

    Ok((
        DeepAnalysis {
            sections,
            tables,
            images,
            links,
            headings,
        },
        word_count,
        char_count,
    ))
}

/// Print info in human-readable format
fn print_info_text(info: &DocumentInfo) {
    println!();
    println!(
        "{}  {}",
        "Document:".cyan().bold(),
        info.file_name.bright_white()
    );
    println!("{}", "─".repeat(50).bright_black());

    println!("  {:<16} {}", "Path:".bright_black(), info.file_path);
    println!(
        "  {:<16} {} ({})",
        "Size:".bright_black(),
        info.file_size_human,
        format!("{} bytes", info.file_size).bright_black()
    );
    println!(
        "  {:<16} {} {}",
        "Format:".bright_black(),
        info.format.cyan(),
        format!("({})", info.format_id).bright_black()
    );
    println!(
        "  {:<16} {}",
        "Supported:".bright_black(),
        if info.supported {
            "Yes".green()
        } else {
            "No".red()
        }
    );

    if let Some(pages) = info.page_count {
        println!(
            "  {:<16} {}",
            "Pages:".bright_black(),
            pages.to_string().yellow()
        );
    }

    if let Some(ref modified) = info.modified {
        println!("  {:<16} {}", "Modified:".bright_black(), modified);
    }

    if let Some(ref created) = info.created {
        println!("  {:<16} {}", "Created:".bright_black(), created);
    }

    // Deep analysis results
    if let Some(ref deep) = info.deep_analysis {
        println!();
        println!("{}", "Deep Analysis:".cyan().bold());
        println!("{}", "─".repeat(50).bright_black());

        if let Some(words) = info.word_count {
            println!(
                "  {:<16} {}",
                "Words:".bright_black(),
                words.to_string().yellow()
            );
        }
        if let Some(chars) = info.char_count {
            println!(
                "  {:<16} {}",
                "Characters:".bright_black(),
                chars.to_string().yellow()
            );
        }
        println!(
            "  {:<16} {}",
            "Sections:".bright_black(),
            deep.sections.to_string().yellow()
        );
        println!(
            "  {:<16} {}",
            "Tables:".bright_black(),
            deep.tables.to_string().yellow()
        );
        println!(
            "  {:<16} {}",
            "Images:".bright_black(),
            deep.images.to_string().yellow()
        );
        println!(
            "  {:<16} {}",
            "Links:".bright_black(),
            deep.links.to_string().yellow()
        );

        if !deep.headings.is_empty() {
            println!();
            println!("  {}:", "Headings".bright_black());
            for (i, heading) in deep.headings.iter().enumerate() {
                println!("    {} {}", format!("{}.", i + 1).bright_black(), heading);
            }
        }
    }

    println!();
}

/// Handle config subcommands
fn config_command(action: ConfigAction, verbosity: Verbosity) -> Result<()> {
    match action {
        ConfigAction::Init { global, force } => config_init(global, force, verbosity),
        ConfigAction::Show { json } => config_show(json),
        ConfigAction::Get { key } => config_get(&key),
        ConfigAction::Set { key, value, global } => config_set(&key, &value, global, verbosity),
        ConfigAction::Reset { global, yes } => config_reset(global, yes, verbosity),
        ConfigAction::Path { all } => config_path(all),
    }
}

/// Create a new configuration file with sensible defaults
fn config_init(global: bool, force: bool, verbosity: Verbosity) -> Result<()> {
    let config_path = if global {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .join(".docling.toml")
    } else {
        PathBuf::from(".docling.toml")
    };

    if config_path.exists() && !force {
        eprintln!(
            "{} Configuration file already exists: {}",
            "Error:".red().bold(),
            config_path.display()
        );
        eprintln!("{} Use --force to overwrite", "Hint:".cyan().bold());
        std::process::exit(1);
    }

    let default_config = r#"# Docling Configuration File
# See: https://github.com/dropbox/dKNOW/docling_rs

# Default settings for the convert command
[convert]
# Output format: markdown, json, or yaml
# format = "markdown"

# Backend: rust or auto
# backend = "auto"

# Enable compact JSON output (no pretty-printing)
# compact = false

# Enable OCR for scanned PDFs
# ocr = false

# Default settings for the batch command
[batch]
# Output format: markdown, json, or yaml
# format = "markdown"

# Continue processing on errors
# continue_on_error = false

# Maximum file size in bytes (skip larger files)
# max_file_size = 104857600  # 100MB

# Enable OCR for scanned PDFs
# ocr = false

# Compact JSON output
# compact = false

# Default settings for the benchmark command
[benchmark]
# Number of iterations
# iterations = 3

# Warmup iterations (discarded)
# warmup = 1

# Output format: text, json, or csv
# format = "text"

# Enable OCR
# ocr = false
"#;

    fs::write(&config_path, default_config)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    if verbosity.should_show_output() {
        println!(
            "{} Created configuration file: {}",
            "Success:".green().bold(),
            config_path.display()
        );
    }

    Ok(())
}

/// Display the current effective configuration
fn config_show(json_output: bool) -> Result<()> {
    let (user_config, project_config) = Config::discover_configs();
    let merged = Config::merge(user_config, project_config);

    if json_output {
        let json = serde_json::to_string_pretty(&merged)?;
        println!("{json}");
    } else {
        let toml = toml::to_string_pretty(&merged)?;
        println!("{toml}");
    }

    Ok(())
}

/// Get a specific configuration value
#[allow(
    clippy::unnecessary_wraps,
    reason = "consistent return type for CLI commands"
)]
fn config_get(key: &str) -> Result<()> {
    let (user_config, project_config) = Config::discover_configs();
    let merged = Config::merge(user_config, project_config);

    // Parse the key path (e.g., "convert.format")
    let parts: Vec<&str> = key.split('.').collect();

    let value: Option<String> = match parts.as_slice() {
        ["convert", "format"] => merged.convert.as_ref().and_then(|c| c.format.clone()),
        ["convert", "backend"] => merged.convert.as_ref().and_then(|c| c.backend.clone()),
        ["convert", "compact"] => merged
            .convert
            .as_ref()
            .and_then(|c| c.compact)
            .map(|v| v.to_string()),
        ["convert", "ocr"] => merged
            .convert
            .as_ref()
            .and_then(|c| c.ocr)
            .map(|v| v.to_string()),
        ["batch", "format"] => merged.batch.as_ref().and_then(|c| c.format.clone()),
        ["batch", "continue_on_error"] => merged
            .batch
            .as_ref()
            .and_then(|c| c.continue_on_error)
            .map(|v| v.to_string()),
        ["batch", "max_file_size"] => merged
            .batch
            .as_ref()
            .and_then(|c| c.max_file_size)
            .map(|v| v.to_string()),
        ["batch", "ocr"] => merged
            .batch
            .as_ref()
            .and_then(|c| c.ocr)
            .map(|v| v.to_string()),
        ["batch", "compact"] => merged
            .batch
            .as_ref()
            .and_then(|c| c.compact)
            .map(|v| v.to_string()),
        ["benchmark", "iterations"] => merged
            .benchmark
            .as_ref()
            .and_then(|c| c.iterations)
            .map(|v| v.to_string()),
        ["benchmark", "warmup"] => merged
            .benchmark
            .as_ref()
            .and_then(|c| c.warmup)
            .map(|v| v.to_string()),
        ["benchmark", "format"] => merged.benchmark.as_ref().and_then(|c| c.format.clone()),
        ["benchmark", "ocr"] => merged
            .benchmark
            .as_ref()
            .and_then(|c| c.ocr)
            .map(|v| v.to_string()),
        _ => {
            eprintln!(
                "{} Unknown configuration key: {}",
                "Error:".red().bold(),
                key
            );
            eprintln!();
            eprintln!("{} Valid keys:", "Help:".cyan().bold());
            eprintln!("  convert.format, convert.backend, convert.compact, convert.ocr");
            eprintln!("  batch.format, batch.continue_on_error, batch.max_file_size, batch.ocr, batch.compact");
            eprintln!("  benchmark.iterations, benchmark.warmup, benchmark.format, benchmark.ocr");
            std::process::exit(1);
        }
    };

    let Some(v) = value else {
        eprintln!("{} Key '{}' is not set", "Note:".yellow().bold(), key);
        std::process::exit(1);
    };
    println!("{v}");

    Ok(())
}

/// Set a configuration value
fn config_set(key: &str, value: &str, global: bool, verbosity: Verbosity) -> Result<()> {
    let config_path = if global {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .join(".docling.toml")
    } else {
        PathBuf::from(".docling.toml")
    };

    // Load existing config or create new
    let mut config: Config = if config_path.exists() {
        Config::load_from_file(&config_path)?
    } else {
        Config::default()
    };

    // Parse the key path and set the value
    let parts: Vec<&str> = key.split('.').collect();

    match parts.as_slice() {
        ["convert", "format"] => {
            let c = config.convert.get_or_insert_with(ConvertConfig::default);
            c.format = Some(value.to_string());
        }
        ["convert", "backend"] => {
            let c = config.convert.get_or_insert_with(ConvertConfig::default);
            c.backend = Some(value.to_string());
        }
        ["convert", "compact"] => {
            let c = config.convert.get_or_insert_with(ConvertConfig::default);
            c.compact = Some(
                value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Expected true or false"))?,
            );
        }
        ["convert", "ocr"] => {
            let c = config.convert.get_or_insert_with(ConvertConfig::default);
            c.ocr = Some(
                value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Expected true or false"))?,
            );
        }
        ["batch", "format"] => {
            let c = config.batch.get_or_insert_with(BatchConfig::default);
            c.format = Some(value.to_string());
        }
        ["batch", "continue_on_error"] => {
            let c = config.batch.get_or_insert_with(BatchConfig::default);
            c.continue_on_error = Some(
                value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Expected true or false"))?,
            );
        }
        ["batch", "max_file_size"] => {
            let c = config.batch.get_or_insert_with(BatchConfig::default);
            c.max_file_size = Some(parse_file_size(value).map_err(|e| anyhow::anyhow!("{e}"))?);
        }
        ["batch", "ocr"] => {
            let c = config.batch.get_or_insert_with(BatchConfig::default);
            c.ocr = Some(
                value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Expected true or false"))?,
            );
        }
        ["batch", "compact"] => {
            let c = config.batch.get_or_insert_with(BatchConfig::default);
            c.compact = Some(
                value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Expected true or false"))?,
            );
        }
        ["benchmark", "iterations"] => {
            let c = config
                .benchmark
                .get_or_insert_with(BenchmarkConfigSettings::default);
            c.iterations = Some(
                value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Expected a number"))?,
            );
        }
        ["benchmark", "warmup"] => {
            let c = config
                .benchmark
                .get_or_insert_with(BenchmarkConfigSettings::default);
            c.warmup = Some(
                value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Expected a number"))?,
            );
        }
        ["benchmark", "format"] => {
            let c = config
                .benchmark
                .get_or_insert_with(BenchmarkConfigSettings::default);
            c.format = Some(value.to_string());
        }
        ["benchmark", "ocr"] => {
            let c = config
                .benchmark
                .get_or_insert_with(BenchmarkConfigSettings::default);
            c.ocr = Some(
                value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Expected true or false"))?,
            );
        }
        _ => {
            eprintln!(
                "{} Unknown configuration key: {}",
                "Error:".red().bold(),
                key
            );
            eprintln!();
            eprintln!("{} Valid keys:", "Help:".cyan().bold());
            eprintln!("  convert.format, convert.backend, convert.compact, convert.ocr");
            eprintln!("  batch.format, batch.continue_on_error, batch.max_file_size, batch.ocr, batch.compact");
            eprintln!("  benchmark.iterations, benchmark.warmup, benchmark.format, benchmark.ocr");
            std::process::exit(1);
        }
    }

    // Write the updated config
    let toml = toml::to_string_pretty(&config)?;
    fs::write(&config_path, toml)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    if verbosity.should_show_output() {
        println!(
            "{} Set {} = {} in {}",
            "Success:".green().bold(),
            key.cyan(),
            value.yellow(),
            config_path.display()
        );
    }

    Ok(())
}

/// Reset configuration to defaults
fn config_reset(global: bool, yes: bool, verbosity: Verbosity) -> Result<()> {
    let config_path = if global {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .join(".docling.toml")
    } else {
        PathBuf::from(".docling.toml")
    };

    if !config_path.exists() {
        if verbosity.should_show_output() {
            println!(
                "{} No configuration file exists at: {}",
                "Note:".yellow().bold(),
                config_path.display()
            );
        }
        return Ok(());
    }

    // Confirm unless --yes is specified
    if !yes {
        eprintln!(
            "{} This will delete: {}",
            "Warning:".yellow().bold(),
            config_path.display()
        );
        eprint!("Continue? [y/N] ");

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("{} Aborted", "Note:".cyan().bold());
            return Ok(());
        }
    }

    fs::remove_file(&config_path)
        .with_context(|| format!("Failed to delete config file: {}", config_path.display()))?;

    if verbosity.should_show_output() {
        println!(
            "{} Deleted configuration file: {}",
            "Success:".green().bold(),
            config_path.display()
        );
    }

    Ok(())
}

/// Show configuration file paths
#[allow(
    clippy::unnecessary_wraps,
    reason = "consistent return type for CLI commands"
)]
fn config_path(all: bool) -> Result<()> {
    let home_config = dirs::home_dir().map(|h| h.join(".docling.toml"));
    let project_config = PathBuf::from(".docling.toml");

    if all {
        // Show all paths with existence status
        println!("{}", "Configuration file paths:".bold());
        println!();

        if let Some(ref home) = home_config {
            let status = if home.exists() {
                "exists".green()
            } else {
                "not found".yellow()
            };
            println!(
                "  {} {} ({})",
                "User:".bright_black(),
                home.display(),
                status
            );
        }

        let status = if project_config.exists() {
            "exists".green()
        } else {
            "not found".yellow()
        };
        println!(
            "  {} {} ({})",
            "Project:".bright_black(),
            project_config.display(),
            status
        );
    } else {
        // Show the effective config path (project if exists, else user, else default project)
        if project_config.exists() {
            println!("{}", project_config.display());
        } else if let Some(ref home) = home_config {
            if home.exists() {
                println!("{}", home.display());
            } else {
                // No config exists, show where project config would be created
                println!("{}", project_config.display());
            }
        } else {
            println!("{}", project_config.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_size_plain_numbers() {
        assert_eq!(parse_file_size("0").unwrap(), 0);
        assert_eq!(parse_file_size("1024").unwrap(), 1024);
        assert_eq!(parse_file_size("10485760").unwrap(), 10_485_760);
    }

    #[test]
    fn test_parse_file_size_kb_suffix() {
        assert_eq!(parse_file_size("1K").unwrap(), 1024);
        assert_eq!(parse_file_size("1k").unwrap(), 1024);
        assert_eq!(parse_file_size("1KB").unwrap(), 1024);
        assert_eq!(parse_file_size("1kb").unwrap(), 1024);
        assert_eq!(parse_file_size("100K").unwrap(), 102400);
    }

    #[test]
    fn test_parse_file_size_mb_suffix() {
        assert_eq!(parse_file_size("1M").unwrap(), 1048576);
        assert_eq!(parse_file_size("1m").unwrap(), 1048576);
        assert_eq!(parse_file_size("1MB").unwrap(), 1048576);
        assert_eq!(parse_file_size("1mb").unwrap(), 1048576);
        assert_eq!(parse_file_size("10M").unwrap(), 10485760);
    }

    #[test]
    fn test_parse_file_size_gb_suffix() {
        assert_eq!(parse_file_size("1G").unwrap(), 1073741824);
        assert_eq!(parse_file_size("1g").unwrap(), 1073741824);
        assert_eq!(parse_file_size("1GB").unwrap(), 1073741824);
        assert_eq!(parse_file_size("1gb").unwrap(), 1073741824);
    }

    #[test]
    fn test_parse_file_size_decimal() {
        assert_eq!(parse_file_size("1.5M").unwrap(), 1572864);
        assert_eq!(parse_file_size("1.5MB").unwrap(), 1572864);
        assert_eq!(parse_file_size("0.5G").unwrap(), 536870912);
        assert_eq!(parse_file_size("2.5K").unwrap(), 2560);
    }

    #[test]
    fn test_parse_file_size_whitespace() {
        assert_eq!(parse_file_size("  10M  ").unwrap(), 10485760);
        assert_eq!(parse_file_size(" 1024 ").unwrap(), 1024);
    }

    #[test]
    fn test_parse_file_size_bytes_suffix() {
        assert_eq!(parse_file_size("100B").unwrap(), 100);
        assert_eq!(parse_file_size("100b").unwrap(), 100);
    }

    #[test]
    fn test_parse_file_size_errors() {
        assert!(parse_file_size("").is_err());
        assert!(parse_file_size("   ").is_err());
        assert!(parse_file_size("abc").is_err());
        assert!(parse_file_size("M").is_err());
        assert!(parse_file_size("-10M").is_err());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 bytes");
        assert_eq!(format_bytes(512), "512 bytes");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1572864), "1.5 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_smart_output_path_markdown() {
        let input = PathBuf::from("/path/to/report.pdf");
        let result = smart_output_path(&input, &OutputFormat::Markdown);
        assert_eq!(result, PathBuf::from("/path/to/report.md"));
    }

    #[test]
    fn test_smart_output_path_json() {
        let input = PathBuf::from("/path/to/report.pdf");
        let result = smart_output_path(&input, &OutputFormat::Json);
        assert_eq!(result, PathBuf::from("/path/to/report.json"));
    }

    #[test]
    fn test_smart_output_path_yaml() {
        let input = PathBuf::from("/path/to/report.pdf");
        let result = smart_output_path(&input, &OutputFormat::Yaml);
        assert_eq!(result, PathBuf::from("/path/to/report.yaml"));
    }

    #[test]
    fn test_smart_output_path_relative() {
        let input = PathBuf::from("document.docx");
        let result = smart_output_path(&input, &OutputFormat::Markdown);
        assert_eq!(result, PathBuf::from("document.md"));
    }

    #[test]
    fn test_smart_output_path_nested() {
        let input = PathBuf::from("docs/subdir/file.html");
        let result = smart_output_path(&input, &OutputFormat::Json);
        assert_eq!(result, PathBuf::from("docs/subdir/file.json"));
    }
}
