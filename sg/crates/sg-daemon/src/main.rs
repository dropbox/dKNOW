//! sg-daemon: Background daemon for SuperGrep
//!
//! Provides:
//! - Unix socket server for IPC
//! - Background search service with loaded model
//! - File watching and incremental indexing (Phase 3)

use anyhow::{Context, Result};
use clap::Parser;
use sg_daemon::config::{default_config_path, load_config, Config};
use sg_daemon::{default_db_path, default_pid_path, default_socket_path, Server};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "sg-daemon")]
#[command(about = "SuperGrep daemon - background service for semantic search")]
#[command(version)]
struct Args {
    /// Run in foreground (don't daemonize)
    #[arg(long)]
    foreground: bool,

    /// Socket path
    #[arg(long)]
    socket: Option<PathBuf>,

    /// Database path
    #[arg(long)]
    db: Option<PathBuf>,

    /// PID file path
    #[arg(long)]
    pid: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config_path = default_config_path()?;
    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!(
                "Failed to load config from {}: {}. Using defaults.",
                config_path.display(),
                err
            );
            Config::default()
        }
    };

    let socket_path = args
        .socket
        .or_else(|| config.daemon_socket_path())
        .unwrap_or_else(default_socket_path);
    let db_path = args.db.unwrap_or_else(default_db_path);
    let pid_path = args.pid.unwrap_or_else(default_pid_path);

    if args.foreground {
        // Run in foreground with logging to stderr
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .init();
        run_daemon(&socket_path, &db_path, &pid_path, &config)
    } else {
        // Daemonize
        daemonize(&socket_path, &db_path, &pid_path, &config)
    }
}

/// Daemonize the process
fn daemonize(socket_path: &Path, db_path: &Path, pid_path: &Path, config: &Config) -> Result<()> {
    // Ensure parent directories exist
    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create PID directory")?;
    }

    // Fork the process
    match unsafe { libc::fork() } {
        -1 => anyhow::bail!("Fork failed"),
        0 => {
            // Child process - continue with daemonization
        }
        _ => {
            // Parent process - exit successfully
            std::process::exit(0);
        }
    }

    // Create new session
    if unsafe { libc::setsid() } == -1 {
        anyhow::bail!("setsid failed");
    }

    // Fork again to prevent terminal reacquisition
    match unsafe { libc::fork() } {
        -1 => anyhow::bail!("Second fork failed"),
        0 => {
            // Grandchild - this is the daemon
        }
        _ => {
            // Child - exit
            std::process::exit(0);
        }
    }

    // Change to root directory to avoid holding mount points
    std::env::set_current_dir("/").ok();

    // Redirect stdio to /dev/null
    let dev_null = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .context("Failed to open /dev/null")?;

    use std::os::unix::io::AsRawFd;
    unsafe {
        libc::dup2(dev_null.as_raw_fd(), libc::STDIN_FILENO);
        libc::dup2(dev_null.as_raw_fd(), libc::STDOUT_FILENO);
        libc::dup2(dev_null.as_raw_fd(), libc::STDERR_FILENO);
    }

    // Set up logging to syslog or file
    // For now, use a log file
    let log_dir = pid_path.parent().unwrap_or(std::path::Path::new("/tmp"));
    let log_path = log_dir.join("sg-daemon.log");

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .context("Failed to open log file")?;

    tracing_subscriber::fmt()
        .with_writer(std::sync::Mutex::new(log_file))
        .with_ansi(false)
        .init();

    run_daemon(socket_path, db_path, pid_path, config)
}

/// Run the daemon (either foreground or after daemonization)
fn run_daemon(socket_path: &Path, db_path: &Path, pid_path: &Path, config: &Config) -> Result<()> {
    // Write PID file
    let pid = std::process::id();
    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(pid_path, pid.to_string()).context("Failed to write PID file")?;

    tracing::info!("sg-daemon starting (pid: {})", pid);

    // Create tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;

    // Run the async main
    let result = rt.block_on(async_main(socket_path, db_path, pid_path, config));

    // Clean up PID file on exit
    std::fs::remove_file(pid_path).ok();

    result
}

/// Async main function
async fn async_main(
    socket_path: &Path,
    db_path: &Path,
    pid_path: &Path,
    config: &Config,
) -> Result<()> {
    // Set up signal handling
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;

    // Create server
    let server = Server::new(socket_path, db_path, config)?;

    // Run server with signal handling
    tokio::select! {
        result = server.run() => {
            result?;
        }
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM, shutting down");
        }
        _ = sigint.recv() => {
            tracing::info!("Received SIGINT, shutting down");
        }
        _ = sighup.recv() => {
            tracing::info!("Received SIGHUP, shutting down");
        }
    }

    // Clean up socket
    std::fs::remove_file(socket_path).ok();
    std::fs::remove_file(pid_path).ok();

    tracing::info!("sg-daemon stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_defaults() {
        let args = Args::parse_from(["sg-daemon"]);
        assert!(!args.foreground);
        assert!(args.socket.is_none());
        assert!(args.db.is_none());
        assert!(args.pid.is_none());
    }

    #[test]
    fn test_args_foreground_flag() {
        let args = Args::parse_from(["sg-daemon", "--foreground"]);
        assert!(args.foreground);
    }

    #[test]
    fn test_args_custom_paths() {
        let args = Args::parse_from([
            "sg-daemon",
            "--socket",
            "/tmp/custom.sock",
            "--db",
            "/tmp/custom.db",
            "--pid",
            "/tmp/custom.pid",
        ]);

        assert_eq!(args.socket.as_deref(), Some(Path::new("/tmp/custom.sock")));
        assert_eq!(args.db.as_deref(), Some(Path::new("/tmp/custom.db")));
        assert_eq!(args.pid.as_deref(), Some(Path::new("/tmp/custom.pid")));
    }
}
