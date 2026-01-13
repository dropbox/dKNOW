//! File system watcher for incremental indexing
//!
//! Provides debounced file watching for automatic re-indexing.

use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Default debounce duration (500ms)
const DEFAULT_DEBOUNCE_MS: u64 = 500;

/// File system watcher for tracking changes with debouncing
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    rx: mpsc::Receiver<Result<Event, notify::Error>>,
    watched_paths: Vec<PathBuf>,
    /// Pending events waiting for debounce timeout
    pending: HashMap<PathBuf, PendingEvent>,
    /// Debounce duration
    debounce: Duration,
}

/// A pending event waiting for debounce
struct PendingEvent {
    kind: FileEventKind,
    last_seen: Instant,
}

impl FileWatcher {
    /// Create a new file watcher with default debounce (500ms)
    pub fn new() -> Result<Self> {
        Self::with_debounce(Duration::from_millis(DEFAULT_DEBOUNCE_MS))
    }

    /// Create a new file watcher with custom debounce duration
    pub fn with_debounce(debounce: Duration) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(tx)?;

        Ok(Self {
            watcher,
            rx,
            watched_paths: Vec::new(),
            pending: HashMap::new(),
            debounce,
        })
    }

    /// Start watching a directory
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        // Don't add duplicates
        if self.watched_paths.contains(&canonical) {
            return Ok(());
        }

        self.watcher.watch(&canonical, RecursiveMode::Recursive)?;
        self.watched_paths.push(canonical);
        Ok(())
    }

    /// Stop watching a directory
    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !self.watched_paths.contains(&canonical) {
            return Ok(());
        }
        self.watcher.unwatch(&canonical)?;
        self.watched_paths.retain(|p| p != &canonical);
        Ok(())
    }

    /// Get watched directories
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }

    /// Poll for ready (debounced) file events
    ///
    /// This method:
    /// 1. Reads all pending raw events from the channel
    /// 2. Updates the pending event map with latest event types
    /// 3. Returns events that have exceeded the debounce timeout
    pub fn poll_events(&mut self) -> Vec<FileEvent> {
        let now = Instant::now();

        // 1. Process raw events from notify
        while let Ok(result) = self.rx.try_recv() {
            if let Ok(event) = result {
                self.process_raw_event(event, now);
            }
        }

        // 2. Collect events that have exceeded debounce timeout
        let mut ready = Vec::new();
        let mut still_pending = HashMap::new();

        for (path, pending) in self.pending.drain() {
            if now.duration_since(pending.last_seen) >= self.debounce {
                ready.push(FileEvent {
                    path,
                    kind: pending.kind,
                });
            } else {
                still_pending.insert(path, pending);
            }
        }

        self.pending = still_pending;
        ready
    }

    /// Check if there are any pending events
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Process a raw notify event into our pending map
    fn process_raw_event(&mut self, event: Event, now: Instant) {
        use notify::EventKind;

        let kind = match event.kind {
            EventKind::Create(_) => FileEventKind::Created,
            EventKind::Modify(_) => FileEventKind::Modified,
            EventKind::Remove(_) => FileEventKind::Deleted,
            _ => return,
        };

        for path in event.paths {
            // Skip if not indexable
            if !self.should_index(&path) {
                continue;
            }

            // Update pending map - later events override earlier ones
            // e.g., Create then Modify = Modified, Create then Delete = nothing
            if let Some(existing) = self.pending.get_mut(&path) {
                existing.kind = merge_event_kinds(existing.kind, kind);
                existing.last_seen = now;
            } else {
                self.pending.insert(
                    path,
                    PendingEvent {
                        kind,
                        last_seen: now,
                    },
                );
            }
        }
    }

    /// Check if a file should be indexed
    #[allow(clippy::unused_self)]
    fn should_index(&self, path: &Path) -> bool {
        // Must be a file (or a path that looks like a file)
        if path.is_dir() {
            return false;
        }

        // Skip system temp directories
        if sg_core::is_system_temp_path(path) {
            return false;
        }

        // Skip common non-source directories
        for component in path.components() {
            if let Some(name) = component.as_os_str().to_str() {
                if sg_core::should_skip_dir(name) {
                    return false;
                }
            }
        }

        sg_core::file_types::is_indexable_path(path)
    }
}

/// Merge two event kinds when the same file has multiple events
fn merge_event_kinds(old: FileEventKind, new: FileEventKind) -> FileEventKind {
    match (old, new) {
        // Created then deleted = no change (will be filtered out later)
        (FileEventKind::Created, FileEventKind::Deleted) => FileEventKind::Deleted,
        // Created then modified = created
        (FileEventKind::Created, FileEventKind::Modified) => FileEventKind::Created,
        // Deleted then created = modified
        (FileEventKind::Deleted, FileEventKind::Created) => FileEventKind::Modified,
        // Otherwise, latest wins
        (_, new) => new,
    }
}

/// A file system event
#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: FileEventKind,
}

/// Type of file system event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEventKind {
    Created,
    Modified,
    Deleted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_index_valid_extensions() {
        let watcher = FileWatcher::new().unwrap();

        // Should index these extensions
        assert!(watcher.should_index(Path::new("/test/file.rs")));
        assert!(watcher.should_index(Path::new("/test/file.py")));
        assert!(watcher.should_index(Path::new("/test/file.js")));
        assert!(watcher.should_index(Path::new("/test/file.ts")));
        assert!(watcher.should_index(Path::new("/test/file.go")));
        assert!(watcher.should_index(Path::new("/test/file.java")));
        assert!(watcher.should_index(Path::new("/test/Dockerfile")));
        assert!(watcher.should_index(Path::new("/test/Dockerfile.dev")));
        assert!(watcher.should_index(Path::new("/test/Makefile")));
        assert!(watcher.should_index(Path::new("/test/LICENSE-MIT")));
        assert!(watcher.should_index(Path::new("/test/README")));
        assert!(watcher.should_index(Path::new("/test/CHANGELOG")));
        assert!(watcher.should_index(Path::new("/test/file.md")));
        assert!(watcher.should_index(Path::new("/test/file.json")));
    }

    #[test]
    fn test_should_not_index_invalid_extensions() {
        let watcher = FileWatcher::new().unwrap();

        // Should not index these extensions (binaries/executables)
        assert!(!watcher.should_index(Path::new("/test/file.exe")));
        assert!(!watcher.should_index(Path::new("/test/file.bin")));
        assert!(!watcher.should_index(Path::new("/test/file.so")));
        assert!(!watcher.should_index(Path::new("/test/file.dll")));
        assert!(!watcher.should_index(Path::new("/test/file.o")));

        // Images: with clip feature they ARE indexable; without they're not
        #[cfg(not(feature = "clip"))]
        {
            assert!(!watcher.should_index(Path::new("/test/file.png")));
            assert!(!watcher.should_index(Path::new("/test/file.jpg")));
        }
        #[cfg(feature = "clip")]
        {
            assert!(watcher.should_index(Path::new("/test/file.png")));
            assert!(watcher.should_index(Path::new("/test/file.jpg")));
        }
    }

    #[test]
    fn test_should_handle_dotfiles() {
        let watcher = FileWatcher::new().unwrap();

        // Indexable dotfiles should be allowed
        assert!(watcher.should_index(Path::new("/test/.hidden.rs")));
        assert!(watcher.should_index(Path::new("/test/.gitignore")));

        // VCS metadata directories should be skipped
        assert!(!watcher.should_index(Path::new("/test/.git/config")));
    }

    #[test]
    fn test_should_skip_common_dirs() {
        let watcher = FileWatcher::new().unwrap();

        // Should skip node_modules, target, etc.
        assert!(!watcher.should_index(Path::new("/test/node_modules/pkg/index.js")));
        assert!(!watcher.should_index(Path::new("/test/target/debug/main.rs")));
        assert!(!watcher.should_index(Path::new("/test/vendor/lib.rs")));
        assert!(!watcher.should_index(Path::new("/test/__pycache__/mod.py")));
        assert!(!watcher.should_index(Path::new("/test/.idea/workspace.xml")));
        assert!(!watcher.should_index(Path::new("/test/.vscode/settings.json")));
    }

    #[test]
    fn test_should_skip_system_temp_dirs() {
        let watcher = FileWatcher::new().unwrap();

        // Should skip /tmp paths
        assert!(!watcher.should_index(Path::new("/tmp/some_file.rs")));
        assert!(!watcher.should_index(Path::new("/tmp/git.traces/file.json")));

        // Should skip /private/tmp paths (macOS)
        assert!(!watcher.should_index(Path::new("/private/tmp/file.rs")));

        // Should skip /var/tmp paths
        assert!(!watcher.should_index(Path::new("/var/tmp/file.rs")));

        // Should skip /var/folders paths (macOS temp folders)
        assert!(!watcher.should_index(Path::new("/var/folders/abc/def/T/file.rs")));
    }

    #[test]
    fn test_merge_event_kinds() {
        // Created then modified = created
        assert_eq!(
            merge_event_kinds(FileEventKind::Created, FileEventKind::Modified),
            FileEventKind::Created
        );

        // Created then deleted = deleted
        assert_eq!(
            merge_event_kinds(FileEventKind::Created, FileEventKind::Deleted),
            FileEventKind::Deleted
        );

        // Deleted then created = modified
        assert_eq!(
            merge_event_kinds(FileEventKind::Deleted, FileEventKind::Created),
            FileEventKind::Modified
        );

        // Modified then modified = modified
        assert_eq!(
            merge_event_kinds(FileEventKind::Modified, FileEventKind::Modified),
            FileEventKind::Modified
        );

        // Modified then deleted = deleted
        assert_eq!(
            merge_event_kinds(FileEventKind::Modified, FileEventKind::Deleted),
            FileEventKind::Deleted
        );
    }

    #[test]
    fn test_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());

        let watcher = watcher.unwrap();
        assert!(watcher.watched_paths().is_empty());
        assert!(!watcher.has_pending());
    }

    #[test]
    fn test_watcher_custom_debounce() {
        let watcher = FileWatcher::with_debounce(Duration::from_millis(100));
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watch_adds_path_to_watched_list() {
        let mut watcher = FileWatcher::new().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        assert!(watcher.watched_paths().is_empty());

        let result = watcher.watch(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(watcher.watched_paths().len(), 1);
    }

    #[test]
    fn test_watch_prevents_duplicate_paths() {
        let mut watcher = FileWatcher::new().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        // Watch the same path twice
        watcher.watch(temp_dir.path()).unwrap();
        watcher.watch(temp_dir.path()).unwrap();

        // Should only have one entry
        assert_eq!(watcher.watched_paths().len(), 1);
    }

    #[test]
    fn test_unwatch_removes_path_from_watched_list() {
        let mut watcher = FileWatcher::new().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        watcher.watch(temp_dir.path()).unwrap();
        assert_eq!(watcher.watched_paths().len(), 1);

        let result = watcher.unwatch(temp_dir.path());
        assert!(result.is_ok());
        assert!(watcher.watched_paths().is_empty());
    }

    #[test]
    fn test_unwatch_unknown_path_is_noop() {
        let mut watcher = FileWatcher::new().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let result = watcher.unwatch(temp_dir.path());
        assert!(result.is_ok());
        assert!(watcher.watched_paths().is_empty());
    }

    #[test]
    fn test_watch_multiple_paths() {
        let mut watcher = FileWatcher::new().unwrap();
        let temp_dir1 = tempfile::TempDir::new().unwrap();
        let temp_dir2 = tempfile::TempDir::new().unwrap();

        watcher.watch(temp_dir1.path()).unwrap();
        watcher.watch(temp_dir2.path()).unwrap();

        assert_eq!(watcher.watched_paths().len(), 2);
    }

    #[test]
    fn test_unwatch_only_removes_specified_path() {
        let mut watcher = FileWatcher::new().unwrap();
        let temp_dir1 = tempfile::TempDir::new().unwrap();
        let temp_dir2 = tempfile::TempDir::new().unwrap();

        watcher.watch(temp_dir1.path()).unwrap();
        watcher.watch(temp_dir2.path()).unwrap();
        assert_eq!(watcher.watched_paths().len(), 2);

        watcher.unwatch(temp_dir1.path()).unwrap();
        assert_eq!(watcher.watched_paths().len(), 1);

        // Verify the remaining path is temp_dir2
        let remaining = &watcher.watched_paths()[0];
        assert_eq!(
            remaining.canonicalize().unwrap(),
            temp_dir2.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_default_debounce_ms_is_reasonable() {
        // Default debounce should be 500ms (range: 100-2000ms is reasonable)
        assert_eq!(DEFAULT_DEBOUNCE_MS, 500);
    }

    #[test]
    fn test_file_event_debug() {
        let event = FileEvent {
            path: PathBuf::from("/test/file.rs"),
            kind: FileEventKind::Modified,
        };
        let debug_str = format!("{event:?}");
        assert!(debug_str.contains("FileEvent"));
        assert!(debug_str.contains("file.rs"));
        assert!(debug_str.contains("Modified"));
    }

    #[test]
    fn test_file_event_clone() {
        let event = FileEvent {
            path: PathBuf::from("/test/file.rs"),
            kind: FileEventKind::Created,
        };
        let cloned = event.clone();
        assert_eq!(cloned.path, PathBuf::from("/test/file.rs"));
        assert_eq!(cloned.kind, FileEventKind::Created);
    }

    #[test]
    fn test_file_event_kind_debug() {
        let created = FileEventKind::Created;
        let modified = FileEventKind::Modified;
        let deleted = FileEventKind::Deleted;

        assert_eq!(format!("{created:?}"), "Created");
        assert_eq!(format!("{modified:?}"), "Modified");
        assert_eq!(format!("{deleted:?}"), "Deleted");
    }

    #[test]
    fn test_file_event_kind_copy() {
        let kind = FileEventKind::Modified;
        let copied = kind; // Copy, not move
        assert_eq!(kind, copied);
        // Both can still be used - proves Copy works
        assert_eq!(kind, FileEventKind::Modified);
        assert_eq!(copied, FileEventKind::Modified);
    }

    #[test]
    fn test_file_event_kind_eq() {
        assert_eq!(FileEventKind::Created, FileEventKind::Created);
        assert_eq!(FileEventKind::Modified, FileEventKind::Modified);
        assert_eq!(FileEventKind::Deleted, FileEventKind::Deleted);

        assert_ne!(FileEventKind::Created, FileEventKind::Modified);
        assert_ne!(FileEventKind::Created, FileEventKind::Deleted);
        assert_ne!(FileEventKind::Modified, FileEventKind::Deleted);
    }
}
