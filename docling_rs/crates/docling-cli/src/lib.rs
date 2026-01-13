//! Command-line interface for `docling_rs` document conversion
//!
//! This crate provides the `docling` command-line tool for converting documents
//! between various formats. It supports 60+ document formats with high-fidelity
//! conversion to Markdown, JSON, or YAML.
//!
//! # Installation
//!
//! ```bash
//! # From source
//! cargo install --path crates/docling-cli
//!
//! # Or build the binary
//! cargo build --release -p docling-cli
//! ```
//!
//! # Quick Start
//!
//! ```bash
//! # Convert a PDF to Markdown
//! docling convert document.pdf
//!
//! # Convert with specific output format
//! docling convert document.pdf -f json -o output.json
//!
//! # Batch convert multiple files
//! docling batch *.pdf --output-dir converted/
//!
//! # Get document info without converting
//! docling info document.pdf
//! ```
//!
//! # Commands
//!
//! ## `convert` - Single File Conversion
//!
//! Convert a document to Markdown, JSON, or YAML:
//!
//! ```bash
//! # Basic conversion (outputs to stdout)
//! docling convert report.pdf
//!
//! # Save to file (smart output: report.pdf → report.md)
//! docling convert report.pdf -o auto
//!
//! # Specify output file
//! docling convert report.pdf -o report.md
//!
//! # Change output format
//! docling convert report.pdf -f json -o report.json
//!
//! # Limit pages (PDF only)
//! docling convert large.pdf --max-pages 10
//!
//! # Force overwrite existing files
//! docling convert report.pdf -o report.md --force
//!
//! # Dry run (show what would happen)
//! docling convert report.pdf -o report.md --dry-run
//! ```
//!
//! ## `batch` - Batch Conversion
//!
//! Convert multiple files at once:
//!
//! ```bash
//! # Convert all PDFs in current directory
//! docling batch *.pdf
//!
//! # Convert to specific output directory
//! docling batch documents/*.docx --output-dir converted/
//!
//! # Parallel processing (faster for many files)
//! docling batch *.pdf --parallel
//!
//! # Read file list from stdin
//! find . -name "*.pdf" | docling batch --stdin
//!
//! # Skip files larger than 10MB
//! docling batch *.pdf --max-size 10M
//! ```
//!
//! ## `info` - Document Information
//!
//! Display metadata about a document:
//!
//! ```bash
//! # Basic info
//! docling info document.pdf
//!
//! # Verbose output (includes full metadata)
//! docling info document.pdf --verbose
//! ```
//!
//! ## `formats` - Supported Formats
//!
//! List all supported input formats:
//!
//! ```bash
//! docling formats
//! ```
//!
//! ## `benchmark` - Performance Testing
//!
//! Benchmark conversion performance:
//!
//! ```bash
//! # Benchmark a single file
//! docling benchmark document.pdf
//!
//! # Benchmark with multiple iterations
//! docling benchmark document.pdf --iterations 10
//!
//! # Output results to file
//! docling benchmark document.pdf -o results.json
//! ```
//!
//! ## `config` - Configuration Management
//!
//! Manage docling configuration:
//!
//! ```bash
//! # Initialize default config
//! docling config init
//!
//! # Show current config
//! docling config show
//!
//! # Get a specific setting
//! docling config get output.format
//!
//! # Set a value
//! docling config set output.format json
//! ```
//!
//! ## `completions` - Shell Completions
//!
//! Generate shell completion scripts:
//!
//! ```bash
//! # Bash
//! docling completions bash > ~/.bash_completion.d/docling
//!
//! # Zsh
//! docling completions zsh > ~/.zfunc/_docling
//!
//! # Fish
//! docling completions fish > ~/.config/fish/completions/docling.fish
//! ```
//!
//! # Global Options
//!
//! - `-q, --quiet` - Suppress all output except errors
//! - `-v, --verbose` - Enable verbose output
//! - `-h, --help` - Show help information
//! - `-V, --version` - Show version information
//!
//! # Configuration
//!
//! The CLI can be configured via:
//!
//! 1. **Project config**: `.docling.toml` in current or parent directories
//! 2. **User config**: `~/.docling.toml`
//! 3. **Command-line flags**: Override any config setting
//!
//! Example `.docling.toml`:
//!
//! ```toml
//! [output]
//! format = "markdown"
//! directory = "./converted"
//!
//! [pdf]
//! max_pages = 100
//! ocr = true
//!
//! [batch]
//! parallel = true
//! max_size = "50M"
//! ```
//!
//! # Profiles
//!
//! Use `--profile` to apply preset configurations:
//!
//! ```bash
//! # Use 'fast' profile (skip OCR, limit pages)
//! docling convert document.pdf --profile fast
//!
//! # Use 'quality' profile (full OCR, all pages)
//! docling convert document.pdf --profile quality
//! ```
//!
//! # Watch Mode
//!
//! Automatically reconvert files when they change:
//!
//! ```bash
//! docling convert document.pdf -o document.md --watch
//! ```
//!
//! # Exit Codes
//!
//! - `0` - Success
//! - `1` - General error
//! - `2` - Invalid arguments
//! - `3` - File not found
//! - `4` - Unsupported format
//! - `5` - Conversion error
//!
//! # Library Usage
//!
//! This crate also exports shared CLI utilities:
//!
//! ```rust,ignore
//! use docling_cli::placeholder;
//!
//! // Currently a placeholder - main functionality is in main.rs
//! ```

/// Placeholder function to prevent empty library warnings.
///
/// The main CLI functionality is in `main.rs`. This lib module exists
/// for potential future library use of CLI components.
pub const fn placeholder() {}
