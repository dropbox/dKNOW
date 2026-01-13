//! Unix socket server for daemon IPC
//!
//! Handles JSON-RPC requests from CLI clients over Unix domain sockets.

use crate::config::Config;
use crate::project::{find_project_root, get_current_timestamp, ProjectManager};
use crate::protocol::{ProjectInfo, Request, Response, SearchResultWire};
use crate::throttle::{ThrottleConfig, Throttler};
use crate::watcher::{FileEvent, FileEventKind, FileWatcher};
use anyhow::{Context, Result};
use sg_core::{
    index_file_backend, load_embedder_from_env, search_backend, BackendEmbedder, EmbedderBackend,
    SearchOptions, DB,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;

/// Shared state for the daemon
/// Note: DB and BackendEmbedder are wrapped in Mutex because rusqlite::Connection
/// is not Send+Sync, and we need thread-safe access from async handlers.
pub struct DaemonState {
    pub db: Mutex<DB>,
    pub embedder: Mutex<BackendEmbedder>,
    pub watcher: Mutex<FileWatcher>,
    pub projects: Mutex<ProjectManager>,
    pub throttler: Throttler,
    pub start_time: Instant,
    /// Maximum storage in bytes (from config)
    pub max_storage_bytes: u64,
}

/// Unix socket server for IPC
pub struct Server {
    listener: UnixListener,
    state: Arc<DaemonState>,
}

/// Interval for polling file system events (100ms)
const WATCHER_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Number of watcher ticks between storage limit checks (~60 seconds at 100ms interval)
const STORAGE_CHECK_INTERVAL_TICKS: u64 = 600;

/// Default index quality estimate for indexed projects (0.0 to 1.0)
/// Used when actual quality metrics aren't available
const DEFAULT_INDEX_QUALITY: f32 = 0.8;

impl Server {
    /// Create a new server bound to the given socket path
    pub fn new(socket_path: &Path, db_path: &Path, config: &Config) -> Result<Self> {
        // Remove stale socket file if it exists
        if socket_path.exists() {
            std::fs::remove_file(socket_path).context("Failed to remove stale socket")?;
        }

        // Ensure parent directory exists
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create socket directory")?;
        }

        // Bind to socket
        let listener = UnixListener::bind(socket_path).context("Failed to bind to Unix socket")?;

        // Set socket permissions (owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(socket_path, perms)
                .context("Failed to set socket permissions")?;
        }

        tracing::info!("Listening on {:?}", socket_path);

        // Open database
        let db = DB::new(db_path).context("Failed to open database")?;

        tracing::info!("Loading embedding model from environment...");
        let mut embedder = load_embedder_from_env()?;
        // Warm up model to initialize GPU kernels
        embedder.warmup()?;
        tracing::info!(
            "Model loaded (backend={}, model={})",
            embedder.kind().as_str(),
            embedder.model().name()
        );

        // Create file watcher
        let watcher = FileWatcher::new().context("Failed to create file watcher")?;

        // Create project manager and run initial discovery
        let mut projects = if let Some(stale_threshold_secs) = config.stale_threshold_secs() {
            ProjectManager::with_limits(100, stale_threshold_secs)
        } else {
            ProjectManager::new()
        };
        projects.run_discovery();
        tracing::info!("Discovered {} projects", projects.count());

        let mut throttle_config = ThrottleConfig::default();
        if let Some(idle_threshold_secs) = config.idle_threshold_secs() {
            throttle_config.idle_threshold_secs =
                idle_threshold_secs.max(throttle_config.recent_activity_threshold_secs);
        }

        let max_storage_bytes = config.max_total_bytes();
        tracing::info!("Storage limit: {} MB", max_storage_bytes / (1024 * 1024));

        let state = Arc::new(DaemonState {
            db: Mutex::new(db),
            embedder: Mutex::new(embedder),
            watcher: Mutex::new(watcher),
            projects: Mutex::new(projects),
            throttler: Throttler::with_config(throttle_config),
            start_time: Instant::now(),
            max_storage_bytes,
        });

        Ok(Self { listener, state })
    }

    /// Run the server event loop
    pub async fn run(&self) -> Result<()> {
        tracing::info!("Server ready, accepting connections");

        // Spawn background task for file watching
        let watcher_state = Arc::clone(&self.state);
        tokio::spawn(async move {
            run_watcher_loop(watcher_state).await;
        });

        loop {
            match self.listener.accept().await {
                Ok((stream, _addr)) => {
                    let state = Arc::clone(&self.state);
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, state).await {
                            tracing::error!("Client handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Accept error: {}", e);
                }
            }
        }
    }
}

/// Background task that polls for file system events and processes them
async fn run_watcher_loop(state: Arc<DaemonState>) {
    let mut interval = tokio::time::interval(WATCHER_POLL_INTERVAL);
    let mut tick_count: u64 = 0;

    loop {
        interval.tick().await;
        tick_count += 1;

        // Periodically check storage limits
        if tick_count.is_multiple_of(STORAGE_CHECK_INTERVAL_TICKS) {
            enforce_storage_limits(&state).await;
        }

        // Poll for events (need to acquire lock briefly)
        let events = {
            let mut watcher = state.watcher.lock().await;
            watcher.poll_events()
        };

        if events.is_empty() {
            continue;
        }

        // Get current throttle limits
        let limits = state.throttler.get_limits();
        tracing::debug!(
            "Processing {} file events (throttle: {}ms delay, batch {})",
            events.len(),
            limits.min_delay_ms,
            limits.batch_size
        );

        // Process events in batches with throttle delays
        for (i, event) in events.iter().enumerate() {
            if let Err(e) = process_file_event(event, &state).await {
                tracing::warn!("Failed to process event {:?}: {}", event.path, e);
            }

            // Apply throttle delay between batches
            if (i + 1) % limits.batch_size == 0 && i + 1 < events.len() {
                tokio::time::sleep(Duration::from_millis(limits.min_delay_ms)).await;
            }
        }
    }
}

/// Check storage limits and evict LRU projects if over limit
async fn enforce_storage_limits(state: &DaemonState) {
    let db = state.db.lock().await;

    let storage_bytes = match db.storage_bytes() {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!("Failed to get storage size: {}", e);
            return;
        }
    };

    if storage_bytes <= state.max_storage_bytes {
        return; // Under limit, nothing to do
    }

    let over_by = storage_bytes.saturating_sub(state.max_storage_bytes);
    tracing::info!(
        "Storage limit exceeded: {} MB used, {} MB limit, {} MB over",
        storage_bytes / (1024 * 1024),
        state.max_storage_bytes / (1024 * 1024),
        over_by / (1024 * 1024)
    );

    // Get projects sorted by LRU (evict_lru returns paths of evicted projects)
    drop(db); // Release DB lock before acquiring projects lock

    let mut projects = state.projects.lock().await;
    let evicted = projects.evict_lru();
    drop(projects);

    if evicted.is_empty() {
        tracing::warn!("Storage limit exceeded but no projects available for eviction");
        return;
    }

    // Remove indexed documents for evicted projects
    let db = state.db.lock().await;
    for path in &evicted {
        if let Err(e) = db.remove_documents_by_prefix(path) {
            tracing::warn!("Failed to remove documents for {:?}: {}", path, e);
        } else {
            tracing::info!("Evicted project {:?} from index due to storage limit", path);
        }
    }
}

/// Process a single file event
async fn process_file_event(event: &FileEvent, state: &DaemonState) -> Result<()> {
    match event.kind {
        FileEventKind::Created | FileEventKind::Modified => {
            // Re-index the file
            let db = state.db.lock().await;
            let mut embedder = state.embedder.lock().await;

            match index_file_backend(&db, &mut *embedder, &event.path) {
                Ok(Some(doc_id)) => {
                    tracing::info!("Indexed {:?} (doc_id={})", event.path, doc_id);
                }
                Ok(None) => {
                    tracing::debug!("Skipped {:?} (empty or unchanged)", event.path);
                }
                Err(e) => {
                    tracing::warn!("Failed to index {:?}: {}", event.path, e);
                }
            }
        }
        FileEventKind::Deleted => {
            // Remove from index
            let db = state.db.lock().await;
            if db.remove_document(&event.path)? {
                tracing::info!("Removed {:?} from index", event.path);
            }
        }
    }
    Ok(())
}

/// Handle a single client connection
async fn handle_client(stream: UnixStream, state: Arc<DaemonState>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Read one line (JSON request)
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        return Ok(()); // Client disconnected
    }

    // Parse request
    let request: Request = match serde_json::from_str(&line) {
        Ok(req) => req,
        Err(e) => {
            let response = Response::Error(format!("Invalid request: {e}"));
            let response_json = serde_json::to_string(&response)?;
            writer.write_all(response_json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            return Ok(());
        }
    };

    // Handle request
    let response = handle_request(request, &state).await;

    // Send response
    let response_json = serde_json::to_string(&response)?;
    writer.write_all(response_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;

    Ok(())
}

/// Handle a parsed request
async fn handle_request(request: Request, state: &DaemonState) -> Response {
    // Record user activity for throttling
    state.throttler.record_activity();

    match request {
        Request::Search { query, options } => handle_search(&query, options, state).await,
        Request::Status => handle_status(state).await,
        Request::Watch { path } => handle_watch(&path, state).await,
        Request::Unwatch { path } => handle_unwatch(&path, state).await,
        Request::ForceIndex { path } => handle_force_index(&path, state).await,
        Request::DetectRoot { path } => handle_detect_root(&path, state).await,
        Request::DiscoverProjects => handle_discover_projects(state).await,
        Request::ListProjects => handle_list_projects(state).await,
        Request::Shutdown => {
            tracing::info!("Shutdown requested");
            // The daemon will be terminated by the caller
            Response::Ok
        }
    }
}

/// Handle search request
async fn handle_search(query: &str, options: SearchOptions, state: &DaemonState) -> Response {
    let db = state.db.lock().await;
    let mut embedder = state.embedder.lock().await;

    match search_backend(&db, &mut *embedder, query, options) {
        Ok(results) => {
            let wire_results: Vec<SearchResultWire> = results
                .into_iter()
                .map(|r| SearchResultWire {
                    score: r.score,
                    path: r.path.to_string_lossy().to_string(),
                    line: r.line,
                    snippet: r.snippet,
                    header_context: r.header_context,
                    language: r.language,
                    links: r
                        .links
                        .into_iter()
                        .map(|link| crate::protocol::SearchResultLinkWire {
                            text: link.text,
                            target: link.target,
                            is_internal: link.is_internal,
                        })
                        .collect(),
                })
                .collect();
            Response::SearchResults(wire_results)
        }
        Err(e) => Response::Error(format!("Search failed: {e}")),
    }
}

/// Handle status request
async fn handle_status(state: &DaemonState) -> Response {
    let uptime = state.start_time.elapsed().as_secs();

    let now = get_current_timestamp();

    // Get document count and storage size from database
    let db = state.db.lock().await;
    let doc_count = db.document_count().unwrap_or(0);
    let storage_bytes = db.storage_bytes().unwrap_or(0);

    // Get watched paths
    let watcher = state.watcher.lock().await;
    let watched_paths: Vec<PathBuf> = watcher.watched_paths().to_vec();
    drop(watcher);

    // Build project status from watched paths
    let projects: Vec<crate::protocol::ProjectStatus> = watched_paths
        .iter()
        .map(|path| {
            let stats = db
                .project_stats(path)
                .unwrap_or(sg_core::storage::ProjectStats {
                    file_count: 0,
                    last_indexed: None,
                });
            crate::protocol::ProjectStatus {
                path: path.to_string_lossy().to_string(),
                file_count: stats.file_count,
                last_indexed_secs_ago: stats
                    .last_indexed
                    .map(|ts| now.saturating_sub(ts))
                    .unwrap_or(0),
                quality: if stats.file_count > 0 {
                    DEFAULT_INDEX_QUALITY
                } else {
                    0.0
                },
            }
        })
        .collect();

    Response::Status(crate::protocol::DaemonStatus {
        uptime_secs: uptime,
        projects,
        storage_bytes,
        index_quality: if doc_count > 0 {
            DEFAULT_INDEX_QUALITY
        } else {
            0.0
        },
        throttle_state: state.throttler.state_description().to_string(),
    })
}

/// Handle watch request - start watching a directory for changes
async fn handle_watch(path: &Path, state: &DaemonState) -> Response {
    let mut watcher = state.watcher.lock().await;

    match watcher.watch(path) {
        Ok(()) => {
            tracing::info!("Now watching {:?}", path);
            // Register with project manager
            let mut projects = state.projects.lock().await;
            projects.add_project(path.to_path_buf());
            projects.set_watching(path, true);
            Response::Ok
        }
        Err(e) => {
            tracing::error!("Failed to watch {:?}: {}", path, e);
            Response::Error(format!("Failed to watch: {e}"))
        }
    }
}

/// Handle unwatch request - stop watching a directory
async fn handle_unwatch(path: &Path, state: &DaemonState) -> Response {
    let mut watcher = state.watcher.lock().await;

    match watcher.unwatch(path) {
        Ok(()) => {
            tracing::info!("Stopped watching {:?}", path);
            // Update project manager
            let mut projects = state.projects.lock().await;
            projects.set_watching(path, false);
            Response::Ok
        }
        Err(e) => {
            tracing::error!("Failed to unwatch {:?}: {}", path, e);
            Response::Error(format!("Failed to unwatch: {e}"))
        }
    }
}

/// Handle force index request
async fn handle_force_index(path: &Path, state: &DaemonState) -> Response {
    let db = state.db.lock().await;
    let mut embedder = state.embedder.lock().await;

    match sg_core::index_directory_backend(&db, &mut *embedder, path, None) {
        Ok(stats) => {
            tracing::info!(
                "Indexed {} files ({} lines) from {:?}",
                stats.indexed_files,
                stats.total_lines,
                path
            );
            Response::Ok
        }
        Err(e) => Response::Error(format!("Index failed: {e}")),
    }
}

/// Handle detect root request - find project root from a path
async fn handle_detect_root(path: &Path, state: &DaemonState) -> Response {
    match find_project_root(path) {
        Some(root) => {
            // Add to project manager
            let mut projects = state.projects.lock().await;
            projects.add_project(root.clone());
            Response::ProjectRoot(Some(root.to_string_lossy().to_string()))
        }
        None => Response::ProjectRoot(None),
    }
}

/// Handle discover projects request
async fn handle_discover_projects(state: &DaemonState) -> Response {
    let mut projects = state.projects.lock().await;
    let before = projects.count();
    projects.run_discovery();
    let after = projects.count();

    tracing::info!(
        "Discovery complete: {} projects (added {})",
        after,
        after.saturating_sub(before)
    );
    Response::Ok
}

/// Handle list projects request
async fn handle_list_projects(state: &DaemonState) -> Response {
    let projects = state.projects.lock().await;
    let now = get_current_timestamp();

    let project_list: Vec<ProjectInfo> = projects
        .all_projects()
        .iter()
        .map(|p| ProjectInfo {
            path: p.path.to_string_lossy().to_string(),
            project_type: format!("{:?}", p.project_type),
            is_watching: p.is_watching,
            last_accessed_secs_ago: now.saturating_sub(p.last_accessed),
        })
        .collect();

    Response::Projects(project_list)
}

/// Get the default socket path
pub fn default_socket_path() -> PathBuf {
    // macOS: ~/Library/Application Support/sg/daemon.sock
    // Linux: ~/.local/share/sg/daemon.sock
    directories::ProjectDirs::from("", "", "sg")
        .map(|dirs| dirs.data_dir().join("daemon.sock"))
        .unwrap_or_else(|| PathBuf::from("/tmp/sg-daemon.sock"))
}

/// Get the default database path
pub fn default_db_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "sg")
        .map(|dirs| dirs.data_dir().join("index.db"))
        .unwrap_or_else(|| PathBuf::from("/tmp/sg-index.db"))
}

/// Get the default PID file path
pub fn default_pid_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "sg")
        .map(|dirs| dirs.data_dir().join("daemon.pid"))
        .unwrap_or_else(|| PathBuf::from("/tmp/sg-daemon.pid"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_poll_interval_is_reasonable() {
        // Poll interval should be between 10ms and 1s
        assert!(WATCHER_POLL_INTERVAL.as_millis() >= 10);
        assert!(WATCHER_POLL_INTERVAL.as_millis() <= 1000);
        // Default is 100ms
        assert_eq!(WATCHER_POLL_INTERVAL, Duration::from_millis(100));
    }

    #[test]
    fn test_default_index_quality_is_valid() {
        // Quality should be 0.8 (between 0.0 and 1.0)
        assert!((DEFAULT_INDEX_QUALITY - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_default_socket_path_is_valid() {
        let path = default_socket_path();
        // Should end with daemon.sock
        assert!(path.ends_with("daemon.sock"));
        // Should have a parent directory
        assert!(path.parent().is_some());
    }

    #[test]
    fn test_default_db_path_is_valid() {
        let path = default_db_path();
        // Should end with index.db
        assert!(path.ends_with("index.db"));
        // Should have a parent directory
        assert!(path.parent().is_some());
    }

    #[test]
    fn test_default_pid_path_is_valid() {
        let path = default_pid_path();
        // Should end with daemon.pid
        assert!(path.ends_with("daemon.pid"));
        // Should have a parent directory
        assert!(path.parent().is_some());
    }

    #[test]
    fn test_default_paths_are_consistent() {
        let socket = default_socket_path();
        let db = default_db_path();
        let pid = default_pid_path();

        // All paths should be in the same directory
        let socket_dir = socket.parent().unwrap();
        let db_dir = db.parent().unwrap();
        let pid_dir = pid.parent().unwrap();

        assert_eq!(socket_dir, db_dir, "socket and db should be in same dir");
        assert_eq!(db_dir, pid_dir, "db and pid should be in same dir");
    }

    #[test]
    fn test_default_paths_are_absolute() {
        let socket = default_socket_path();
        let db = default_db_path();
        let pid = default_pid_path();

        // All paths should be absolute
        assert!(socket.is_absolute(), "socket path should be absolute");
        assert!(db.is_absolute(), "db path should be absolute");
        assert!(pid.is_absolute(), "pid path should be absolute");
    }

    #[test]
    fn test_default_paths_contain_sg_directory() {
        let socket = default_socket_path();
        let db = default_db_path();
        let pid = default_pid_path();

        // Parent directory should contain "sg"
        let socket_parent = socket.parent().unwrap().to_string_lossy();
        let db_parent = db.parent().unwrap().to_string_lossy();
        let pid_parent = pid.parent().unwrap().to_string_lossy();

        assert!(
            socket_parent.contains("sg"),
            "socket parent should contain 'sg'"
        );
        assert!(db_parent.contains("sg"), "db parent should contain 'sg'");
        assert!(pid_parent.contains("sg"), "pid parent should contain 'sg'");
    }

    #[test]
    fn test_default_paths_have_correct_extensions() {
        let socket = default_socket_path();
        let db = default_db_path();
        let pid = default_pid_path();

        // Verify file extensions
        assert_eq!(socket.extension().unwrap(), "sock");
        assert_eq!(db.extension().unwrap(), "db");
        assert_eq!(pid.extension().unwrap(), "pid");
    }
}
