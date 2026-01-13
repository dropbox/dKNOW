//! Video Extract CLI - High-performance media processing tool
//!
//! Command-line interface for the plugin-based media processing system.

use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use rayon::ThreadPoolBuilder;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod commands;
mod parser;

use commands::bulk::BulkCommand;
use commands::debug::DebugCommand;
use commands::fast::FastCommand;

#[derive(Parser)]
#[command(
    name = "video-extract",
    version,
    about = "High-performance media processing for AI workflows",
    long_about = "Extract audio, video frames, keyframes, transcriptions, and more from media files.\n\
                  Powered by a plugin-based architecture inspired by Dropbox Riviera.\n\n\
                  Three execution modes:\n  \
                  - fast: Single file, minimize latency (zero-copy, maximum speed)\n  \
                  - bulk: Multiple files, maximize efficiency per core (parallel processing)\n  \
                  - debug: Inspection mode with verbose logging and intermediate outputs",
    after_help = "EXAMPLES:\n  \
                  # List all available plugins\n  \
                  video-extract plugins\n\n  \
                  # FAST MODE - Single file, maximum speed (zero-copy)\n  \
                  video-extract fast --op keyframes video.mp4\n  \
                  video-extract fast --op keyframes+detect video.mp4  # Zero-copy detection\n  \
                  video-extract fast --op audio video.mp4\n  \
                  video-extract fast --op transcription video.mp4\n  \
                  video-extract fast --op metadata video.mp4  # Extract metadata\n  \
                  video-extract fast --op scene-detection video.mp4  # Detect scene changes\n\n  \
                  # BULK MODE - Multiple files, parallel processing\n  \
                  video-extract bulk --ops audio,transcription *.mp4\n  \
                  video-extract bulk --ops keyframes file1.mp4 file2.mp4 file3.mp4\n  \
                  video-extract bulk --ops \"[audio,keyframes]\" *.mp4  # Parallel operations\n\n  \
                  # DEBUG MODE - Inspection with verbose output\n  \
                  video-extract debug --ops audio,transcription video.mp4\n  \
                  video-extract debug --ops keyframes --max-frames 10 video.mp4\n  \
                  video-extract debug --ops audio --output-dir ./results video.mp4\n\n\
                  For more details on a specific command:\n  \
                  video-extract <COMMAND> --help"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Fast mode - Single file, minimize latency (zero-copy, maximum speed)
    Fast(FastCommand),

    /// Bulk mode - Multiple files, maximize efficiency per core (parallel processing)
    Bulk(BulkCommand),

    /// Debug mode - Inspection with verbose logging and intermediate outputs
    Debug(DebugCommand),

    /// List available plugins
    Plugins,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Configure Rayon thread pool based on environment variable
    // This allows tests to limit parallelism to avoid overwhelming the system
    if let Ok(threads_str) = std::env::var("VIDEO_EXTRACT_THREADS") {
        if let Ok(num_threads) = threads_str.parse::<usize>() {
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .ok(); // Ignore error if already initialized
        }
    }

    let cli = Cli::parse();

    // Initialize logging (suppress for plugins command to reduce noise)
    let log_level = match &cli.command {
        Commands::Plugins => Level::WARN, // Only show warnings/errors for clean output
        _ => {
            if cli.verbose {
                Level::DEBUG
            } else {
                Level::INFO
            }
        }
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    // Execute command
    match cli.command {
        Commands::Fast(cmd) => cmd.execute().await,
        Commands::Bulk(cmd) => cmd.execute().await,
        Commands::Debug(cmd) => cmd.execute().await,
        Commands::Plugins => commands::plugins::list_plugins().await,
    }
}
