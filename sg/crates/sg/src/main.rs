//! sg - SuperGrep CLI
//!
//! Semantic code search that understands meaning, not just text.
//!
//! Usage:
//!   sg "query"                Search for code matching query
//!   sg status                 Show index status
//!   sg index \[path\]           Index a directory
//!   sg daemon start           Start the daemon
//!   sg daemon stop            Stop the daemon
//!   sg daemon status          Show daemon status

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use csv::{ReaderBuilder, WriterBuilder};
use directories::ProjectDirs;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sg_core::{
    compute_adaptive_cluster_count, detect_optimal_model_with_stats,
    index_directory_with_options_backend, is_indexable_path, load_embedder_from_env,
    load_or_create_index, optimal_batch_size, search_backend, search_clustered_backend,
    BackendEmbedder, CompactionStats, ContentType, EmbedderBackend, EmbeddingModel,
    IndexDirectoryOptions, IndexHealthMetrics, SearchOptions, DB, DEFAULT_CROSSFILE_BATCH_SIZE,
};
use sg_daemon::{
    config::{default_config_path, load_config},
    default_pid_path, default_socket_path, find_project_root, kill_daemon, read_daemon_pid, Client,
    ProjectStatus,
};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::Command;

mod eval;

/// Find the sg-daemon binary.
/// First looks in the same directory as the current executable,
/// then falls back to searching PATH.
fn find_daemon_binary() -> PathBuf {
    // Try to find sg-daemon next to the current executable
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let daemon_path = exe_dir.join("sg-daemon");
            if daemon_path.exists() {
                return daemon_path;
            }
            // Also try with .exe suffix on Windows
            #[cfg(windows)]
            {
                let daemon_path_exe = exe_dir.join("sg-daemon.exe");
                if daemon_path_exe.exists() {
                    return daemon_path_exe;
                }
            }
        }
    }
    // Fall back to PATH lookup
    PathBuf::from("sg-daemon")
}

fn resolve_path(path: Option<PathBuf>) -> Result<PathBuf> {
    let path = match path {
        Some(path) => path,
        None => std::env::current_dir().context("Failed to determine current directory")?,
    };
    // Always canonicalize to absolute path - the daemon runs with "/" as its working
    // directory, so relative paths would resolve incorrectly
    path.canonicalize()
        .with_context(|| format!("Failed to resolve path: {}", path.display()))
}

fn resolve_path_allow_missing(path: PathBuf) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .context("Failed to determine current directory")?
            .join(path)
    };

    if path.exists() {
        path.canonicalize()
            .with_context(|| format!("Failed to resolve path: {}", path.display()))
    } else {
        Ok(path)
    }
}

fn resolved_socket_path() -> PathBuf {
    let config_path = match default_config_path() {
        Ok(path) => path,
        Err(_) => return default_socket_path(),
    };

    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(_) => return default_socket_path(),
    };

    config
        .daemon_socket_path()
        .unwrap_or_else(default_socket_path)
}

fn load_embedder() -> Result<BackendEmbedder> {
    let mut embedder = load_embedder_from_env()?;
    // Warm up model to initialize GPU kernels
    embedder.warmup()?;
    Ok(embedder)
}

/// Load embedder with spinner feedback
fn load_embedder_with_spinner(show_spinner: bool) -> Result<BackendEmbedder> {
    let spinner = if show_spinner {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg} [{elapsed}]")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        sp.set_message("Loading embedding model...");
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(sp)
    } else {
        None
    };

    let mut embedder = load_embedder_from_env()?;

    // Warm up model to initialize GPU kernels (part of model loading)
    embedder.warmup()?;

    if let Some(sp) = spinner {
        sp.finish_with_message(format!(
            "Model loaded (backend={}, model={})",
            embedder.kind().as_str(),
            embedder.model().name()
        ));
    }

    Ok(embedder)
}

/// Load embedder for a specific model with spinner feedback
fn load_embedder_for_model(
    model: EmbeddingModel,
    model_path: Option<&std::path::Path>,
    show_spinner: bool,
) -> Result<BackendEmbedder> {
    let model_name = if let Some(p) = model_path {
        format!("custom ({})", p.display())
    } else {
        model.name().to_string()
    };

    let spinner = if show_spinner {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg} [{elapsed}]")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        sp.set_message(format!("Loading {model_name} model..."));
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(sp)
    } else {
        None
    };

    let mut embedder = if let Some(p) = model_path {
        BackendEmbedder::from_custom_path(p)?
    } else {
        BackendEmbedder::from_model(model)?
    };

    // Warm up model to initialize GPU kernels
    embedder.warmup()?;

    if let Some(sp) = spinner {
        sp.finish_with_message(format!(
            "Model loaded ({}, {} dims)",
            model_name,
            embedder.embedding_dim()
        ));
    }

    Ok(embedder)
}

/// Common search options shared between CLI and search subcommand
#[derive(Parser, Clone)]
struct SearchArgs {
    /// Number of results to return
    #[arg(short = 'n', long, default_value = "10")]
    limit: usize,

    /// Search directory (defaults to current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Disable hybrid search (use semantic-only)
    #[arg(long = "no-hybrid")]
    no_hybrid: bool,

    /// Auto-select semantic vs hybrid based on query style
    /// Docstring-style queries use semantic-only; natural language uses hybrid
    #[arg(long = "auto-hybrid")]
    auto_hybrid: bool,

    /// Rerank results using LLM for higher precision (requires ANTHROPIC_API_KEY)
    #[arg(long)]
    rerank: bool,

    /// Force direct mode (don't use daemon even if running)
    #[arg(long)]
    direct: bool,

    /// Output results as JSON
    #[arg(long)]
    json: bool,

    /// Number of context lines around matches
    #[arg(short = 'C', long, default_value = "2")]
    context: usize,

    /// Skip auto-indexing when searching an unindexed project
    #[arg(long)]
    no_auto_index: bool,

    /// Filter results by file type (e.g., rs, py, makefile). Can be specified multiple times.
    #[arg(short = 't', long = "type", value_name = "EXT")]
    file_types: Vec<String>,

    /// Exclude results by file type (e.g., test.rs, spec.js). Can be specified multiple times.
    #[arg(short = 'T', long = "exclude-type", value_name = "EXT")]
    exclude_file_types: Vec<String>,

    /// Disable query preprocessing for code-like searches
    #[arg(long = "no-preprocess-query")]
    no_preprocess_query: bool,

    /// Show detailed match information (raw scores)
    #[arg(short = 'v', long)]
    verbose: bool,

    /// Search images instead of text (requires --features clip)
    #[arg(long = "images")]
    images: bool,
}

impl From<&SearchArgs> for SearchCliOptions {
    fn from(args: &SearchArgs) -> Self {
        SearchCliOptions {
            limit: args.limit,
            hybrid: !args.no_hybrid,
            auto_hybrid: args.auto_hybrid,
            rerank: args.rerank,
            direct: args.direct,
            json: args.json,
            context: args.context,
            no_auto_index: args.no_auto_index,
            file_types: args.file_types.clone(),
            exclude_file_types: args.exclude_file_types.clone(),
            preprocess_query: !args.no_preprocess_query,
            verbose: args.verbose,
            images: args.images,
        }
    }
}

#[derive(Parser)]
#[command(name = "sg")]
#[command(about = "SuperGrep - semantic code search")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Search query (when no subcommand given)
    query: Option<String>,

    #[command(flatten)]
    search: SearchArgs,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for code (use this when your query matches a subcommand name)
    Search {
        /// Search query
        query: String,

        #[command(flatten)]
        args: SearchArgs,
    },
    /// Show index status
    Status {
        /// Output status as JSON
        #[arg(long)]
        json: bool,
    },
    /// Compact the index database (remove orphaned data and reclaim space)
    Compact {
        /// Output compaction results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Index a directory
    Index {
        /// Directory to index (defaults to current directory)
        path: Option<PathBuf>,
        /// Use pipelined architecture (overlaps file I/O with embedding for faster indexing)
        #[arg(long)]
        pipelined: bool,
        /// Use parallel file reading with rayon (parallelizes I/O and chunking across cores)
        #[arg(long)]
        parallel: bool,
        /// Cross-file embedding batch size (tune for GPU throughput vs. memory)
        /// Use "auto" for device-specific optimization, or a number >= 1
        #[arg(long, value_name = "N|auto")]
        batch_size: Option<String>,
        /// Embedding model to use (default: xtr)
        /// Available: xtr, unixcoder, jina-code, jina-colbert (onnx models require --features onnx)
        #[arg(
            long,
            value_name = "MODEL",
            conflicts_with = "auto_model",
            conflicts_with = "model_path"
        )]
        model: Option<String>,
        /// Path to a custom/fine-tuned model directory
        /// Must contain config.json, tokenizer.json, and model.safetensors
        #[arg(
            long,
            value_name = "PATH",
            conflicts_with = "model",
            conflicts_with = "auto_model"
        )]
        model_path: Option<PathBuf>,
        /// Automatically select the best embedding model based on corpus content
        /// Scans files and uses UniXcoder for code (>50% code files) or XTR for text
        #[arg(long)]
        auto_model: bool,
        /// Rebalance clusters without indexing new files
        /// Moves embeddings from overloaded clusters to underutilized ones
        #[arg(long)]
        rebalance: bool,
        /// Force re-indexing of all files, ignoring content hashes
        /// Useful when the embedding model has changed or embeddings are corrupted
        #[arg(long)]
        force: bool,
    },
    /// Daemon management commands
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Run evaluation against local corpora
    Eval {
        /// Evaluation spec files (defaults to eval/*_queries.json)
        #[arg(long, value_name = "PATH")]
        spec: Vec<PathBuf>,
        /// Print per-query hit/miss information
        #[arg(long)]
        verbose: bool,
        /// Embedding model to use (default: xtr)
        /// Available: xtr, unixcoder, jina-code, jina-colbert (onnx models require --features onnx)
        #[arg(long, value_name = "MODEL")]
        model: Option<String>,
        /// Path to custom model directory (e.g., fine-tuned model)
        #[arg(long, value_name = "PATH")]
        model_path: Option<PathBuf>,
        /// List available embedding models
        #[arg(long)]
        list_models: bool,
        /// Enable hybrid search (semantic + keyword with RRF fusion)
        #[arg(long)]
        hybrid: bool,
        /// Auto-select semantic vs hybrid based on query style
        #[arg(long)]
        auto_hybrid: bool,
        /// Rerank results using LLM (requires ANTHROPIC_API_KEY)
        #[arg(long)]
        rerank: bool,
    },
    /// Project management commands
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// Run performance benchmarks
    Benchmark {
        /// Directory to benchmark (defaults to current directory)
        path: Option<PathBuf>,
        /// Number of search iterations
        #[arg(short = 'n', long, default_value = "10")]
        iterations: usize,
    },
    /// Bulk search from CSV input (Phase 12.2)
    ///
    /// Reads queries from stdin or a file, executes searches, and outputs results as CSV.
    /// Input CSV format: `query[,path][,limit]`
    /// Output CSV format: `query,rank,path,score,line,snippet`
    Bulk {
        /// Input file (default: read from stdin)
        #[arg(short, long)]
        input: Option<PathBuf>,
        /// Output file (default: write to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Search directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Default number of results per query
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
        /// Input has no header row
        #[arg(long)]
        no_header: bool,
        /// Disable progress output
        #[arg(short, long)]
        quiet: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Generate shell initialization (cd hooks for auto-indexing)
    Init {
        /// Shell to generate init script for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Show embedding model information and recommendations
    Model {
        #[command(subcommand)]
        action: ModelAction,
    },
    /// List indexed files
    ///
    /// Shows all files in the index with their line and chunk counts.
    /// Optionally filter by path prefix.
    Files {
        /// Directory path to filter (show only files under this directory)
        path: Option<PathBuf>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Show only file count and totals (no individual files)
        #[arg(long)]
        summary: bool,
    },
    /// Show chunks for a file
    ///
    /// Displays chunk details for an indexed file including line ranges,
    /// header context, detected language, and content hash.
    Chunks {
        /// Path to the file to inspect
        path: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Show chunk content preview (first 100 chars)
        #[arg(long, short = 'c')]
        content: bool,
    },
    /// Index images using CLIP (requires --features clip)
    ///
    /// Recursively finds and indexes all images in a directory using CLIP embeddings.
    /// Indexed images can be searched with `sg --images "description"`.
    #[command(name = "index-images")]
    IndexImages {
        /// Directory containing images to index
        path: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// List known projects
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Run project discovery (scans common locations)
    Discover,
    /// Detect project root from path
    Detect {
        /// Path to detect project root from (defaults to current directory)
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the daemon
    Stop,
    /// Show daemon status
    Status {
        /// Output status as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ModelAction {
    /// List available embedding models
    List,
    /// Show detailed info about a specific model
    Info {
        /// Model name (xtr, unixcoder, jina-code, jina-colbert, clip)
        name: String,
    },
    /// Get recommended model for a query or file
    Recommend {
        /// Query or file path to analyze
        input: String,
        /// Treat input as file path instead of query
        #[arg(short, long)]
        file: bool,
    },
}

fn main() -> Result<()> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Search { query, args }) => {
            let path = resolve_path(args.path.clone())?;
            let opts = SearchCliOptions::from(&args);
            cmd_search(&query, &path, &opts)
        }
        Some(Commands::Status { json }) => cmd_status(json),
        Some(Commands::Compact { json }) => cmd_compact(json),
        Some(Commands::Index {
            path,
            pipelined,
            parallel,
            batch_size,
            model,
            model_path,
            auto_model,
            rebalance,
            force,
        }) => {
            // Parse batch_size: "auto", number, or None (defaults to static value)
            let batch_size_parsed = match batch_size.as_deref() {
                Some("auto") => BatchSize::Auto,
                Some(s) => {
                    let n: usize = s.parse().with_context(|| {
                        format!("invalid batch-size '{s}': expected 'auto' or a number")
                    })?;
                    if n == 0 {
                        anyhow::bail!("batch-size must be at least 1");
                    }
                    BatchSize::Fixed(n)
                }
                None => BatchSize::Default,
            };
            // Parse model selection (explicit --model takes precedence)
            let selected_model = match model {
                Some(ref name) => {
                    let m: EmbeddingModel = name.parse()?;
                    if !m.is_available() {
                        if m.requires_onnx() {
                            anyhow::bail!(
                                "Model '{}' requires ONNX support. Rebuild with --features onnx",
                                m.name()
                            );
                        }
                        let available: Vec<_> = EmbeddingModel::available()
                            .iter()
                            .map(|m| m.name().to_lowercase())
                            .collect();
                        anyhow::bail!(
                            "Model '{}' is not available. Available: {}",
                            m.name(),
                            available.join(", ")
                        );
                    }
                    Some(m)
                }
                None => None,
            };
            if rebalance {
                cmd_rebalance()
            } else {
                let path = resolve_path(path)?;
                cmd_index(
                    &path,
                    pipelined,
                    parallel,
                    batch_size_parsed,
                    selected_model,
                    model_path.as_deref(),
                    auto_model,
                    force,
                )
            }
        }
        Some(Commands::Daemon { action }) => match action {
            DaemonAction::Start { foreground } => cmd_daemon_start(foreground),
            DaemonAction::Stop => cmd_daemon_stop(),
            DaemonAction::Status { json } => cmd_daemon_status(json),
        },
        Some(Commands::Eval {
            spec,
            verbose,
            model,
            model_path,
            list_models,
            hybrid,
            auto_hybrid,
            rerank,
        }) => cmd_eval(spec, verbose, model, model_path, list_models, hybrid, auto_hybrid, rerank),
        Some(Commands::Project { action }) => match action {
            ProjectAction::List { json } => cmd_project_list(json),
            ProjectAction::Discover => cmd_project_discover(),
            ProjectAction::Detect { path } => {
                let path = resolve_path(path)?;
                cmd_project_detect(&path)
            }
        },
        Some(Commands::Benchmark { path, iterations }) => {
            let path = resolve_path(path)?;
            cmd_benchmark(&path, iterations)
        }
        Some(Commands::Bulk {
            input,
            output,
            path,
            limit,
            no_header,
            quiet,
        }) => {
            let path = resolve_path(path)?;
            cmd_bulk(input, output, &path, limit, no_header, quiet)
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "sg", &mut std::io::stdout());
            Ok(())
        }
        Some(Commands::Init { shell }) => {
            print!("{}", generate_init_script(shell));
            Ok(())
        }
        Some(Commands::Model { action }) => match action {
            ModelAction::List => cmd_model_list(),
            ModelAction::Info { name } => cmd_model_info(&name),
            ModelAction::Recommend { input, file } => cmd_model_recommend(&input, file),
        },
        Some(Commands::Files {
            path,
            json,
            summary,
        }) => {
            let path = path.map(resolve_path_allow_missing).transpose()?;
            cmd_files(path.as_deref(), json, summary)
        }
        Some(Commands::Chunks {
            path,
            json,
            content,
        }) => {
            let path = resolve_path_allow_missing(path)?;
            cmd_chunks(&path, json, content)
        }
        Some(Commands::IndexImages { path, json }) => {
            let path = resolve_path(Some(path))?;
            cmd_index_images(&path, json)
        }
        None => {
            if let Some(query) = cli.query {
                let path = resolve_path(cli.search.path.clone())?;
                let opts = SearchCliOptions::from(&cli.search);
                cmd_search(&query, &path, &opts)
            } else {
                // No query, show help
                println!("Usage: sg <query> or sg <command>");
                println!();
                println!("Commands:");
                println!("  sg \"query\"            Search for code matching query");
                println!("  sg search \"query\"     Explicit search (for queries matching command names)");
                println!("  sg status             Show index status");
                println!("  sg index              Index current directory");
                println!("  sg daemon start       Start the background daemon");
                println!("  sg daemon stop        Stop the background daemon");
                println!("  sg daemon status      Show daemon status");
                println!();
                println!("Run 'sg --help' for more options.");
                Ok(())
            }
        }
    }
}

/// Get the database path for this system
fn get_db_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "sg").context("Could not determine data directory")?;
    let data_dir = dirs.data_dir();
    Ok(data_dir.join("index.db"))
}

/// Collect all indexable files from a directory for auto-model detection.
///
/// This is a lightweight scan that just collects file paths without reading content.
/// Used by --auto-model to determine the optimal embedding model.
fn collect_indexable_files(path: &std::path::Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_indexable_files_recursive(path, &mut files)?;
    Ok(files)
}

fn collect_indexable_files_recursive(
    path: &std::path::Path,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    use std::fs;

    if path.is_file() {
        if is_indexable_path(path) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    if !path.is_dir() {
        return Ok(());
    }

    // Skip hidden directories and common non-source directories
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') {
            return Ok(());
        }
        if matches!(
            name,
            "node_modules" | "target" | "build" | "dist" | "__pycache__" | "vendor"
        ) {
            return Ok(());
        }
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            collect_indexable_files_recursive(&entry_path, files)?;
        } else if is_indexable_path(&entry_path) {
            files.push(entry_path);
        }
    }

    Ok(())
}

/// Index command implementation
enum IndexOutput {
    Human,
    Quiet,
}

/// Batch size configuration for indexing
#[derive(Debug, Clone, Copy)]
enum BatchSize {
    /// Use device-specific auto-tuning (calls optimal_batch_size)
    Auto,
    /// Use the static default (DEFAULT_CROSSFILE_BATCH_SIZE)
    Default,
    /// Use a fixed user-specified value
    Fixed(usize),
}

fn cmd_index(
    path: &std::path::Path,
    pipelined: bool,
    parallel: bool,
    batch_size: BatchSize,
    model: Option<EmbeddingModel>,
    model_path: Option<&std::path::Path>,
    auto_model: bool,
    force: bool,
) -> Result<()> {
    cmd_index_with_output(
        path,
        IndexOutput::Human,
        pipelined,
        parallel,
        batch_size,
        model,
        model_path,
        auto_model,
        force,
    )
}

fn cmd_index_with_output(
    path: &std::path::Path,
    output: IndexOutput,
    pipelined: bool,
    parallel: bool,
    batch_size: BatchSize,
    model: Option<EmbeddingModel>,
    model_path: Option<&std::path::Path>,
    auto_model: bool,
    force: bool,
) -> Result<()> {
    let db_path = get_db_path()?;
    if matches!(output, IndexOutput::Human) {
        let mode = match (pipelined, parallel, force) {
            (_, _, true) => " (force)",
            (true, true, false) => " (pipelined + parallel)",
            (true, false, false) => " (pipelined)",
            (false, true, false) => " (parallel)",
            (false, false, false) => "",
        };
        println!(
            "Indexing{} {} (database: {})",
            mode,
            path.display().to_string().cyan(),
            db_path.display()
        );
        if force {
            println!(
                "  {} - re-embedding all files regardless of content hashes",
                "Force mode".yellow()
            );
        }
    }

    // Determine which model to use
    let selected_model = if let Some(m) = model {
        // Explicit model selection takes precedence
        m
    } else if auto_model {
        // Auto-detect optimal model based on corpus content
        if matches!(output, IndexOutput::Human) {
            println!("  Scanning files for auto-model selection...");
        }
        let files = collect_indexable_files(path)?;
        let detection = detect_optimal_model_with_stats(&files);

        if matches!(output, IndexOutput::Human) {
            println!(
                "  {} files scanned: {} code, {} text ({:.0}% code)",
                detection.total_files,
                detection.code_files.to_string().cyan(),
                detection.text_files,
                detection.code_ratio * 100.0
            );
            println!(
                "  Auto-selected model: {} (best for {})",
                detection.model.name().green().bold(),
                if detection.code_ratio > 0.5 {
                    "code"
                } else {
                    "text"
                }
            );
        }
        detection.model
    } else {
        // Default to XTR
        EmbeddingModel::default()
    };

    // Open database
    let db = DB::new(&db_path)?;

    // Check if existing index uses a different model
    let existing_model = db
        .get_index_state("embedding_model")?
        .and_then(|s| s.parse::<EmbeddingModel>().ok());

    if let Some(existing) = existing_model {
        if existing != selected_model && !force {
            if matches!(output, IndexOutput::Human) {
                eprintln!(
                    "{} Existing index uses {} model, but {} was selected.",
                    "Warning:".yellow().bold(),
                    existing.name().cyan(),
                    selected_model.name().cyan()
                );
                eprintln!("  Different embedding models produce incompatible embeddings.");
                eprintln!(
                    "  Use {} to rebuild the index with the new model.",
                    "--force".green()
                );
            }
            anyhow::bail!(
                "Model mismatch: index uses {}, but {} was requested. Use --force to rebuild.",
                existing.name(),
                selected_model.name()
            );
        }
    }

    // Store the model being used
    db.set_index_state("embedding_model", selected_model.name())?;

    // Load embedder with spinner
    let show_spinner = matches!(output, IndexOutput::Human);
    let mut embedder = load_embedder_for_model(selected_model, model_path, show_spinner)?;

    // Resolve batch size based on config
    let resolved_batch_size = match batch_size {
        BatchSize::Auto => {
            let size = optimal_batch_size(&embedder.kind());
            if matches!(output, IndexOutput::Human) {
                println!(
                    "  Batch size: {} (auto-tuned for {})",
                    size.to_string().cyan(),
                    embedder.kind().as_str()
                );
            }
            size
        }
        BatchSize::Default => DEFAULT_CROSSFILE_BATCH_SIZE,
        BatchSize::Fixed(n) => n,
    };

    // Create progress bar
    let pb = match output {
        IndexOutput::Human => {
            let pb = ProgressBar::new(0);
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
            );
            pb
        }
        IndexOutput::Quiet => ProgressBar::hidden(),
    };

    // Index the directory
    // Note: parallel file reading requires pipelined mode
    let options = IndexDirectoryOptions {
        pipelined: pipelined || parallel, // parallel implies pipelined
        parallel_file_reading: parallel,
        crossfile_batch_size: resolved_batch_size,
        force,
        ..Default::default()
    };
    let stats = index_directory_with_options_backend(&db, &mut embedder, path, Some(&pb), options)?;
    pb.finish_and_clear();

    // Print results
    if matches!(output, IndexOutput::Human) {
        println!();
        println!("{}", "Indexing complete!".green().bold());
        println!(
            "  Files indexed: {}",
            stats.indexed_files.to_string().cyan()
        );
        println!(
            "  Files skipped: {}",
            stats.skipped_files.to_string().yellow()
        );
        if stats.failed_files > 0 {
            println!("  Files failed:  {}", stats.failed_files.to_string().red());
        }
        println!("  Total lines:   {}", stats.total_lines.to_string().cyan());
    }

    // Notify daemon to register the project if daemon is running
    let client = Client::new(&resolved_socket_path());
    if client.is_daemon_running() {
        // Register the project with the daemon (detect root adds to project manager)
        if let Some(root) = find_project_root(path) {
            let _ = client.detect_root(&root);
            // Start watching the project
            if let Err(e) = client.watch(&root) {
                tracing::debug!("Could not start watching project: {}", e);
            } else if matches!(output, IndexOutput::Human) {
                println!(
                    "  {}",
                    "Project registered with daemon and watching enabled".dimmed()
                );
            }
        }
    }

    Ok(())
}

fn cmd_eval(
    specs: Vec<PathBuf>,
    verbose: bool,
    model: Option<String>,
    model_path: Option<PathBuf>,
    list_models: bool,
    hybrid: bool,
    auto_hybrid: bool,
    rerank: bool,
) -> Result<()> {
    // Handle --list-models flag
    if list_models {
        println!("Available embedding models:\n");
        for m in EmbeddingModel::all() {
            let status = if m.is_available() {
                "available"
            } else if m.requires_onnx() {
                "requires --features onnx"
            } else {
                "not implemented"
            };
            let multi = if m.is_multi_vector() {
                "multi-vector"
            } else {
                "single-vector"
            };
            println!(
                "  {} ({}, {} dim, {})",
                m.name().cyan(),
                status,
                m.embedding_dim(),
                multi
            );
            println!("    HuggingFace: {}", m.model_id());
            println!();
        }
        return Ok(());
    }

    // Parse model selection
    let selected_model = match model {
        Some(ref name) => {
            let m: EmbeddingModel = name.parse()?;
            if !m.is_available() {
                if m.requires_onnx() {
                    return Err(anyhow::anyhow!(
                        "Model '{}' requires ONNX support. Rebuild with --features onnx",
                        m.name()
                    ));
                }
                let available: Vec<_> = EmbeddingModel::available()
                    .iter()
                    .map(|m| m.name().to_lowercase())
                    .collect();
                return Err(anyhow::anyhow!(
                    "Model '{}' is not available. Available: {}. Use --list-models for details.",
                    m.name(),
                    available.join(", ")
                ));
            }
            m
        }
        None => EmbeddingModel::default(),
    };

    let spec_paths = if specs.is_empty() {
        eval::discover_spec_paths()?
    } else {
        specs
    };

    let spec_files = eval::load_specs(&spec_paths)?;
    if spec_files.is_empty() {
        return Err(anyhow::anyhow!("No evaluation specs found"));
    }

    let mode = if auto_hybrid {
        if rerank { "auto-hybrid + rerank" } else { "auto-hybrid" }
    } else if hybrid {
        if rerank { "hybrid + rerank" } else { "hybrid" }
    } else if rerank {
        "semantic + rerank"
    } else {
        "semantic-only"
    };
    let model_label = if let Some(ref p) = model_path {
        format!("{} (custom: {})", selected_model.name(), p.display())
    } else {
        selected_model.name().to_string()
    };
    println!(
        "Running evaluation with {} model ({}) on {} corpus specs...",
        model_label.cyan(),
        mode,
        spec_files.len()
    );
    let mut embedder = load_embedder_for_model(selected_model, model_path.as_deref(), true)?;

    for spec_file in &spec_files {
        println!();
        println!(
            "{} ({})",
            spec_file
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("eval"),
            spec_file
                .spec
                .description
                .as_deref()
                .unwrap_or("no description")
        );

        let summary = eval::run_eval_spec(spec_file, &mut embedder, verbose, hybrid, auto_hybrid, rerank)?;
        println!(
            "  P@1: {:.2} ({}/{})",
            summary.p_at_1, summary.hits_at_1, summary.total_queries
        );
        println!("  MRR: {:.2}", summary.mrr);
    }

    Ok(())
}

/// Model list command - show available embedding models
fn cmd_model_list() -> Result<()> {
    println!("{}", "Available Embedding Models".cyan().bold());
    println!("{}", "═".repeat(50).dimmed());
    println!();

    for m in EmbeddingModel::all() {
        let status = if m.is_available() {
            "available".green()
        } else if m.requires_onnx() {
            "requires --features onnx".yellow()
        } else {
            "not implemented".yellow()
        };
        let vector_type = if m.is_multi_vector() {
            "multi-vector"
        } else {
            "single-vector"
        };

        println!("{}", m.name().white().bold());
        println!("  Status:     {status}");
        println!("  Dimensions: {}", m.embedding_dim().to_string().cyan());
        println!("  Type:       {}", vector_type.cyan());
        println!("  Model ID:   {}", m.model_id().dimmed());
        println!();
    }

    println!("{}", "Recommendation".cyan().bold());
    println!("{}", "─".repeat(50).dimmed());
    println!(
        "  • Use {} for general text search (prose, documentation)",
        "XTR".cyan()
    );
    println!(
        "  • Use {} for code search (20% better P@1 on code)",
        "UniXcoder".cyan()
    );
    println!();
    println!("  Run 'sg model recommend <query>' to get a specific recommendation.");

    Ok(())
}

/// Model info command - show detailed info about a model
fn cmd_model_info(name: &str) -> Result<()> {
    let model: EmbeddingModel = name.parse()?;

    println!("{}", model.name().cyan().bold());
    println!("{}", "═".repeat(50).dimmed());
    println!();

    let status = if model.is_available() {
        "Available".green()
    } else if model.requires_onnx() {
        "Requires --features onnx".yellow()
    } else {
        "Not implemented".yellow()
    };

    println!("  Status:       {status}");
    println!("  HuggingFace:  {}", model.model_id().cyan());
    println!(
        "  Dimensions:   {} per {}",
        model.embedding_dim().to_string().cyan(),
        if model.is_multi_vector() {
            "token"
        } else {
            "document"
        }
    );
    println!();

    // Model-specific info
    match model {
        EmbeddingModel::Xtr => {
            println!("{}", "Architecture".white().bold());
            println!("  • T5 encoder + linear projection to 128 dims");
            println!("  • Multi-vector: one embedding per token");
            println!("  • Scoring: MaxSim (max similarity per query token)");
            println!();
            println!("{}", "Best For".white().bold());
            println!("  • General text retrieval");
            println!("  • Documentation search");
            println!("  • Natural language prose");
            println!();
            println!("{}", "Performance".white().bold());
            println!("  • Gutenberg (prose): P@1 = 0.90, MRR = 0.95");
            println!("  • Code: P@1 = 0.50, MRR = 0.69");
            println!("  • Multilingual: P@1 = 0.67, MRR = 0.83");
        }
        EmbeddingModel::UniXcoder => {
            println!("{}", "Architecture".white().bold());
            println!("  • RoBERTa-based encoder");
            println!("  • Single-vector: 768 dim CLS pooling");
            println!("  • Scoring: Cosine similarity");
            println!();
            println!("{}", "Best For".white().bold());
            println!("  • Code search");
            println!("  • Code-comment matching");
            println!("  • Programming language content");
            println!();
            println!("{}", "Performance".white().bold());
            println!("  • Code: P@1 = 0.60, MRR = 0.75 (20% better than XTR)");
            println!("  • Gutenberg (prose): P@1 = 0.10, MRR = 0.33 (poor on prose)");
        }
        EmbeddingModel::JinaCode => {
            println!("{}", "Note".yellow().bold());
            println!("  This model is not yet implemented.");
            println!();
            println!("{}", "Architecture".white().bold());
            println!("  • BERT-based with ALiBi positional encoding");
            println!("  • Single-vector: 768 dim mean pooling");
            println!("  • Supports up to 8192 tokens (long context)");
            println!();
            println!("{}", "Best For".white().bold());
            println!("  • Code retrieval");
            println!("  • Long code files");
        }
        // CLIP variant only exists when clip feature is enabled
        #[allow(unreachable_patterns)]
        _ => {
            // Handle CLIP (only reachable when clip feature is enabled)
            println!("{}", "Architecture".white().bold());
            println!("  • Vision Transformer (ViT-B/32) + Text Transformer");
            println!("  • Single-vector: 512 dim for both text and images");
            println!("  • Scoring: Cosine similarity (cross-modal)");
            println!();
            println!("{}", "Best For".white().bold());
            println!("  • Image search by text description");
            println!("  • Cross-modal retrieval (text <-> image)");
            println!("  • Zero-shot image classification");
            println!();
            println!("{}", "Note".yellow().bold());
            println!("  CLIP embeddings require a separate image index.");
            println!("  Cannot be mixed with text embeddings (different dimensions).");
        }
    }

    Ok(())
}

/// Model recommend command - recommend a model for a query or file
fn cmd_model_recommend(input: &str, is_file: bool) -> Result<()> {
    let content_type = if is_file {
        ContentType::from_path(std::path::Path::new(input))
    } else {
        ContentType::from_query(input)
    };

    let recommended = content_type.recommended_model();
    let input_desc = if is_file { "file" } else { "query" };

    println!("{}", "Model Recommendation".cyan().bold());
    println!("{}", "═".repeat(50).dimmed());
    println!();

    // Show the input
    println!("  {} {}:", input_desc.white().bold(), input.cyan());
    println!();

    // Show content type detection
    let type_name = match content_type {
        ContentType::Cjk => "CJK (Chinese/Japanese/Korean)",
        ContentType::Code => "Code",
        ContentType::Text => "Text",
    };
    println!("  Detected type: {}", type_name.cyan());
    println!();

    // Show recommendation
    println!(
        "  {} {}",
        "Recommended model:".white().bold(),
        recommended.name().green().bold()
    );
    println!();

    // Show reasoning
    match content_type {
        ContentType::Cjk => {
            println!("  {}", "Why Jina-ColBERT?".white().bold());
            println!("  • Input contains CJK (Chinese/Japanese/Korean) characters");
            println!("  • Jina-ColBERT-v2 achieves P@1=1.00 on Japanese (vs XTR 0.67)");
            println!("  • Native CJK tokenization eliminates need for hybrid search fallback");
            println!("  • Supports 94 languages with 8192 token context");
        }
        ContentType::Code => {
            println!("  {}", "Why UniXcoder?".white().bold());
            println!("  • Input appears to contain code patterns (camelCase, snake_case, etc.)");
            println!("  • UniXcoder achieves 20% better P@1 on code search");
            println!("  • Specialized for code-comment alignment");
        }
        ContentType::Text => {
            println!("  {}", "Why XTR?".white().bold());
            println!("  • Input appears to be natural language text");
            println!("  • XTR achieves 9x better results on prose (P@1 0.90 vs 0.10)");
            println!("  • Multi-vector approach captures more nuance");
        }
    }

    println!();
    println!(
        "  To use this model: {}",
        format!("sg eval --model {}", recommended.name().to_lowercase()).dimmed()
    );

    Ok(())
}

/// CLI options for search command
struct SearchCliOptions {
    limit: usize,
    hybrid: bool,
    auto_hybrid: bool,
    rerank: bool,
    direct: bool,
    json: bool,
    context: usize,
    no_auto_index: bool,
    file_types: Vec<String>,
    exclude_file_types: Vec<String>,
    preprocess_query: bool,
    verbose: bool,
    images: bool,
}

/// Match quality tier based on semantic similarity score
#[derive(Debug, Clone, Copy)]
enum MatchQuality {
    /// Score >= 0.8: Very strong semantic match
    Excellent,
    /// Score >= 0.6: Good semantic relevance
    Good,
    /// Score >= 0.4: Moderate relevance, may be tangentially related
    Fair,
    /// Score < 0.4: Weak match, possibly noise
    Relevant,
}

impl MatchQuality {
    fn from_score(score: f32) -> Self {
        if score >= 0.8 {
            MatchQuality::Excellent
        } else if score >= 0.6 {
            MatchQuality::Good
        } else if score >= 0.4 {
            MatchQuality::Fair
        } else {
            MatchQuality::Relevant
        }
    }

    fn label(&self) -> &'static str {
        match self {
            MatchQuality::Excellent => "Excellent",
            MatchQuality::Good => "Good",
            MatchQuality::Fair => "Fair",
            MatchQuality::Relevant => "Relevant",
        }
    }

    fn colored_label(&self) -> colored::ColoredString {
        match self {
            MatchQuality::Excellent => self.label().green().bold(),
            MatchQuality::Good => self.label().green(),
            MatchQuality::Fair => self.label().yellow(),
            MatchQuality::Relevant => self.label().dimmed(),
        }
    }
}

/// Search command implementation
fn cmd_search(query: &str, path: &std::path::Path, opts: &SearchCliOptions) -> Result<()> {
    // Validate query is not empty
    let query = query.trim();
    if query.is_empty() {
        if opts.json {
            println!(r#"{{"error": "Query cannot be empty"}}"#);
        } else {
            eprintln!(
                "{} Query cannot be empty. Usage: sg \"your search query\"",
                "Error:".red().bold()
            );
        }
        return Ok(());
    }

    // Handle image search if --images flag is set
    #[cfg(feature = "clip")]
    if opts.images {
        return cmd_search_images(query, path, opts);
    }

    #[cfg(not(feature = "clip"))]
    if opts.images {
        eprintln!(
            "{} Image search requires the 'clip' feature. Build with: cargo build --features clip",
            "Error:".red().bold()
        );
        return Ok(());
    }

    // Check if project needs indexing (auto-index unless disabled)
    if !opts.no_auto_index {
        let db_path = get_db_path()?;
        let project_root = find_project_root(path).unwrap_or_else(|| path.to_path_buf());

        // Check if project has any indexed documents
        let needs_indexing = if db_path.exists() {
            let db = DB::new(&db_path)?;
            let stats = db.project_stats(&project_root)?;
            stats.file_count == 0
        } else {
            true
        };

        if needs_indexing {
            if !opts.json {
                eprintln!(
                    "{} No index found for {}. Auto-indexing...",
                    "Note:".yellow().bold(),
                    project_root.display()
                );
            }
            // Index the project
            let output = if opts.json {
                IndexOutput::Quiet
            } else {
                IndexOutput::Human
            };
            cmd_index_with_output(
                &project_root,
                output,
                false,
                false,
                BatchSize::Default,
                None,
                None,  // model_path
                false, // auto_model - use default model for auto-indexing
                false, // Don't force re-index during auto-indexing
            )?;
        }
    }

    // Try daemon first if not in direct mode
    if !opts.direct {
        let client = Client::new(&resolved_socket_path());
        if client.is_daemon_running() {
            return cmd_search_via_daemon(&client, query, path, opts);
        }
    }

    // Fall back to direct mode
    cmd_search_direct(query, path, opts)
}

/// Search via the daemon
fn cmd_search_via_daemon(
    client: &Client,
    query: &str,
    path: &std::path::Path,
    opts: &SearchCliOptions,
) -> Result<()> {
    let options = SearchOptions {
        top_k: opts.limit,
        threshold: 0.0,
        hybrid: opts.hybrid,
        auto_hybrid: opts.auto_hybrid,
        root: Some(path.to_path_buf()),
        context: opts.context,
        file_types: opts.file_types.clone(),
        exclude_file_types: opts.exclude_file_types.clone(),
        preprocess_query: opts.preprocess_query,
    };

    let results = client.search(query, options)?;

    if opts.json {
        return print_results_json(&results, query);
    }

    print_results_human(&results, query, Some(path), opts.verbose)
}

/// Search directly (without daemon)
fn cmd_search_direct(query: &str, path: &std::path::Path, opts: &SearchCliOptions) -> Result<()> {
    let db_path = get_db_path()?;

    // Check if database exists
    if !db_path.exists() {
        if opts.json {
            println!(r#"{{"error": "No index found. Run 'sg index' first."}}"#);
        } else {
            eprintln!(
                "{} No index found. Run 'sg index' first.",
                "Error:".red().bold()
            );
        }
        return Ok(());
    }

    // Open database
    let db = DB::new(&db_path)?;

    // Check if we have any documents
    let doc_count = db.document_count()?;
    if doc_count == 0 {
        if opts.json {
            println!(r#"{{"error": "Index is empty. Run 'sg index' first."}}"#);
        } else {
            eprintln!(
                "{} Index is empty. Run 'sg index' first.",
                "Error:".red().bold()
            );
        }
        return Ok(());
    }

    // Detect which model the index was built with and load appropriate embedder
    let index_model = db
        .get_index_state("embedding_model")?
        .and_then(|s| s.parse::<EmbeddingModel>().ok())
        .unwrap_or(EmbeddingModel::default());

    // Load embedder with spinner (silent in JSON mode)
    let mut embedder = load_embedder_for_model(index_model, None, !opts.json)?;

    // Search options - request more candidates if reranking
    let search_limit = if opts.rerank { opts.limit.max(50) } else { opts.limit };
    let options = SearchOptions {
        top_k: search_limit,
        threshold: 0.0,
        hybrid: opts.hybrid,
        auto_hybrid: opts.auto_hybrid,
        root: Some(path.to_path_buf()),
        context: opts.context,
        file_types: opts.file_types.clone(),
        exclude_file_types: opts.exclude_file_types.clone(),
        preprocess_query: opts.preprocess_query,
    };

    // Execute search with spinner
    let spinner = if !opts.json {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg} [{elapsed}]")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        let msg = if opts.rerank {
            "Searching + reranking..."
        } else {
            "Searching..."
        };
        sp.set_message(msg);
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(sp)
    } else {
        None
    };

    // Use clustered search for large indices (>100 docs), brute-force for small
    // Note: Clustered search requires XTR model (128-dim multi-vector). Other models use brute-force.
    const CLUSTERED_THRESHOLD: u64 = 100;
    let doc_count = doc_count as u64;
    let use_clustered = doc_count > CLUSTERED_THRESHOLD && index_model == EmbeddingModel::Xtr;
    let mut results = if use_clustered {
        // Load or create LazyIndex for fast clustered search
        // Adaptive cluster count: k ≈ sqrt(n), clamped to [16, 256]
        let num_clusters = compute_adaptive_cluster_count(doc_count as usize);
        let (lazy_index, _from_cache) = load_or_create_index(&db, &db_path, num_clusters)?;
        search_clustered_backend(&db, &lazy_index, &mut embedder, query, options)?
    } else {
        // Brute-force for small indices or non-XTR models
        search_backend(&db, &mut embedder, query, options)?
    };

    // Apply LLM reranking if requested
    if opts.rerank && !results.is_empty() {
        match sg_core::LLMReranker::from_env() {
            Ok(reranker) => {
                use sg_core::{RerankOptions, Reranker};
                let rerank_options = RerankOptions {
                    candidates: results.len(),
                    top_k: opts.limit,
                    ..Default::default()
                };
                match reranker.rerank(query, results.clone(), &rerank_options) {
                    Ok(reranked) => results = reranked,
                    Err(e) => {
                        if !opts.json {
                            eprintln!(
                                "{} Reranking failed: {}",
                                "Warning:".yellow().bold(),
                                e
                            );
                        }
                        // Fall back to original results, truncated
                        results.truncate(opts.limit);
                    }
                }
            }
            Err(e) => {
                if !opts.json {
                    eprintln!(
                        "{} LLM reranker unavailable: {}",
                        "Warning:".yellow().bold(),
                        e
                    );
                    eprintln!("  Set ANTHROPIC_API_KEY to enable --rerank");
                }
                results.truncate(opts.limit);
            }
        }
    }

    if let Some(sp) = spinner {
        sp.finish_and_clear();
    }

    // Convert to daemon-style results for unified output
    let daemon_results: Vec<sg_daemon::SearchResultWire> = results
        .iter()
        .map(|r| sg_daemon::SearchResultWire {
            path: r.path.display().to_string(),
            score: r.score,
            line: r.line,
            snippet: r.snippet.clone(),
            header_context: r.header_context.clone(),
            language: r.language.clone(),
            links: r
                .links
                .iter()
                .map(|link| sg_daemon::SearchResultLinkWire {
                    text: link.text.clone(),
                    target: link.target.clone(),
                    is_internal: link.is_internal,
                })
                .collect(),
        })
        .collect();

    if opts.json {
        print_results_json(&daemon_results, query)
    } else {
        print_results_human(&daemon_results, query, Some(path), opts.verbose)
    }
}

/// Search images by text description using CLIP
#[cfg(feature = "clip")]
fn cmd_search_images(query: &str, path: &std::path::Path, opts: &SearchCliOptions) -> Result<()> {
    use sg_core::{search_images, ClipEmbedder, ImageSearchOptions};

    let db_path = get_db_path()?;

    // Check if database exists
    if !db_path.exists() {
        if opts.json {
            println!(r#"{{"error": "No index found. Run 'sg index' first."}}"#);
        } else {
            eprintln!(
                "{} No index found. Run 'sg index' first.",
                "Error:".red().bold()
            );
        }
        return Ok(());
    }

    // Open database
    let db = sg_core::DB::new(&db_path)?;

    // Check if we have any images
    let image_count = db.image_count()?;
    if image_count == 0 {
        if opts.json {
            println!(r#"{{"error": "No images indexed. Run 'sg index --images <dir>' first."}}"#);
        } else {
            eprintln!(
                "{} No images indexed. Run 'sg index-images <dir>' to index images.",
                "Error:".red().bold()
            );
        }
        return Ok(());
    }

    // Load CLIP embedder with spinner
    let spinner = if !opts.json {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg} [{elapsed}]")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        sp.set_message("Loading CLIP model...");
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(sp)
    } else {
        None
    };

    let device = sg_core::make_device();
    let mut clip_embedder = ClipEmbedder::new(&device)?;

    if let Some(sp) = &spinner {
        sp.set_message("Searching images...");
    }

    // Search options
    let options = ImageSearchOptions {
        max_results: opts.limit,
        min_score: 0.0,
    };

    // Execute image search
    let results = search_images(&db, &mut clip_embedder, query, options)?;

    if let Some(sp) = spinner {
        sp.finish_and_clear();
    }

    // Output results
    if opts.json {
        print_image_results_json(&results, query, path)
    } else {
        print_image_results_human(&results, query, path, opts.verbose)
    }
}

/// Print image search results as JSON
#[cfg(feature = "clip")]
fn print_image_results_json(
    results: &[sg_core::ImageSearchResult],
    query: &str,
    _root: &std::path::Path,
) -> Result<()> {
    #[derive(serde::Serialize)]
    struct JsonResult {
        path: String,
        score: f32,
    }

    #[derive(serde::Serialize)]
    struct JsonOutput<'a> {
        query: &'a str,
        count: usize,
        results: Vec<JsonResult>,
    }

    let json_results: Vec<JsonResult> = results
        .iter()
        .map(|r| JsonResult {
            path: r.path.clone(),
            score: r.score,
        })
        .collect();

    let output = JsonOutput {
        query,
        count: results.len(),
        results: json_results,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Print image search results in human-readable format
#[cfg(feature = "clip")]
fn print_image_results_human(
    results: &[sg_core::ImageSearchResult],
    query: &str,
    root: &std::path::Path,
    verbose: bool,
) -> Result<()> {
    if results.is_empty() {
        println!(
            "{} No images found for \"{}\"",
            "Note:".yellow().bold(),
            query
        );
        return Ok(());
    }

    println!(
        "\n{} {} for \"{}\":\n",
        "Images".green().bold(),
        results.len(),
        query.cyan()
    );

    for (i, result) in results.iter().enumerate() {
        let relative_path = std::path::Path::new(&result.path)
            .strip_prefix(root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| result.path.clone());

        let score_str = if verbose {
            format!(" (score: {:.3})", result.score)
        } else {
            String::new()
        };

        let quality = if result.score >= 0.3 {
            "Excellent".green()
        } else if result.score >= 0.25 {
            "Good".yellow()
        } else if result.score >= 0.2 {
            "Fair".white()
        } else {
            "Weak".dimmed()
        };

        println!(
            "{}. {} [{quality}]{score_str}",
            (i + 1).to_string().white().bold(),
            relative_path.cyan(),
        );
    }

    println!();
    Ok(())
}

/// Print search results as JSON
fn print_results_json(results: &[sg_daemon::SearchResultWire], query: &str) -> Result<()> {
    #[derive(serde::Serialize)]
    struct JsonOutput<'a> {
        query: &'a str,
        count: usize,
        results: &'a [sg_daemon::SearchResultWire],
    }

    let output = JsonOutput {
        query,
        count: results.len(),
        results,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Print search results in human-readable format
fn format_result_path(path: &str, root: Option<&std::path::Path>) -> String {
    let path_buf = std::path::Path::new(path);
    if let Some(root) = root {
        if let Ok(relative) = path_buf.strip_prefix(root) {
            if relative.as_os_str().is_empty() {
                if let Some(name) = path_buf.file_name() {
                    return name.to_string_lossy().to_string();
                }
            } else {
                return relative.to_string_lossy().to_string();
            }
        }
    }

    path.to_string()
}

fn format_result_links(links: &[sg_daemon::SearchResultLinkWire]) -> String {
    const MAX_LINKS: usize = 4;

    let mut parts = Vec::new();
    for link in links.iter().take(MAX_LINKS) {
        let text = link.text.trim();
        let target = link.target.trim();
        let label = if text.is_empty() || text == target {
            target.to_string()
        } else {
            format!("{text} -> {target}")
        };
        let kind = if link.is_internal {
            "internal"
        } else {
            "external"
        };
        parts.push(format!("{label} ({kind})"));
    }

    let remaining = links.len().saturating_sub(MAX_LINKS);
    if remaining > 0 {
        parts.push(format!("+{remaining} more"));
    }

    parts.join(" | ")
}

fn print_results_human(
    results: &[sg_daemon::SearchResultWire],
    query: &str,
    root: Option<&std::path::Path>,
    verbose: bool,
) -> Result<()> {
    if results.is_empty() {
        println!("No results found for \"{query}\"");
        return Ok(());
    }

    println!(
        "Found {} results for \"{}\":",
        results.len().to_string().cyan(),
        query.yellow()
    );
    println!();

    for (i, result) in results.iter().enumerate() {
        // Print file path with line number and quality indicator
        let display_path = format_result_path(&result.path, root);
        let location = if result.line > 0 {
            format!("{}:{}", display_path, result.line)
        } else {
            display_path
        };

        let quality = MatchQuality::from_score(result.score);

        if verbose {
            // Verbose mode: show quality label with percentage
            println!(
                "{} {} {} {}",
                format!("{}.", i + 1).dimmed(),
                location.cyan().bold(),
                quality.colored_label(),
                format!("({:.0}%)", result.score * 100.0).dimmed()
            );
        } else {
            // Normal mode: show quality label only
            println!(
                "{} {} {}",
                format!("{}.", i + 1).dimmed(),
                location.cyan().bold(),
                quality.colored_label()
            );
        }

        // Show header context and/or language if available
        let has_context = !result.header_context.is_empty();
        let has_language = result.language.is_some();

        if has_context || has_language {
            let mut context_parts = Vec::new();
            if has_context {
                context_parts.push(result.header_context.blue().to_string());
            }
            if let Some(ref lang) = result.language {
                context_parts.push(format!("[{lang}]").magenta().to_string());
            }
            println!("  {} {}", "Context:".dimmed(), context_parts.join(" "));
        }

        if verbose && !result.links.is_empty() {
            let formatted = format_result_links(&result.links);
            if !formatted.is_empty() {
                println!("  {} {}", "Links:".dimmed(), formatted.dimmed());
            }
        }

        // Print snippet with line numbers
        let base_line = if result.line > 0 { result.line } else { 1 };
        for (offset, line) in result.snippet.lines().enumerate() {
            let line_num = base_line + offset;
            println!(
                "  {} {}",
                format!("{line_num:>4} |").dimmed(),
                line.dimmed()
            );
        }
        println!();
    }

    // In verbose mode, show legend
    if verbose {
        println!("{}", "─".repeat(40).dimmed());
        println!("{}", "Match Quality Legend:".dimmed());
        println!(
            "  {} = 80%+ semantic similarity",
            "Excellent".green().bold()
        );
        println!("  {}      = 60-79% semantic similarity", "Good".green());
        println!("  {}      = 40-59% semantic similarity", "Fair".yellow());
        println!("  {}  = <40% semantic similarity", "Relevant".dimmed());
    }

    Ok(())
}

/// Status output for JSON serialization
#[derive(Serialize)]
struct StatusOutput {
    daemon: DaemonStatusOutput,
    index: IndexStatusOutput,
    paths: PathsOutput,
}

#[derive(Serialize)]
struct DaemonStatusOutput {
    running: bool,
    pid: Option<u32>,
    uptime_secs: Option<u64>,
    index_quality: Option<f32>,
    project_count: Option<usize>,
    throttle_state: Option<String>,
    projects: Option<Vec<ProjectStatus>>,
}

#[derive(Serialize)]
struct IndexStatusOutput {
    exists: bool,
    path: String,
    size_bytes: Option<u64>,
    document_count: Option<usize>,
    line_count: Option<usize>,
}

#[derive(Serialize)]
struct PathsOutput {
    socket: String,
    pid_file: String,
}

/// Status command implementation
fn cmd_status(json: bool) -> Result<()> {
    let db_path = get_db_path()?;
    let pid_path = default_pid_path();
    let socket_path = resolved_socket_path();

    if json {
        return cmd_status_json(&db_path, &pid_path, &socket_path);
    }

    // Print header
    println!("{}", "━".repeat(50).dimmed());
    println!("{}", "            SuperGrep Status".cyan().bold());
    println!("{}", "━".repeat(50).dimmed());
    println!();

    // Daemon section
    println!("{}", "Daemon".white().bold());
    if let Some(pid) = read_daemon_pid(&pid_path)? {
        println!("  Status:  {} (pid {})", "running".green(), pid);

        // Get more info from daemon if available
        let client = Client::new(&socket_path);
        if let Ok(status) = client.status() {
            let uptime = format_duration(status.uptime_secs);
            println!("  Uptime:  {}", uptime.cyan());
            println!(
                "  Quality: {}%",
                (status.index_quality * 100.0).round().to_string().cyan()
            );

            // Show project count
            if !status.projects.is_empty() {
                println!("  Projects: {}", status.projects.len().to_string().cyan());
            }

            // Show throttle state if available
            if !status.throttle_state.is_empty() {
                println!("  Indexing: {}", status.throttle_state.cyan());
            }
        }
    } else {
        println!("  Status:  {}", "not running".yellow());
        println!(
            "  {}",
            "Run 'sg daemon start' to enable background indexing".dimmed()
        );
    }
    println!();

    // Index section
    println!("{}", "Index".white().bold());
    if !db_path.exists() {
        println!("  Status: {}", "not created".yellow());
        println!("  {}", "Run 'sg index <path>' to create an index".dimmed());
        return Ok(());
    }

    // Get database file size
    let db_size = std::fs::metadata(&db_path)
        .map(|m| format_bytes(m.len()))
        .unwrap_or_else(|_| "unknown".to_string());

    // Open database
    let db = DB::new(&db_path)?;
    let doc_count = db.document_count()?;
    let line_count = db.total_lines()?;

    println!("  Path:      {}", db_path.display().to_string().dimmed());
    println!("  Size:      {}", db_size.cyan());
    println!("  Documents: {}", doc_count.to_string().cyan());
    println!("  Lines:     {}", line_count.to_string().cyan());
    println!();

    // Cluster Health section (if index has data)
    if doc_count > 0 {
        println!("{}", "Cluster Health".white().bold());
        let num_clusters = compute_adaptive_cluster_count(doc_count as usize);
        match load_or_create_index(&db, &db_path, num_clusters) {
            Ok((lazy_index, _from_cache)) => {
                let metrics = lazy_index.get_health_metrics();
                print_health_metrics(&metrics);
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("Embedding size mismatch") {
                    println!(
                        "  {}",
                        "Mixed-model index detected (XTR + UniXcoder embeddings)".yellow()
                    );
                    println!(
                        "  {}",
                        "Run 'sg index --force' to rebuild with a single model".dimmed()
                    );
                } else {
                    println!("  {}", format!("Could not load index: {e}").yellow());
                }
            }
        }
        println!();
    }

    // Paths section
    println!("{}", "Paths".white().bold());
    println!("  Socket:   {}", socket_path.display().to_string().dimmed());
    println!("  PID file: {}", pid_path.display().to_string().dimmed());

    Ok(())
}

/// Status command JSON output
fn cmd_status_json(
    db_path: &std::path::Path,
    pid_path: &std::path::Path,
    socket_path: &std::path::Path,
) -> Result<()> {
    // Gather daemon info
    let daemon_pid = read_daemon_pid(pid_path)?;
    let daemon_status = if daemon_pid.is_some() {
        let client = Client::new(socket_path);
        client.status().ok()
    } else {
        None
    };

    let daemon = DaemonStatusOutput {
        running: daemon_pid.is_some(),
        pid: daemon_pid,
        uptime_secs: daemon_status.as_ref().map(|s| s.uptime_secs),
        index_quality: daemon_status.as_ref().map(|s| s.index_quality),
        project_count: daemon_status.as_ref().map(|s| s.projects.len()),
        throttle_state: daemon_status
            .as_ref()
            .filter(|s| !s.throttle_state.is_empty())
            .map(|s| s.throttle_state.clone()),
        projects: daemon_status.map(|s| s.projects),
    };

    // Gather index info
    let index = if db_path.exists() {
        let size_bytes = std::fs::metadata(db_path).map(|m| m.len()).ok();
        let (document_count, line_count) = if let Ok(db) = DB::new(db_path) {
            (db.document_count().ok(), db.total_lines().ok())
        } else {
            (None, None)
        };

        IndexStatusOutput {
            exists: true,
            path: db_path.display().to_string(),
            size_bytes,
            document_count,
            line_count,
        }
    } else {
        IndexStatusOutput {
            exists: false,
            path: db_path.display().to_string(),
            size_bytes: None,
            document_count: None,
            line_count: None,
        }
    };

    let paths = PathsOutput {
        socket: socket_path.display().to_string(),
        pid_file: pid_path.display().to_string(),
    };

    let output = StatusOutput {
        daemon,
        index,
        paths,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Files command implementation - list indexed files
fn cmd_files(path: Option<&std::path::Path>, json: bool, summary: bool) -> Result<()> {
    let db_path = get_db_path()?;

    if !db_path.exists() {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "error": "Index not found",
                    "files": [],
                    "total_files": 0,
                    "total_lines": 0,
                    "total_chunks": 0
                })
            );
        } else {
            println!("{}", "Index not found.".yellow());
            println!("Run 'sg index <path>' to create an index.");
        }
        return Ok(());
    }

    let db = DB::new(&db_path)?;
    let documents = db.list_documents(path)?;

    // Calculate totals
    let total_files = documents.len();
    let total_lines: usize = documents.iter().map(|d| d.line_count).sum();
    let total_chunks: usize = documents.iter().map(|d| d.chunk_count).sum();

    if json {
        #[derive(Serialize)]
        struct FileSummary {
            path: String,
            lines: usize,
            chunks: usize,
            indexed_at: i64,
        }

        #[derive(Serialize)]
        struct FilesOutput {
            files: Vec<FileSummary>,
            total_files: usize,
            total_lines: usize,
            total_chunks: usize,
            #[serde(skip_serializing_if = "Option::is_none")]
            filter_path: Option<String>,
        }

        let files: Vec<FileSummary> = if summary {
            vec![]
        } else {
            documents
                .iter()
                .map(|d| FileSummary {
                    path: d.path.clone(),
                    lines: d.line_count,
                    chunks: d.chunk_count,
                    indexed_at: d.indexed_at,
                })
                .collect()
        };

        let output = FilesOutput {
            files,
            total_files,
            total_lines,
            total_chunks,
            filter_path: path.map(|p| p.display().to_string()),
        };

        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Human-readable output
        if let Some(filter_path) = path {
            println!(
                "{}",
                format!("Indexed files under: {}", filter_path.display())
                    .white()
                    .bold()
            );
        } else {
            println!("{}", "Indexed files".white().bold());
        }
        println!();

        if !summary {
            // Header
            println!(
                "  {:<8} {:<8} {}",
                "Lines".dimmed(),
                "Chunks".dimmed(),
                "Path".dimmed()
            );
            println!(
                "  {} {} {}",
                "─".repeat(8).dimmed(),
                "─".repeat(8).dimmed(),
                "─".repeat(50).dimmed()
            );

            for doc in &documents {
                println!(
                    "  {:>8} {:>8} {}",
                    doc.line_count.to_string().cyan(),
                    doc.chunk_count.to_string().yellow(),
                    doc.path.dimmed()
                );
            }
            println!();
        }

        // Summary
        println!("{}", "Summary".white().bold());
        println!("  Files:  {}", total_files.to_string().cyan());
        println!("  Lines:  {}", total_lines.to_string().cyan());
        println!("  Chunks: {}", total_chunks.to_string().yellow());
    }

    Ok(())
}

/// Chunks command implementation - show chunks for a file
fn cmd_chunks(path: &std::path::Path, json: bool, show_content: bool) -> Result<()> {
    let db_path = get_db_path()?;

    if !db_path.exists() {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "error": "Index not found",
                    "chunks": []
                })
            );
        } else {
            println!("{}", "Index not found.".yellow());
            println!("Run 'sg index <path>' to create an index.");
        }
        return Ok(());
    }

    let db = DB::new(&db_path)?;

    // Get document by path
    let doc = match db.get_document_by_path(path)? {
        Some(d) => d,
        None => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "error": "File not indexed",
                        "path": path.display().to_string(),
                        "chunks": []
                    })
                );
            } else {
                println!("{}", "File not indexed.".yellow());
                println!("Path: {}", path.display());
                println!("Run 'sg index <path>' to index this file.");
            }
            return Ok(());
        }
    };

    // Get chunks for document
    let chunks = db.get_chunks_for_doc(doc.id)?;

    if json {
        #[derive(Serialize)]
        struct ChunkInfo {
            index: usize,
            start_line: usize,
            end_line: usize,
            #[serde(skip_serializing_if = "String::is_empty")]
            header_context: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            language: Option<String>,
            #[serde(skip_serializing_if = "String::is_empty")]
            content_hash: String,
            link_count: usize,
            #[serde(skip_serializing_if = "Option::is_none")]
            content_preview: Option<String>,
        }

        #[derive(Serialize)]
        struct ChunksOutput {
            path: String,
            doc_id: u32,
            total_lines: usize,
            total_chunks: usize,
            chunks: Vec<ChunkInfo>,
        }

        // Get content previews if requested
        let content_lines: Vec<&str> = if show_content {
            doc.content.lines().collect()
        } else {
            vec![]
        };

        let chunk_infos: Vec<ChunkInfo> = chunks
            .iter()
            .map(|c| {
                let content_preview = if show_content {
                    let start = c.start_line.saturating_sub(1);
                    let end = c.end_line.min(content_lines.len());
                    let preview: String = content_lines[start..end]
                        .iter()
                        .take(5)
                        .copied()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .chars()
                        .take(100)
                        .collect();
                    Some(preview)
                } else {
                    None
                };

                ChunkInfo {
                    index: c.chunk_index,
                    start_line: c.start_line,
                    end_line: c.end_line,
                    header_context: c.header_context.clone(),
                    language: c.language.clone(),
                    content_hash: if c.content_hash.len() > 8 {
                        c.content_hash[..8].to_string()
                    } else {
                        c.content_hash.clone()
                    },
                    link_count: c.links.len(),
                    content_preview,
                }
            })
            .collect();

        let output = ChunksOutput {
            path: doc.path.clone(),
            doc_id: doc.id,
            total_lines: doc.line_count,
            total_chunks: chunks.len(),
            chunks: chunk_infos,
        };

        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Human-readable output
        println!("{}", format!("Chunks for: {}", doc.path).white().bold());
        println!(
            "  {} lines, {} chunks",
            doc.line_count.to_string().cyan(),
            chunks.len().to_string().yellow()
        );
        println!();

        // Header
        println!(
            "  {:<5} {:>10} {:>12} {:<20} {}",
            "#".dimmed(),
            "Lines".dimmed(),
            "Hash".dimmed(),
            "Language".dimmed(),
            "Context".dimmed()
        );
        println!(
            "  {} {} {} {} {}",
            "─".repeat(5).dimmed(),
            "─".repeat(10).dimmed(),
            "─".repeat(12).dimmed(),
            "─".repeat(20).dimmed(),
            "─".repeat(30).dimmed()
        );

        // Get content lines if showing preview
        let content_lines: Vec<&str> = if show_content {
            doc.content.lines().collect()
        } else {
            vec![]
        };

        for chunk in &chunks {
            let hash_preview = if chunk.content_hash.len() > 8 {
                &chunk.content_hash[..8]
            } else {
                &chunk.content_hash
            };

            let lang = chunk.language.as_deref().unwrap_or("-");
            let context = if chunk.header_context.is_empty() {
                "-".to_string()
            } else {
                // Truncate context if too long
                if chunk.header_context.len() > 30 {
                    format!("{}...", &chunk.header_context[..27])
                } else {
                    chunk.header_context.clone()
                }
            };

            println!(
                "  {:>5} {:>4}-{:<5} {:>12} {:<20} {}",
                chunk.chunk_index.to_string().cyan(),
                chunk.start_line,
                chunk.end_line,
                hash_preview.dimmed(),
                if lang == "-" {
                    lang.dimmed().to_string()
                } else {
                    format!("[{lang}]").magenta().to_string()
                },
                context.dimmed()
            );

            // Show content preview if requested
            if show_content && !content_lines.is_empty() {
                let start = chunk.start_line.saturating_sub(1);
                let end = chunk.end_line.min(content_lines.len());
                let preview: String = content_lines[start..end]
                    .iter()
                    .take(3)
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                let preview: String = preview.chars().take(60).collect();
                if !preview.is_empty() {
                    println!("        {}", preview.dimmed());
                }
            }

            // Show link count if any
            if !chunk.links.is_empty() {
                println!("        {} links", chunk.links.len().to_string().blue());
            }
        }
    }

    Ok(())
}

/// Index images in a directory using CLIP embeddings
fn cmd_index_images(path: &std::path::Path, json: bool) -> Result<()> {
    #[cfg(not(feature = "clip"))]
    {
        let _ = (path, json);
        eprintln!(
            "{} Image indexing requires the 'clip' feature. Build with: cargo build --features clip",
            "Error:".red().bold()
        );
        Ok(())
    }

    #[cfg(feature = "clip")]
    {
        use sg_core::{index_images_in_directory, ClipEmbedder};

        let db_path = get_db_path()?;
        let db = sg_core::DB::new(&db_path)?;

        // Load CLIP embedder with spinner
        let spinner = if !json {
            let sp = ProgressBar::new_spinner();
            sp.set_style(
                ProgressStyle::with_template("{spinner:.green} {msg} [{elapsed}]")
                    .unwrap()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
            );
            sp.set_message("Loading CLIP model...");
            sp.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(sp)
        } else {
            None
        };

        let device = sg_core::make_device();
        let mut clip_embedder = ClipEmbedder::new(&device)?;

        if let Some(sp) = &spinner {
            sp.finish_and_clear();
        }

        // Create progress bar for indexing
        let progress = if !json {
            let pb = ProgressBar::new(0);
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} images",
                )?
                .progress_chars("=>-"),
            );
            Some(pb)
        } else {
            None
        };

        // Index images
        let stats = index_images_in_directory(&db, &mut clip_embedder, path, progress.as_ref())?;

        // Output results
        if json {
            #[derive(serde::Serialize)]
            struct IndexImagesOutput {
                path: String,
                total_found: usize,
                indexed: usize,
                skipped: usize,
                failed: usize,
            }

            let output = IndexImagesOutput {
                path: path.display().to_string(),
                total_found: stats.total_found,
                indexed: stats.indexed,
                skipped: stats.skipped,
                failed: stats.failed,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!();
            println!(
                "{} Image indexing complete for {}",
                "Done:".green().bold(),
                path.display()
            );
            println!("  Found: {} images", stats.total_found.to_string().cyan());
            println!("  Indexed: {} images", stats.indexed.to_string().green());
            if stats.skipped > 0 {
                println!(
                    "  Skipped: {} (already indexed)",
                    stats.skipped.to_string().yellow()
                );
            }
            if stats.failed > 0 {
                println!("  Failed: {} images", stats.failed.to_string().red());
            }
            println!();
            println!(
                "Search images with: {}",
                "sg --images \"your description\"".cyan()
            );
        }

        Ok(())
    }
}

/// Compact the index database
fn cmd_compact(json: bool) -> Result<()> {
    let db_path = get_db_path()?;

    if !db_path.exists() {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "error": "Index database does not exist",
                    "path": db_path.display().to_string()
                })
            );
        } else {
            println!("{}", "Index database does not exist".red());
            println!("Run 'sg index <path>' first to create an index.");
        }
        return Ok(());
    }

    if !json {
        println!("{}", "Compacting Index".cyan().bold());
        println!("{}", "━".repeat(50).dimmed());
        println!();
        println!("Database: {}", db_path.display().to_string().dimmed());
        println!();
    }

    // Open database with write access for compaction
    let mut db = DB::new(&db_path)?;

    // Run compaction
    let stats = db.compact()?;

    if json {
        #[derive(Serialize)]
        struct CompactOutput {
            documents_before: usize,
            documents_after: usize,
            chunks_before: usize,
            chunks_after: usize,
            size_before_bytes: u64,
            size_after_bytes: u64,
            space_reclaimed_bytes: u64,
            orphaned_embeddings_removed: usize,
            orphaned_chunks_removed: usize,
            orphaned_chunk_embeddings_removed: usize,
            stale_centroids_removed: usize,
            total_removed: usize,
        }

        let output = CompactOutput {
            documents_before: stats.documents_before,
            documents_after: stats.documents_after,
            chunks_before: stats.chunks_before,
            chunks_after: stats.chunks_after,
            size_before_bytes: stats.size_before_bytes,
            size_after_bytes: stats.size_after_bytes,
            space_reclaimed_bytes: stats.space_reclaimed(),
            orphaned_embeddings_removed: stats.orphaned_embeddings_removed,
            orphaned_chunks_removed: stats.orphaned_chunks_removed,
            orphaned_chunk_embeddings_removed: stats.orphaned_chunk_embeddings_removed,
            stale_centroids_removed: stats.stale_centroids_removed,
            total_removed: stats.total_removed(),
        };

        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Human-readable output
        let total_removed = stats.total_removed();
        let space_reclaimed = stats.space_reclaimed();

        if total_removed > 0 || space_reclaimed > 0 {
            println!("{}", "Cleaned up:".green().bold());

            if stats.orphaned_embeddings_removed > 0 {
                println!(
                    "  Orphaned embeddings:       {}",
                    stats.orphaned_embeddings_removed.to_string().yellow()
                );
            }
            if stats.orphaned_chunks_removed > 0 {
                println!(
                    "  Orphaned chunks:           {}",
                    stats.orphaned_chunks_removed.to_string().yellow()
                );
            }
            if stats.orphaned_chunk_embeddings_removed > 0 {
                println!(
                    "  Orphaned chunk embeddings: {}",
                    stats.orphaned_chunk_embeddings_removed.to_string().yellow()
                );
            }
            if stats.stale_centroids_removed > 0 {
                println!(
                    "  Stale centroids:           {}",
                    stats.stale_centroids_removed.to_string().yellow()
                );
            }
            println!();
        } else {
            println!("{}", "No orphaned data found.".green());
            println!();
        }

        println!("{}", "Storage:".white().bold());
        println!(
            "  Before: {} ({} documents, {} chunks)",
            CompactionStats::format_size(stats.size_before_bytes).cyan(),
            stats.documents_before,
            stats.chunks_before
        );
        println!(
            "  After:  {} ({} documents, {} chunks)",
            CompactionStats::format_size(stats.size_after_bytes).cyan(),
            stats.documents_after,
            stats.chunks_after
        );

        if space_reclaimed > 0 {
            println!(
                "  {}",
                format!(
                    "Reclaimed {} ({:.1}%)",
                    CompactionStats::format_size(space_reclaimed),
                    (space_reclaimed as f64 / stats.size_before_bytes as f64) * 100.0
                )
                .green()
            );
        }
        println!();
        println!("{}", "Compaction complete!".green().bold());
    }

    Ok(())
}

fn cmd_rebalance() -> Result<()> {
    let db_path = get_db_path()?;

    if !db_path.exists() {
        println!("{}", "Index database does not exist".red());
        println!("Run 'sg index <path>' first to create an index.");
        return Ok(());
    }

    println!("{}", "Rebalancing Clusters".cyan().bold());
    println!("{}", "━".repeat(50).dimmed());
    println!();
    println!("Database: {}", db_path.display().to_string().dimmed());
    println!();

    // Open database
    let db = DB::new(&db_path)?;

    // Get document count for adaptive cluster count
    let doc_count = db.document_count()?;
    let num_clusters = compute_adaptive_cluster_count(doc_count);

    // Load the index
    let (mut index, _from_cache) = load_or_create_index(&db, &db_path, num_clusters)?;

    // Get health metrics before rebalancing
    let metrics_before = index.get_health_metrics();
    println!("{}", "Before:".white().bold());
    println!(
        "  Health:      {}",
        format_health_score(metrics_before.health_score)
    );
    println!(
        "  Clusters:    {} ({} empty)",
        metrics_before.cluster_count, metrics_before.empty_clusters
    );
    println!("  Imbalance:   {:.1}x", metrics_before.imbalance_ratio);
    println!();

    // Perform rebalancing
    let moved = index.rebalance_clusters()?;

    // Get health metrics after
    let metrics_after = index.get_health_metrics();

    println!("{}", "After:".white().bold());
    println!(
        "  Health:      {}",
        format_health_score(metrics_after.health_score)
    );
    println!(
        "  Clusters:    {} ({} empty)",
        metrics_after.cluster_count, metrics_after.empty_clusters
    );
    println!("  Imbalance:   {:.1}x", metrics_after.imbalance_ratio);
    println!();

    if moved > 0 {
        println!("{}", format!("Moved {moved} embeddings").green().bold());

        // Save the updated index
        let index_path = db_path.with_extension("idx");
        index.save(&index_path)?;
        println!(
            "Index saved to {}",
            index_path.display().to_string().dimmed()
        );
    } else {
        println!(
            "{}",
            "Clusters already balanced - no changes needed.".green()
        );
    }

    Ok(())
}

fn format_health_score(health_score: f32) -> String {
    // health_score is 0.0 (perfect) to 1.0 (poor), convert to percentage
    let pct = ((1.0 - health_score) * 100.0).round() as u32;
    let s = format!("{pct}%");
    if pct >= 80 {
        s.green().to_string()
    } else if pct >= 50 {
        s.yellow().to_string()
    } else {
        s.red().to_string()
    }
}

/// Format bytes to human readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Print health metrics for the index
fn print_health_metrics(metrics: &IndexHealthMetrics) {
    // Health score as percentage (inverted: 100% = perfect, 0% = poor)
    let health_pct = ((1.0 - metrics.health_score) * 100.0).round();
    let health_str = format!("{health_pct}%");
    let health_colored = if health_pct >= 80.0 {
        health_str.green()
    } else if health_pct >= 60.0 {
        health_str.yellow()
    } else {
        health_str.red()
    };
    println!("  Health:       {health_colored}");

    // Cluster statistics
    println!(
        "  Clusters:     {} ({} empty)",
        metrics.cluster_count.to_string().cyan(),
        metrics.empty_clusters
    );
    println!(
        "  Distribution: {} / {} / {} (min/avg/max)",
        metrics.smallest_cluster,
        format!("{:.1}", metrics.avg_cluster_size).cyan(),
        metrics.largest_cluster
    );

    // Imbalance warning
    if metrics.needs_rebalancing {
        println!(
            "  {}",
            format!(
                "⚠ Imbalanced: {:.1}x ratio (run 'sg index --rebalance')",
                metrics.imbalance_ratio
            )
            .yellow()
        );
    } else if metrics.imbalance_ratio > 1.0 {
        println!("  Imbalance:    {:.1}x", metrics.imbalance_ratio);
    }

    // Features
    let mut features = Vec::new();
    if metrics.using_kmeans {
        features.push("k-means");
    } else {
        features.push("LSH");
    }
    if metrics.using_quantization {
        features.push("PQ");
    }
    if metrics.using_hnsw {
        features.push("HNSW");
    }
    println!("  Features:     {}", features.join(", ").dimmed());

    // Storage estimate
    if metrics.storage_bytes > 0 {
        println!(
            "  Embeddings:   {} ({})",
            metrics.total_docs.to_string().cyan(),
            format_bytes(metrics.storage_bytes).dimmed()
        );
    }
}

/// Start the daemon
fn cmd_daemon_start(foreground: bool) -> Result<()> {
    let pid_path = default_pid_path();

    // Check if already running
    if let Some(pid) = read_daemon_pid(&pid_path)? {
        println!(
            "{} Daemon already running (pid {})",
            "Note:".yellow().bold(),
            pid
        );
        return Ok(());
    }

    let daemon_binary = find_daemon_binary();

    if foreground {
        // Run in foreground - exec the daemon binary
        println!("Starting daemon in foreground...");
        let status = Command::new(&daemon_binary)
            .arg("--foreground")
            .status()
            .context("Failed to start daemon")?;
        if !status.success() {
            anyhow::bail!("Daemon exited with error");
        }
    } else {
        // Start daemon in background
        println!("Starting daemon...");
        let child = Command::new(&daemon_binary)
            .spawn()
            .context("Failed to start daemon")?;

        // Poll for startup with increasing delays (total ~2.5s max)
        let socket_path = resolved_socket_path();
        let mut started = false;
        for delay_ms in [100, 200, 400, 800, 1000] {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            if let Some(pid) = read_daemon_pid(&pid_path)? {
                // Verify the daemon is actually responding
                let client = Client::new(&socket_path);
                if client.is_daemon_running() {
                    println!("{} (pid {})", "Daemon started".green().bold(), pid);
                    started = true;
                    break;
                }
            }
        }

        if !started {
            eprintln!(
                "{} Daemon may have failed to start. Check logs at {:?}",
                "Warning:".yellow().bold(),
                pid_path.parent().map(|p| p.join("sg-daemon.log"))
            );
        }

        // Detach from child
        drop(child);
    }

    Ok(())
}

/// Stop the daemon
fn cmd_daemon_stop() -> Result<()> {
    let pid_path = default_pid_path();
    let socket_path = resolved_socket_path();

    // First try graceful shutdown via socket
    let client = Client::new(&socket_path);
    if client.is_daemon_running() {
        println!("Requesting daemon shutdown...");
        match client.shutdown() {
            Ok(_) => {
                // Wait for graceful shutdown
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            Err(_) => {
                // Ignore errors, will try to kill below
            }
        }
    }

    // Check if still running and kill if necessary
    if let Some(pid) = read_daemon_pid(&pid_path)? {
        println!("Stopping daemon (pid {pid})...");
        if kill_daemon(&pid_path)? {
            println!("{}", "Daemon stopped".green().bold());
        }
    } else {
        println!("{}", "Daemon not running".yellow());
    }

    // Clean up socket file
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).ok();
    }

    Ok(())
}

/// Show daemon status
fn cmd_daemon_status(json: bool) -> Result<()> {
    let pid_path = default_pid_path();
    let socket_path = resolved_socket_path();

    if json {
        #[derive(Serialize)]
        struct DaemonStatusJson {
            running: bool,
            pid: Option<u32>,
            uptime_secs: Option<u64>,
            index_quality: Option<f32>,
            project_count: Option<usize>,
            throttle_state: Option<String>,
            projects: Option<Vec<ProjectStatus>>,
            socket: String,
            pid_file: String,
        }

        let daemon_pid = read_daemon_pid(&pid_path)?;
        let daemon_status = if daemon_pid.is_some() {
            let client = Client::new(&socket_path);
            client.status().ok()
        } else {
            None
        };

        let output = DaemonStatusJson {
            running: daemon_pid.is_some(),
            pid: daemon_pid,
            uptime_secs: daemon_status.as_ref().map(|s| s.uptime_secs),
            index_quality: daemon_status.as_ref().map(|s| s.index_quality),
            project_count: daemon_status.as_ref().map(|s| s.projects.len()),
            throttle_state: daemon_status
                .as_ref()
                .filter(|s| !s.throttle_state.is_empty())
                .map(|s| s.throttle_state.clone()),
            projects: daemon_status.map(|s| s.projects),
            socket: socket_path.display().to_string(),
            pid_file: pid_path.display().to_string(),
        };

        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("{}", "Daemon Status".cyan().bold());
    println!();

    match read_daemon_pid(&pid_path)? {
        Some(pid) => {
            println!("Status: {} (pid {})", "running".green(), pid);

            // Try to get more info from daemon
            let client = Client::new(&socket_path);
            if let Ok(status) = client.status() {
                let uptime = format_duration(status.uptime_secs);
                println!("Uptime: {}", uptime.cyan());
                println!(
                    "Index quality: {}%",
                    (status.index_quality * 100.0).round().to_string().cyan()
                );
            }
        }
        None => {
            println!("Status: {}", "not running".yellow());
        }
    }

    println!();
    println!("Socket: {}", socket_path.display().to_string().dimmed());
    println!("PID file: {}", pid_path.display().to_string().dimmed());

    Ok(())
}

/// Format a duration in seconds to a human-readable string
fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs < 86400 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
    }
}

/// List known projects
fn cmd_project_list(json: bool) -> Result<()> {
    let socket_path = resolved_socket_path();
    let client = Client::new(&socket_path);

    if !client.is_daemon_running() {
        if json {
            println!(r#"{{"error": "Daemon not running"}}"#);
        } else {
            eprintln!(
                "{} Daemon not running. Start with 'sg daemon start'",
                "Error:".red().bold()
            );
        }
        return Ok(());
    }

    let projects = client.list_projects()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&projects)?);
        return Ok(());
    }

    println!("{}", "Known Projects".cyan().bold());
    println!();

    if projects.is_empty() {
        println!("No projects discovered yet.");
        println!("Run 'sg project discover' to scan common locations.");
        return Ok(());
    }

    for project in &projects {
        let status = if project.is_watching {
            "watching".green()
        } else {
            "idle".dimmed()
        };
        let last_access = format_duration(project.last_accessed_secs_ago);
        println!(
            "  {} {} [{}] ({})",
            if project.is_watching { "*" } else { " " },
            project.path.cyan(),
            project.project_type.dimmed(),
            status
        );
        if project.last_accessed_secs_ago > 0 {
            println!("      Last accessed: {} ago", last_access.dimmed());
        }
    }

    println!();
    println!("{} projects total", projects.len().to_string().cyan());

    Ok(())
}

/// Run project discovery
fn cmd_project_discover() -> Result<()> {
    let socket_path = resolved_socket_path();
    let client = Client::new(&socket_path);

    if !client.is_daemon_running() {
        eprintln!(
            "{} Daemon not running. Start with 'sg daemon start'",
            "Error:".red().bold()
        );
        return Ok(());
    }

    println!("Discovering projects in common locations...");
    client.discover_projects()?;
    println!("{}", "Discovery complete".green().bold());

    // Show the list
    cmd_project_list(false)
}

/// Detect project root from path
fn cmd_project_detect(path: &std::path::Path) -> Result<()> {
    // Try via daemon first
    let socket_path = resolved_socket_path();
    let client = Client::new(&socket_path);

    if client.is_daemon_running() {
        match client.detect_root(path)? {
            Some(root) => {
                println!("Project root: {}", root.cyan().bold());
            }
            None => {
                println!(
                    "{} No project root found for {}",
                    "Note:".yellow(),
                    path.display()
                );
            }
        }
    } else {
        // Fall back to direct detection (without daemon)
        match find_project_root(path) {
            Some(root) => {
                println!("Project root: {}", root.display().to_string().cyan().bold());
            }
            None => {
                println!(
                    "{} No project root found for {}",
                    "Note:".yellow(),
                    path.display()
                );
            }
        }
    }

    Ok(())
}

/// Generate shell initialization script for cd hooks
fn generate_init_script(shell: Shell) -> String {
    match shell {
        Shell::Bash => r#"# SuperGrep shell integration for Bash
# Add this to your ~/.bashrc:
#   eval "$(sg init bash)"

__sg_cd_hook() {
    # Run sg project detect in background to register with daemon
    # The command handles gracefully when daemon isn't running
    sg project detect "$PWD" >/dev/null 2>&1 &
    disown 2>/dev/null
}

# Wrap cd to call hook after directory change
__sg_cd() {
    builtin cd "$@" && __sg_cd_hook
}

# Wrap pushd/popd similarly
__sg_pushd() {
    builtin pushd "$@" && __sg_cd_hook
}

__sg_popd() {
    builtin popd "$@" && __sg_cd_hook
}

alias cd='__sg_cd'
alias pushd='__sg_pushd'
alias popd='__sg_popd'
"#
        .to_string(),

        Shell::Zsh => r#"# SuperGrep shell integration for Zsh
# Add this to your ~/.zshrc:
#   eval "$(sg init zsh)"

__sg_chpwd_hook() {
    # Run sg project detect in background to register with daemon
    # The command handles gracefully when daemon isn't running
    sg project detect "$PWD" >/dev/null 2>&1 &!
}

# Use zsh's built-in chpwd hook system
autoload -Uz add-zsh-hook
add-zsh-hook chpwd __sg_chpwd_hook
"#
        .to_string(),

        Shell::Fish => r#"# SuperGrep shell integration for Fish
# Add this to your ~/.config/fish/config.fish:
#   sg init fish | source

function __sg_cd_hook --on-variable PWD
    # Run sg project detect in background to register with daemon
    # The command handles gracefully when daemon isn't running
    sg project detect "$PWD" >/dev/null 2>&1 &
    disown 2>/dev/null
end
"#
        .to_string(),

        Shell::Elvish => r"# SuperGrep shell integration for Elvish
# Add this to your ~/.elvish/rc.elv:
#   eval (sg init elvish | slurp)

set after-chdir = [
    $@after-chdir
    {|_|
        # Run sg project detect in background to register with daemon
        # The command handles gracefully when daemon isn't running
        sg project detect $pwd >/dev/null 2>&1 &
    }
]
"
        .to_string(),

        Shell::PowerShell => r#"# SuperGrep shell integration for PowerShell
# Add this to your $PROFILE:
#   Invoke-Expression (sg init powershell | Out-String)

function __sg_cd_hook {
    param($Path)
    # Run sg project detect in background to register with daemon
    # The command handles gracefully when daemon isn't running
    Start-Job -ScriptBlock { sg project detect $using:PWD } | Out-Null
}

# Use prompt to detect directory changes
$__sg_last_dir = $PWD.Path
function prompt {
    if ($PWD.Path -ne $script:__sg_last_dir) {
        $script:__sg_last_dir = $PWD.Path
        __sg_cd_hook
    }
    # Return default prompt
    "PS $($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1)) "
}
"#
        .to_string(),

        _ => format!("# Shell '{shell}' is not supported for sg init\n# Supported shells: bash, zsh, fish, elvish, powershell\n"),
    }
}

/// Run performance benchmarks
fn cmd_benchmark(path: &std::path::Path, iterations: usize) -> Result<()> {
    use std::time::Instant;

    println!("{}", "━".repeat(50).dimmed());
    println!("{}", "          Performance Benchmark".cyan().bold());
    println!("{}", "━".repeat(50).dimmed());
    println!();

    // Use temporary database for clean benchmarks
    let temp_dir = tempfile::TempDir::new().context("Failed to create temp directory")?;
    let db_path = temp_dir.path().join("benchmark.db");

    // 1. Model load time
    println!("{}", "1. Model Loading".white().bold());
    let start = Instant::now();
    let mut embedder = load_embedder()?;
    let model_load_time = start.elapsed();
    println!("   Backend: {}", embedder.kind().as_str());
    println!("   Load time: {:.2}s", model_load_time.as_secs_f64());
    println!();

    // 2. Indexing benchmark
    println!("{}", "2. Indexing".white().bold());
    println!("   Path: {}", path.display());

    let db = DB::new(&db_path)?;
    let start = Instant::now();
    let stats = index_directory_with_options_backend(
        &db,
        &mut embedder,
        path,
        None,
        IndexDirectoryOptions {
            allow_temp_paths: true,
            ..Default::default()
        },
    )?;
    let index_time = start.elapsed();

    let files_per_sec = if index_time.as_secs_f64() > 0.0 {
        stats.indexed_files as f64 / index_time.as_secs_f64()
    } else {
        0.0
    };
    let lines_per_sec = if index_time.as_secs_f64() > 0.0 {
        stats.total_lines as f64 / index_time.as_secs_f64()
    } else {
        0.0
    };

    println!("   Files: {}", stats.indexed_files);
    println!("   Lines: {}", stats.total_lines);
    println!("   Time: {:.2}s", index_time.as_secs_f64());
    println!(
        "   Throughput: {files_per_sec:.1} files/s, {lines_per_sec:.0} lines/s"
    );
    println!();

    // 3. Search latency benchmark
    println!("{}", "3. Search Latency".white().bold());
    println!("   Iterations: {iterations}");

    // Load or create index for clustered search (more realistic benchmark)
    // Lower threshold than regular search to exercise clustered path in benchmarks
    let doc_count = db.document_count()?;
    const CLUSTERED_THRESHOLD: usize = 50;
    let lazy_index = if doc_count > CLUSTERED_THRESHOLD {
        let num_clusters = compute_adaptive_cluster_count(doc_count);
        match load_or_create_index(&db, &db_path, num_clusters) {
            Ok((index, _)) => Some(index),
            Err(_) => None,
        }
    } else {
        None
    };

    let search_mode = if lazy_index.is_some() {
        "clustered"
    } else {
        "brute-force"
    };
    println!("   Mode: {search_mode}");

    let test_queries = [
        "function definition",
        "error handling",
        "database connection",
        "authentication",
        "parse json",
    ];

    let mut latencies_ms: Vec<f64> = Vec::new();

    for query in &test_queries {
        for _ in 0..iterations {
            let start = Instant::now();
            let options = SearchOptions {
                top_k: 10,
                ..Default::default()
            };
            if let Some(ref index) = lazy_index {
                let _ = search_clustered_backend(&db, index, &mut embedder, query, options)?;
            } else {
                let _ = search_backend(&db, &mut embedder, query, options)?;
            }
            latencies_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }

    // Calculate percentiles
    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let search_count = latencies_ms.len();
    let total_search_time_ms: f64 = latencies_ms.iter().sum();
    let avg_latency_ms = total_search_time_ms / search_count as f64;
    let queries_per_sec = search_count as f64 / (total_search_time_ms / 1000.0);

    let p50_idx = (search_count as f64 * 0.50) as usize;
    let p95_idx = (search_count as f64 * 0.95) as usize;
    let p99_idx = (search_count as f64 * 0.99) as usize;

    let p50 = latencies_ms
        .get(p50_idx.min(search_count - 1))
        .copied()
        .unwrap_or(0.0);
    let p95 = latencies_ms
        .get(p95_idx.min(search_count - 1))
        .copied()
        .unwrap_or(0.0);
    let p99 = latencies_ms
        .get(p99_idx.min(search_count - 1))
        .copied()
        .unwrap_or(0.0);

    println!("   Total queries: {search_count}");
    println!("   Avg latency: {avg_latency_ms:.1}ms");
    println!("   p50 latency: {p50:.1}ms");
    println!("   p95 latency: {p95:.1}ms");
    println!("   p99 latency: {p99:.1}ms");
    println!("   Throughput: {queries_per_sec:.1} queries/s");
    println!();

    // 4. Summary
    println!("{}", "━".repeat(50).dimmed());
    println!("{}", "Summary".cyan().bold());
    println!("{}", "━".repeat(50).dimmed());
    println!("   Model load:      {:.2}s", model_load_time.as_secs_f64());
    println!("   Index time:      {:.2}s", index_time.as_secs_f64());
    println!("   Index rate:      {files_per_sec:.1} files/s");
    println!(
        "   Search latency:  {avg_latency_ms:.1}ms avg, {p50:.1}ms p50, {p95:.1}ms p95"
    );
    println!("   Search rate:     {queries_per_sec:.1} queries/s");

    // Memory usage (rough estimate from database size)
    if let Ok(metadata) = std::fs::metadata(&db_path) {
        println!("   Index size:      {}", format_bytes(metadata.len()));
    }

    println!();
    println!(
        "{}",
        "Note: Results vary by hardware and codebase size.".dimmed()
    );

    Ok(())
}

/// Input row for bulk search CSV
#[derive(Debug, Deserialize)]
struct BulkInputRow {
    /// The search query
    query: String,
    /// Optional limit override for this query
    #[serde(default)]
    limit: Option<usize>,
}

/// Output row for bulk search CSV
#[derive(Debug, Serialize)]
struct BulkOutputRow {
    /// The original query
    query: String,
    /// Rank within results (1-based)
    rank: usize,
    /// File path
    path: String,
    /// Relevance score
    score: f32,
    /// Line number
    line: usize,
    /// Match quality tier
    quality: String,
    /// Code snippet (may be truncated)
    snippet: String,
}

/// Bulk search command implementation
fn cmd_bulk(
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    search_path: &std::path::Path,
    default_limit: usize,
    no_header: bool,
    quiet: bool,
) -> Result<()> {
    let db_path = get_db_path()?;

    // Check if database exists
    if !db_path.exists() {
        anyhow::bail!("No index found. Run 'sg index' first.");
    }

    // Open database
    let db = DB::new(&db_path)?;

    // Check if we have any documents
    let doc_count = db.document_count()?;
    if doc_count == 0 {
        anyhow::bail!("Index is empty. Run 'sg index' first.");
    }

    // Load embedder (with spinner if not quiet)
    let mut embedder = load_embedder_with_spinner(!quiet)?;

    // Load or create index for clustered search
    const CLUSTERED_THRESHOLD: u64 = 100;
    let lazy_index = if doc_count > CLUSTERED_THRESHOLD as usize {
        let num_clusters = compute_adaptive_cluster_count(doc_count);
        let (index, _) = load_or_create_index(&db, &db_path, num_clusters)?;
        Some(index)
    } else {
        None
    };

    // Read input queries
    let queries = read_bulk_input(input.as_deref(), no_header)?;

    if queries.is_empty() {
        if !quiet {
            eprintln!("{} No queries found in input", "Warning:".yellow().bold());
        }
        return Ok(());
    }

    if !quiet {
        eprintln!("Processing {} queries...", queries.len().to_string().cyan());
    }

    // Create progress bar
    let pb = if quiet {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new(queries.len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} queries",
            )
            .unwrap()
            .progress_chars("#>-"),
        );
        pb
    };

    // Prepare output writer
    let mut writer = create_bulk_output_writer(output.as_deref())?;

    // Write header
    writer.write_record([
        "query", "rank", "path", "score", "line", "quality", "snippet",
    ])?;

    // Process each query
    let mut total_results = 0usize;
    for input_row in &queries {
        let query = input_row.query.trim();
        if query.is_empty() {
            pb.inc(1);
            continue;
        }

        let limit = input_row.limit.unwrap_or(default_limit);
        let options = SearchOptions {
            top_k: limit,
            threshold: 0.0,
            hybrid: true, // Use hybrid search by default
            root: Some(search_path.to_path_buf()),
            context: 2,
            file_types: Vec::new(),
            exclude_file_types: Vec::new(),
            ..SearchOptions::default()
        };

        // Execute search
        let results = if let Some(ref index) = lazy_index {
            search_clustered_backend(&db, index, &mut embedder, query, options)?
        } else {
            search_backend(&db, &mut embedder, query, options)?
        };

        // Write results
        for (rank, result) in results.iter().enumerate() {
            let quality = MatchQuality::from_score(result.score);
            let display_path =
                format_result_path(&result.path.display().to_string(), Some(search_path));

            // Truncate snippet to single line for CSV
            let snippet = result
                .snippet
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .chars()
                .take(200)
                .collect::<String>();

            let output_row = BulkOutputRow {
                query: query.to_string(),
                rank: rank + 1,
                path: display_path,
                score: result.score,
                line: result.line,
                quality: quality.label().to_string(),
                snippet,
            };

            writer.serialize(&output_row)?;
            total_results += 1;
        }

        pb.inc(1);
    }

    writer.flush()?;
    pb.finish_and_clear();

    if !quiet {
        eprintln!(
            "{} {} queries processed, {} total results",
            "Done:".green().bold(),
            queries.len(),
            total_results
        );
    }

    Ok(())
}

/// Read bulk input from a file or stdin
fn read_bulk_input(input: Option<&std::path::Path>, no_header: bool) -> Result<Vec<BulkInputRow>> {
    let mut queries = Vec::new();

    if let Some(path) = input {
        // Read from file
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open input file: {}", path.display()))?;
        let mut reader = ReaderBuilder::new()
            .has_headers(!no_header)
            .flexible(true)
            .from_reader(file);

        for result in reader.deserialize() {
            let row: BulkInputRow = result.context("Failed to parse CSV row")?;
            queries.push(row);
        }
    } else {
        // Read from stdin
        let stdin = io::stdin();
        let handle = stdin.lock();

        // Check if stdin has data (non-interactive)
        if atty::is(atty::Stream::Stdin) {
            // Interactive mode - read line by line as simple queries
            eprintln!("Enter queries (one per line, Ctrl+D to finish):");
            for line in handle.lines() {
                let line = line.context("Failed to read line")?;
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    queries.push(BulkInputRow {
                        query: trimmed.to_string(),
                        limit: None,
                    });
                }
            }
        } else {
            // Piped input - try to parse as CSV
            let mut reader = ReaderBuilder::new()
                .has_headers(!no_header)
                .flexible(true)
                .from_reader(handle);

            for result in reader.deserialize() {
                match result {
                    Ok(row) => queries.push(row),
                    Err(_) => {
                        // Fall back to treating as simple text lines
                        break;
                    }
                }
            }

            // If CSV parsing failed, try reading as simple lines
            if queries.is_empty() {
                let stdin = io::stdin();
                for line in stdin.lock().lines() {
                    let line = line.context("Failed to read line")?;
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        queries.push(BulkInputRow {
                            query: trimmed.to_string(),
                            limit: None,
                        });
                    }
                }
            }
        }
    }

    Ok(queries)
}

/// Create CSV writer for bulk output
fn create_bulk_output_writer(
    output: Option<&std::path::Path>,
) -> Result<csv::Writer<Box<dyn Write>>> {
    let writer: Box<dyn Write> = if let Some(path) = output {
        Box::new(
            std::fs::File::create(path)
                .with_context(|| format!("Failed to create output file: {}", path.display()))?,
        )
    } else {
        Box::new(io::stdout())
    };

    Ok(WriterBuilder::new().from_writer(writer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(1), "1s");
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(59), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60), "1m 0s");
        assert_eq!(format_duration(61), "1m 1s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3599), "59m 59s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600), "1h 0m");
        assert_eq!(format_duration(3660), "1h 1m");
        assert_eq!(format_duration(7200), "2h 0m");
        assert_eq!(format_duration(86399), "23h 59m");
    }

    #[test]
    fn test_format_duration_days() {
        assert_eq!(format_duration(86400), "1d 0h");
        assert_eq!(format_duration(90000), "1d 1h");
        assert_eq!(format_duration(172800), "2d 0h");
        assert_eq!(format_duration(604800), "7d 0h");
    }

    #[test]
    fn test_format_bytes_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1), "1 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn test_format_bytes_kilobytes() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(10240), "10.0 KB");
        assert_eq!(format_bytes(1048575), "1024.0 KB");
    }

    #[test]
    fn test_format_bytes_megabytes() {
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1572864), "1.5 MB");
        assert_eq!(format_bytes(104857600), "100.0 MB");
        assert_eq!(format_bytes(1073741823), "1024.0 MB");
    }

    #[test]
    fn test_format_bytes_gigabytes() {
        assert_eq!(format_bytes(1073741824), "1.0 GB");
        assert_eq!(format_bytes(1610612736), "1.5 GB");
        assert_eq!(format_bytes(10737418240), "10.0 GB");
    }

    #[test]
    fn test_generate_init_script_bash() {
        let script = generate_init_script(Shell::Bash);
        assert!(script.contains("SuperGrep shell integration for Bash"));
        assert!(script.contains("__sg_cd_hook"));
        assert!(script.contains("alias cd='__sg_cd'"));
        assert!(script.contains("sg project detect"));
    }

    #[test]
    fn test_generate_init_script_zsh() {
        let script = generate_init_script(Shell::Zsh);
        assert!(script.contains("SuperGrep shell integration for Zsh"));
        assert!(script.contains("__sg_chpwd_hook"));
        assert!(script.contains("add-zsh-hook chpwd"));
        assert!(script.contains("sg project detect"));
    }

    #[test]
    fn test_generate_init_script_fish() {
        let script = generate_init_script(Shell::Fish);
        assert!(script.contains("SuperGrep shell integration for Fish"));
        assert!(script.contains("__sg_cd_hook --on-variable PWD"));
        assert!(script.contains("sg project detect"));
    }

    #[test]
    fn test_generate_init_script_elvish() {
        let script = generate_init_script(Shell::Elvish);
        assert!(script.contains("SuperGrep shell integration for Elvish"));
        assert!(script.contains("after-chdir"));
        assert!(script.contains("sg project detect"));
    }

    #[test]
    fn test_generate_init_script_powershell() {
        let script = generate_init_script(Shell::PowerShell);
        assert!(script.contains("SuperGrep shell integration for PowerShell"));
        assert!(script.contains("__sg_cd_hook"));
        assert!(script.contains("Start-Job"));
        assert!(script.contains("sg project detect"));
    }

    #[test]
    fn test_format_result_path_with_root() {
        let root = std::path::Path::new("/home/user/project");
        let path = "/home/user/project/src/main.rs";
        let result = format_result_path(path, Some(root));
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn test_format_result_path_without_root() {
        let path = "/home/user/project/src/main.rs";
        let result = format_result_path(path, None);
        assert_eq!(result, "/home/user/project/src/main.rs");
    }

    #[test]
    fn test_format_result_path_not_under_root() {
        let root = std::path::Path::new("/home/user/other");
        let path = "/home/user/project/src/main.rs";
        let result = format_result_path(path, Some(root));
        // Path not under root, should return original
        assert_eq!(result, "/home/user/project/src/main.rs");
    }

    #[test]
    fn test_format_result_path_root_is_path() {
        let root = std::path::Path::new("/home/user/project");
        let path = "/home/user/project";
        let result = format_result_path(path, Some(root));
        // When path equals root, returns the directory name
        assert_eq!(result, "project");
    }

    #[test]
    fn test_resolve_path_with_none_uses_current_dir() {
        // When path is None, resolve_path should use current directory
        let result = resolve_path(None);
        assert!(result.is_ok());
        let resolved = result.unwrap();
        // Should be an absolute path
        assert!(resolved.is_absolute());
        // Should match current directory
        let current = std::env::current_dir().unwrap().canonicalize().unwrap();
        assert_eq!(resolved, current);
    }

    #[test]
    fn test_resolve_path_with_some_path() {
        // Create a temp directory to test with
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().to_path_buf();

        let result = resolve_path(Some(path.clone()));
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(resolved.is_absolute());
        // Should be canonicalized form of the path
        assert_eq!(resolved, path.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_path_nonexistent_path_fails() {
        let nonexistent = PathBuf::from("/this/path/definitely/does/not/exist/xyz123");
        let result = resolve_path(Some(nonexistent));
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_path_allow_missing_existing_path() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().to_path_buf();

        let result = resolve_path_allow_missing(path.clone());
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved, path.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_path_allow_missing_keeps_missing_absolute() {
        let missing = PathBuf::from("/this/path/definitely/does/not/exist/xyz123");
        let result = resolve_path_allow_missing(missing.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), missing);
    }

    #[test]
    fn test_resolve_path_allow_missing_rel_to_absolute() {
        let missing = PathBuf::from("missing-relative-path-xyz123");
        let result = resolve_path_allow_missing(missing);
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("missing-relative-path-xyz123"));
    }

    #[test]
    fn test_get_db_path_returns_valid_structure() {
        let result = get_db_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        // Should end with index.db
        assert_eq!(path.file_name().unwrap(), "index.db");
        // Should be an absolute path
        assert!(path.is_absolute());
    }

    #[test]
    fn test_find_daemon_binary_returns_path() {
        let path = find_daemon_binary();
        // Should return some path (either next to exe or just "sg-daemon")
        assert!(!path.as_os_str().is_empty());
        // The file name should be sg-daemon (possibly with .exe on Windows)
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("sg-daemon"));
    }

    #[test]
    fn test_search_args_to_search_cli_options_defaults() {
        let args = SearchArgs {
            limit: 10,
            path: None,
            no_hybrid: false,
            auto_hybrid: false,
            rerank: false,
            direct: false,
            json: false,
            context: 2,
            no_auto_index: false,
            file_types: vec![],
            exclude_file_types: vec![],
            no_preprocess_query: false,
            verbose: false,
            images: false,
        };
        let opts = SearchCliOptions::from(&args);
        assert_eq!(opts.limit, 10);
        assert!(opts.hybrid); // no_hybrid=false means hybrid=true
        assert!(!opts.auto_hybrid);
        assert!(!opts.rerank);
        assert!(!opts.direct);
        assert!(!opts.json);
        assert_eq!(opts.context, 2);
        assert!(!opts.no_auto_index);
        assert!(opts.file_types.is_empty());
        assert!(opts.exclude_file_types.is_empty());
        assert!(opts.preprocess_query);
        assert!(!opts.verbose);
    }

    #[test]
    fn test_search_args_to_search_cli_options_no_hybrid() {
        let args = SearchArgs {
            limit: 5,
            path: None,
            no_hybrid: true, // Disable hybrid search
            auto_hybrid: false,
            rerank: false,
            direct: true,
            json: true,
            context: 5,
            no_auto_index: true,
            file_types: vec!["rs".to_string(), "py".to_string()],
            exclude_file_types: vec!["test.rs".to_string()],
            no_preprocess_query: true,
            verbose: true,
            images: false,
        };
        let opts = SearchCliOptions::from(&args);
        assert_eq!(opts.limit, 5);
        assert!(!opts.hybrid); // no_hybrid=true means hybrid=false
        assert!(!opts.auto_hybrid);
        assert!(opts.direct);
        assert!(opts.json);
        assert_eq!(opts.context, 5);
        assert!(opts.no_auto_index);
        assert_eq!(opts.file_types, vec!["rs", "py"]);
        assert_eq!(opts.exclude_file_types, vec!["test.rs"]);
        assert!(!opts.preprocess_query);
        assert!(opts.verbose);
    }

    #[test]
    fn test_search_args_file_types_are_cloned() {
        let args = SearchArgs {
            limit: 10,
            path: None,
            no_hybrid: false,
            auto_hybrid: false,
            rerank: false,
            direct: false,
            json: false,
            context: 2,
            no_auto_index: false,
            file_types: vec!["rs".to_string()],
            exclude_file_types: vec!["test".to_string()],
            no_preprocess_query: false,
            verbose: false,
            images: false,
        };
        let opts = SearchCliOptions::from(&args);
        // Verify the vectors are properly cloned (not just referenced)
        assert_eq!(opts.file_types.len(), 1);
        assert_eq!(opts.exclude_file_types.len(), 1);
        assert_eq!(opts.file_types[0], "rs");
        assert_eq!(opts.exclude_file_types[0], "test");
    }

    #[test]
    fn test_match_quality_from_score() {
        // Excellent: >= 0.8
        assert!(matches!(
            MatchQuality::from_score(1.0),
            MatchQuality::Excellent
        ));
        assert!(matches!(
            MatchQuality::from_score(0.9),
            MatchQuality::Excellent
        ));
        assert!(matches!(
            MatchQuality::from_score(0.8),
            MatchQuality::Excellent
        ));

        // Good: 0.6 - 0.79
        assert!(matches!(MatchQuality::from_score(0.79), MatchQuality::Good));
        assert!(matches!(MatchQuality::from_score(0.7), MatchQuality::Good));
        assert!(matches!(MatchQuality::from_score(0.6), MatchQuality::Good));

        // Fair: 0.4 - 0.59
        assert!(matches!(MatchQuality::from_score(0.59), MatchQuality::Fair));
        assert!(matches!(MatchQuality::from_score(0.5), MatchQuality::Fair));
        assert!(matches!(MatchQuality::from_score(0.4), MatchQuality::Fair));

        // Relevant: < 0.4
        assert!(matches!(
            MatchQuality::from_score(0.39),
            MatchQuality::Relevant
        ));
        assert!(matches!(
            MatchQuality::from_score(0.2),
            MatchQuality::Relevant
        ));
        assert!(matches!(
            MatchQuality::from_score(0.0),
            MatchQuality::Relevant
        ));
    }

    #[test]
    fn test_match_quality_labels() {
        assert_eq!(MatchQuality::Excellent.label(), "Excellent");
        assert_eq!(MatchQuality::Good.label(), "Good");
        assert_eq!(MatchQuality::Fair.label(), "Fair");
        assert_eq!(MatchQuality::Relevant.label(), "Relevant");
    }
}
