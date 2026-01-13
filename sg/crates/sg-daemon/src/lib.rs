//! sg-daemon: Library for SuperGrep daemon
//!
//! This crate provides:
//! - Unix socket server for IPC
//! - Client library for communicating with the daemon
//! - Protocol types for client-daemon communication
//! - File system watcher for incremental indexing
//! - Project detection and auto-discovery

pub mod client;
pub mod config;
pub mod project;
pub mod protocol;
pub mod server;
pub mod throttle;
pub mod watcher;

// Re-exports for convenience
pub use client::{kill_daemon, read_daemon_pid, Client};
pub use project::{
    discover_projects, find_project_root, is_project_root, Project, ProjectManager, ProjectType,
};
pub use protocol::{
    DaemonStatus, ProjectInfo, ProjectStatus, Request, Response, SearchResultLinkWire,
    SearchResultWire,
};
pub use server::{default_db_path, default_pid_path, default_socket_path, Server};
pub use throttle::Throttler;
pub use watcher::{FileEvent, FileEventKind, FileWatcher};

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for re-exported types from client module
    #[test]
    fn test_client_reexports() {
        // Verify Client type is accessible
        let _client: Client = Client::with_default_socket();

        // Verify kill_daemon function is accessible
        // It takes &Path and returns anyhow::Result<bool>
        let pid_path = default_pid_path();
        let _result: anyhow::Result<bool> = kill_daemon(&pid_path);

        // Verify read_daemon_pid function is accessible
        // It takes &Path and returns Result<Option<u32>>
        let _result = read_daemon_pid(&pid_path);
    }

    // Tests for re-exported types from project module
    #[test]
    fn test_project_reexports() {
        // Verify ProjectType enum is accessible
        let _pt = ProjectType::Rust;
        let _pt2 = ProjectType::Unknown;

        // Verify Project struct fields are accessible
        let project = Project {
            path: std::path::PathBuf::from("/test"),
            project_type: ProjectType::Rust,
            last_accessed: 1234567890, // unix timestamp
            is_watching: false,
        };
        assert_eq!(project.path, std::path::PathBuf::from("/test"));
        assert!(!project.is_watching);
        assert_eq!(project.last_accessed, 1234567890);

        // Verify ProjectManager is accessible
        let _pm = ProjectManager::new();

        // Verify functions are accessible and return correct types
        // find_project_root takes &Path and returns Option<PathBuf>
        let root = find_project_root(std::path::Path::new("/nonexistent"));
        assert!(root.is_none());

        // is_project_root takes &Path and returns bool
        let is_root = is_project_root(std::path::Path::new("/nonexistent"));
        assert!(!is_root);

        // discover_projects returns Vec<Project>
        let projects: Vec<Project> = discover_projects();
        // Result may be empty or contain found projects
        let _ = projects;
    }

    // Tests for re-exported types from protocol module
    #[test]
    fn test_protocol_reexports() {
        // Verify Request enum variants are accessible
        let _req = Request::Status;
        let _req2 = Request::Shutdown;

        // Verify Response enum variants are accessible
        let _resp = Response::Ok;
        let _resp2 = Response::Error("test".to_string());

        // Verify DaemonStatus struct is accessible
        let status = DaemonStatus {
            uptime_secs: 100,
            projects: vec![],
            storage_bytes: 1024,
            index_quality: 0.9,
            throttle_state: "idle".to_string(),
        };
        assert_eq!(status.uptime_secs, 100);
        assert_eq!(status.index_quality, 0.9);

        // Verify ProjectInfo struct is accessible
        let info = ProjectInfo {
            path: "/test".to_string(),
            project_type: "rust".to_string(),
            is_watching: true,
            last_accessed_secs_ago: 60,
        };
        assert!(info.is_watching);

        // Verify ProjectStatus struct is accessible
        let ps = ProjectStatus {
            path: "/test".to_string(),
            file_count: 10,
            last_indexed_secs_ago: 30,
            quality: 0.95,
        };
        assert_eq!(ps.file_count, 10);

        // Verify SearchResultWire struct is accessible
        let result = SearchResultWire {
            score: 0.85,
            path: "/test/file.rs".to_string(),
            line: 42,
            snippet: "fn main()".to_string(),
            header_context: "# Test".to_string(),
            language: Some("rust".to_string()),
            links: vec![],
        };
        assert_eq!(result.score, 0.85);
        assert_eq!(result.line, 42);
        assert_eq!(result.language, Some("rust".to_string()));

        let link = SearchResultLinkWire {
            text: "Docs".to_string(),
            target: "./docs/README.md".to_string(),
            is_internal: true,
        };
        assert!(link.is_internal);
    }

    // Tests for re-exported types from server module
    #[test]
    fn test_server_reexports() {
        // Verify Server struct is accessible (can't easily instantiate without async runtime)
        // Just verify the type exists
        let _ = std::any::type_name::<Server>();

        // Verify path helper functions are accessible
        let socket_path = default_socket_path();
        let pid_path = default_pid_path();
        let db_path = default_db_path();

        // Paths should be non-empty
        assert!(!socket_path.as_os_str().is_empty());
        assert!(!pid_path.as_os_str().is_empty());
        assert!(!db_path.as_os_str().is_empty());
    }

    // Tests for re-exported types from throttle module
    #[test]
    fn test_throttle_reexports() {
        // Verify Throttler struct is accessible
        let throttler = Throttler::new();

        // Verify it has expected methods
        let _idle = throttler.idle_duration();

        let limits = throttler.get_limits();
        assert!(limits.batch_size >= 1);

        let state = throttler.state_description();
        assert!(!state.is_empty());
    }

    // Tests for re-exported types from watcher module
    #[test]
    fn test_watcher_reexports() {
        // Verify FileEventKind enum is accessible
        let _kind = FileEventKind::Created;
        let _kind2 = FileEventKind::Modified;
        let _kind3 = FileEventKind::Deleted;

        // Verify FileEvent struct is accessible
        let event = FileEvent {
            path: std::path::PathBuf::from("/test/file.rs"),
            kind: FileEventKind::Modified,
        };
        assert_eq!(event.path, std::path::PathBuf::from("/test/file.rs"));
        assert!(matches!(event.kind, FileEventKind::Modified));

        // Verify FileWatcher is accessible (can't instantiate without runtime)
        let _ = std::any::type_name::<FileWatcher>();
    }

    // Test that all expected items are re-exported at crate root
    #[test]
    fn test_all_reexports_present() {
        // This test verifies that the crate's public API is as expected
        // by importing all expected items

        // From client
        use crate::{kill_daemon, read_daemon_pid, Client};
        let _ = (
            kill_daemon as fn(&std::path::Path) -> anyhow::Result<bool>,
            read_daemon_pid as fn(&std::path::Path) -> anyhow::Result<Option<u32>>,
            Client::with_default_socket,
        );

        // From project
        use crate::{
            discover_projects, find_project_root, is_project_root, Project, ProjectManager,
            ProjectType,
        };
        let _ = (
            discover_projects as fn() -> Vec<Project>,
            find_project_root as fn(&std::path::Path) -> Option<std::path::PathBuf>,
            is_project_root as fn(&std::path::Path) -> bool,
            Project {
                path: std::path::PathBuf::new(),
                project_type: ProjectType::Unknown,
                last_accessed: 0,
                is_watching: false,
            },
            ProjectManager::new(),
        );

        // From protocol
        use crate::{
            DaemonStatus, ProjectInfo, ProjectStatus, Request, Response, SearchResultWire,
        };
        let _ = (
            DaemonStatus {
                uptime_secs: 0,
                projects: vec![],
                storage_bytes: 0,
                index_quality: 0.0,
                throttle_state: String::new(),
            },
            ProjectInfo {
                path: String::new(),
                project_type: String::new(),
                is_watching: false,
                last_accessed_secs_ago: 0,
            },
            ProjectStatus {
                path: String::new(),
                file_count: 0,
                last_indexed_secs_ago: 0,
                quality: 0.0,
            },
            Request::Status,
            Response::Ok,
            SearchResultWire {
                score: 0.0,
                path: String::new(),
                line: 0,
                snippet: String::new(),
                header_context: String::new(),
                language: None,
                links: vec![],
            },
        );

        // From server
        use crate::{default_db_path, default_pid_path, default_socket_path, Server};
        let _ = (default_db_path, default_pid_path, default_socket_path);
        let _ = std::any::type_name::<Server>();

        // From throttle
        use crate::Throttler;
        let _ = Throttler::new();

        // From watcher
        use crate::{FileEvent, FileEventKind, FileWatcher};
        let _ = (
            FileEvent {
                path: std::path::PathBuf::new(),
                kind: FileEventKind::Created,
            },
            std::any::type_name::<FileWatcher>(),
        );
    }
}
