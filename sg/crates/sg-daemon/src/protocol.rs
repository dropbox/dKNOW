//! IPC protocol for daemon communication
//!
//! This module defines the JSON-RPC protocol for client-daemon communication.

use serde::{Deserialize, Serialize};
use sg_core::SearchOptions;
use std::path::PathBuf;

/// Request from client to daemon
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// Search for documents matching query
    Search {
        query: String,
        options: SearchOptions,
    },
    /// Watch a directory for changes
    Watch { path: PathBuf },
    /// Stop watching a directory
    Unwatch { path: PathBuf },
    /// Force re-index a directory
    ForceIndex { path: PathBuf },
    /// Detect project root from path (walks up directory tree)
    DetectRoot { path: PathBuf },
    /// Run project discovery in common locations
    DiscoverProjects,
    /// List known projects
    ListProjects,
    /// Get daemon status
    Status,
    /// Shutdown daemon
    Shutdown,
}

/// Response from daemon to client
#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    /// Search results
    SearchResults(Vec<SearchResultWire>),
    /// Daemon status
    Status(DaemonStatus),
    /// Project list
    Projects(Vec<ProjectInfo>),
    /// Detected project root path
    ProjectRoot(Option<String>),
    /// Success with no data
    Ok,
    /// Error message
    Error(String),
}

/// Search result for wire protocol
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResultWire {
    pub score: f32,
    pub path: String,
    pub line: usize,
    pub snippet: String,
    #[serde(default)]
    pub header_context: String,
    /// Programming language for code blocks (e.g., "rust", "python")
    #[serde(default)]
    pub language: Option<String>,
    /// Links found in this chunk (markdown links, wiki-style links, etc.)
    #[serde(default)]
    pub links: Vec<SearchResultLinkWire>,
}

/// Link metadata for search results in the wire protocol
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResultLinkWire {
    pub text: String,
    pub target: String,
    pub is_internal: bool,
}

/// Daemon status information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub uptime_secs: u64,
    pub projects: Vec<ProjectStatus>,
    pub storage_bytes: u64,
    pub index_quality: f32,
    /// Current throttle state (e.g., "active (throttled)", "idle", "away (full speed)")
    #[serde(default)]
    pub throttle_state: String,
}

/// Project status information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectStatus {
    pub path: String,
    pub file_count: usize,
    pub last_indexed_secs_ago: u64,
    pub quality: f32,
}

/// Project information for listing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub path: String,
    pub project_type: String,
    pub is_watching: bool,
    pub last_accessed_secs_ago: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_wire_debug() {
        let result = SearchResultWire {
            score: 0.95,
            path: "/test/file.rs".to_string(),
            line: 42,
            snippet: "fn main()".to_string(),
            header_context: "# Test".to_string(),
            language: Some("rust".to_string()),
            links: vec![],
        };
        let debug_str = format!("{result:?}");
        assert!(debug_str.contains("SearchResultWire"));
        assert!(debug_str.contains("0.95"));
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn test_daemon_status_debug() {
        let status = DaemonStatus {
            uptime_secs: 3600,
            projects: vec![],
            storage_bytes: 1024,
            index_quality: 0.85,
            throttle_state: "idle".to_string(),
        };
        let debug_str = format!("{status:?}");
        assert!(debug_str.contains("DaemonStatus"));
        assert!(debug_str.contains("3600"));
        assert!(debug_str.contains("idle"));
    }

    #[test]
    fn test_daemon_status_default_throttle_state() {
        // Test that throttle_state defaults correctly when missing from JSON
        let json = r#"{
            "uptime_secs": 100,
            "projects": [],
            "storage_bytes": 0,
            "index_quality": 0.0
        }"#;
        let status: DaemonStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.throttle_state, ""); // serde default is empty string
    }

    #[test]
    fn test_project_status_debug() {
        let status = ProjectStatus {
            path: "/project".to_string(),
            file_count: 100,
            last_indexed_secs_ago: 60,
            quality: 0.9,
        };
        let debug_str = format!("{status:?}");
        assert!(debug_str.contains("ProjectStatus"));
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("0.9"));
    }

    #[test]
    fn test_project_info_debug() {
        let info = ProjectInfo {
            path: "/home/user/project".to_string(),
            project_type: "rust".to_string(),
            is_watching: true,
            last_accessed_secs_ago: 300,
        };
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("ProjectInfo"));
        assert!(debug_str.contains("rust"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_request_debug() {
        let request = Request::Status;
        let debug_str = format!("{request:?}");
        assert!(debug_str.contains("Status"));

        let request = Request::Shutdown;
        let debug_str = format!("{request:?}");
        assert!(debug_str.contains("Shutdown"));
    }

    #[test]
    fn test_response_debug() {
        let response = Response::Ok;
        let debug_str = format!("{response:?}");
        assert!(debug_str.contains("Ok"));

        let response = Response::Error("test error".to_string());
        let debug_str = format!("{response:?}");
        assert!(debug_str.contains("Error"));
        assert!(debug_str.contains("test error"));
    }

    #[test]
    fn test_search_result_wire_boundary_values() {
        // Test with boundary values
        let result = SearchResultWire {
            score: 0.0,
            path: "".to_string(),
            line: 0,
            snippet: "".to_string(),
            header_context: "".to_string(),
            language: None,
            links: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: SearchResultWire = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.score, 0.0);
        assert_eq!(parsed.line, 0);
        assert!(parsed.path.is_empty());

        // Test with max-ish values
        let result = SearchResultWire {
            score: 1.0,
            path: "a".repeat(1000),
            line: usize::MAX,
            snippet: "long snippet".repeat(100),
            header_context: "# Long Header".to_string(),
            language: Some("rust".to_string()),
            links: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: SearchResultWire = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.score, 1.0);
        assert_eq!(parsed.line, usize::MAX);
    }

    #[test]
    fn test_daemon_status_with_projects() {
        let status = DaemonStatus {
            uptime_secs: 86400,
            projects: vec![
                ProjectStatus {
                    path: "/project1".to_string(),
                    file_count: 50,
                    last_indexed_secs_ago: 10,
                    quality: 0.95,
                },
                ProjectStatus {
                    path: "/project2".to_string(),
                    file_count: 100,
                    last_indexed_secs_ago: 3600,
                    quality: 0.8,
                },
            ],
            storage_bytes: 1024 * 1024 * 100,
            index_quality: 0.87,
            throttle_state: "active (throttled)".to_string(),
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: DaemonStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.projects.len(), 2);
        assert_eq!(parsed.projects[0].file_count, 50);
        assert_eq!(parsed.projects[1].file_count, 100);
    }

    #[test]
    fn test_request_with_path_special_chars() {
        // Test paths with special characters
        let request = Request::Watch {
            path: PathBuf::from("/path/with spaces/and-dashes/project"),
        };
        let json = serde_json::to_string(&request).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        match parsed {
            Request::Watch { path } => {
                assert!(path.to_string_lossy().contains("with spaces"));
            }
            _ => panic!("Expected Watch request"),
        }
    }

    #[test]
    fn test_response_error_with_special_chars() {
        let response = Response::Error("Error: 'connection' failed\n\twith details".to_string());
        let json = serde_json::to_string(&response).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        match parsed {
            Response::Error(msg) => {
                assert!(msg.contains("connection"));
                assert!(msg.contains("details"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_project_info_watching_states() {
        let watching = ProjectInfo {
            path: "/project".to_string(),
            project_type: "rust".to_string(),
            is_watching: true,
            last_accessed_secs_ago: 0,
        };
        let json = serde_json::to_string(&watching).unwrap();
        assert!(json.contains("true"));

        let not_watching = ProjectInfo {
            path: "/project".to_string(),
            project_type: "rust".to_string(),
            is_watching: false,
            last_accessed_secs_ago: 0,
        };
        let json = serde_json::to_string(&not_watching).unwrap();
        assert!(json.contains("false"));
    }

    #[test]
    fn test_request_variants_serde_roundtrip() {
        // Search request
        let req = Request::Search {
            query: "test query".to_string(),
            options: SearchOptions::default(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        match parsed {
            Request::Search { query, .. } => assert_eq!(query, "test query"),
            _ => panic!("Expected Search request"),
        }

        // Watch request
        let req = Request::Watch {
            path: PathBuf::from("/project/path"),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        match parsed {
            Request::Watch { path } => assert_eq!(path, PathBuf::from("/project/path")),
            _ => panic!("Expected Watch request"),
        }

        // Unwatch request
        let req = Request::Unwatch {
            path: PathBuf::from("/project/path"),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        match parsed {
            Request::Unwatch { path } => assert_eq!(path, PathBuf::from("/project/path")),
            _ => panic!("Expected Unwatch request"),
        }

        // ForceIndex request
        let req = Request::ForceIndex {
            path: PathBuf::from("/path/to/index"),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        match parsed {
            Request::ForceIndex { path } => assert_eq!(path, PathBuf::from("/path/to/index")),
            _ => panic!("Expected ForceIndex request"),
        }

        // DetectRoot request
        let req = Request::DetectRoot {
            path: PathBuf::from("/some/path"),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        match parsed {
            Request::DetectRoot { path } => assert_eq!(path, PathBuf::from("/some/path")),
            _ => panic!("Expected DetectRoot request"),
        }

        // DiscoverProjects request
        let req = Request::DiscoverProjects;
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Request::DiscoverProjects));

        // ListProjects request
        let req = Request::ListProjects;
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Request::ListProjects));

        // Status request
        let req = Request::Status;
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Request::Status));

        // Shutdown request
        let req = Request::Shutdown;
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Request::Shutdown));
    }

    #[test]
    fn test_response_variants_serde_roundtrip() {
        // SearchResults response
        let results = vec![SearchResultWire {
            score: 0.9,
            path: "/test.rs".to_string(),
            line: 10,
            snippet: "fn test()".to_string(),
            header_context: "".to_string(),
            language: None,
            links: vec![],
        }];
        let resp = Response::SearchResults(results);
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        match parsed {
            Response::SearchResults(r) => {
                assert_eq!(r.len(), 1);
                assert_eq!(r[0].score, 0.9);
            }
            _ => panic!("Expected SearchResults response"),
        }

        // Status response
        let status = DaemonStatus {
            uptime_secs: 100,
            projects: vec![],
            storage_bytes: 1024,
            index_quality: 0.85,
            throttle_state: "idle".to_string(),
        };
        let resp = Response::Status(status);
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        match parsed {
            Response::Status(s) => {
                assert_eq!(s.uptime_secs, 100);
                assert_eq!(s.index_quality, 0.85);
            }
            _ => panic!("Expected Status response"),
        }

        // Projects response
        let projects = vec![ProjectInfo {
            path: "/project".to_string(),
            project_type: "rust".to_string(),
            is_watching: true,
            last_accessed_secs_ago: 60,
        }];
        let resp = Response::Projects(projects);
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        match parsed {
            Response::Projects(p) => {
                assert_eq!(p.len(), 1);
                assert!(p[0].is_watching);
            }
            _ => panic!("Expected Projects response"),
        }

        // ProjectRoot response with Some path
        let resp = Response::ProjectRoot(Some("/detected/root".to_string()));
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        match parsed {
            Response::ProjectRoot(Some(p)) => assert_eq!(p, "/detected/root"),
            _ => panic!("Expected ProjectRoot response with path"),
        }

        // ProjectRoot response with None
        let resp = Response::ProjectRoot(None);
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Response::ProjectRoot(None)));

        // Ok response
        let resp = Response::Ok;
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Response::Ok));

        // Error response
        let resp = Response::Error("Something went wrong".to_string());
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        match parsed {
            Response::Error(msg) => assert_eq!(msg, "Something went wrong"),
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_search_result_wire_clone() {
        let original = SearchResultWire {
            score: 0.95,
            path: "/test/file.rs".to_string(),
            line: 42,
            snippet: "fn main()".to_string(),
            header_context: "# Test".to_string(),
            language: Some("rust".to_string()),
            links: vec![],
        };
        let cloned = original.clone();
        assert_eq!(cloned.score, 0.95);
        assert_eq!(cloned.path, "/test/file.rs");
        assert_eq!(cloned.line, 42);
        assert_eq!(cloned.snippet, "fn main()");
        assert_eq!(cloned.header_context, "# Test");
        assert_eq!(cloned.language, Some("rust".to_string()));
    }

    #[test]
    fn test_search_result_wire_partial_eq() {
        let result1 = SearchResultWire {
            score: 0.95,
            path: "/test/file.rs".to_string(),
            line: 42,
            snippet: "fn main()".to_string(),
            header_context: "".to_string(),
            language: None,
            links: vec![],
        };
        let result2 = SearchResultWire {
            score: 0.95,
            path: "/test/file.rs".to_string(),
            line: 42,
            snippet: "fn main()".to_string(),
            header_context: "".to_string(),
            language: None,
            links: vec![],
        };
        let result3 = SearchResultWire {
            score: 0.80,
            path: "/other/file.rs".to_string(),
            line: 10,
            snippet: "fn other()".to_string(),
            header_context: "".to_string(),
            language: None,
            links: vec![],
        };
        assert_eq!(result1, result2);
        assert_ne!(result1, result3);
    }

    #[test]
    fn test_daemon_status_clone() {
        let original = DaemonStatus {
            uptime_secs: 3600,
            projects: vec![ProjectStatus {
                path: "/project".to_string(),
                file_count: 100,
                last_indexed_secs_ago: 60,
                quality: 0.9,
            }],
            storage_bytes: 1024,
            index_quality: 0.85,
            throttle_state: "idle".to_string(),
        };
        let cloned = original.clone();
        assert_eq!(cloned.uptime_secs, 3600);
        assert_eq!(cloned.projects.len(), 1);
        assert_eq!(cloned.storage_bytes, 1024);
        assert_eq!(cloned.index_quality, 0.85);
        assert_eq!(cloned.throttle_state, "idle");
    }

    #[test]
    fn test_daemon_status_partial_eq() {
        let status1 = DaemonStatus {
            uptime_secs: 3600,
            projects: vec![],
            storage_bytes: 1024,
            index_quality: 0.85,
            throttle_state: "idle".to_string(),
        };
        let status2 = DaemonStatus {
            uptime_secs: 3600,
            projects: vec![],
            storage_bytes: 1024,
            index_quality: 0.85,
            throttle_state: "idle".to_string(),
        };
        let status3 = DaemonStatus {
            uptime_secs: 7200,
            projects: vec![],
            storage_bytes: 2048,
            index_quality: 0.90,
            throttle_state: "active".to_string(),
        };
        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }

    #[test]
    fn test_project_status_clone() {
        let original = ProjectStatus {
            path: "/project".to_string(),
            file_count: 100,
            last_indexed_secs_ago: 60,
            quality: 0.9,
        };
        let cloned = original.clone();
        assert_eq!(cloned.path, "/project");
        assert_eq!(cloned.file_count, 100);
        assert_eq!(cloned.last_indexed_secs_ago, 60);
        assert_eq!(cloned.quality, 0.9);
    }

    #[test]
    fn test_project_status_partial_eq() {
        let status1 = ProjectStatus {
            path: "/project".to_string(),
            file_count: 100,
            last_indexed_secs_ago: 60,
            quality: 0.9,
        };
        let status2 = ProjectStatus {
            path: "/project".to_string(),
            file_count: 100,
            last_indexed_secs_ago: 60,
            quality: 0.9,
        };
        let status3 = ProjectStatus {
            path: "/other".to_string(),
            file_count: 50,
            last_indexed_secs_ago: 120,
            quality: 0.8,
        };
        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }

    #[test]
    fn test_project_info_clone() {
        let original = ProjectInfo {
            path: "/home/user/project".to_string(),
            project_type: "rust".to_string(),
            is_watching: true,
            last_accessed_secs_ago: 300,
        };
        let cloned = original.clone();
        assert_eq!(cloned.path, "/home/user/project");
        assert_eq!(cloned.project_type, "rust");
        assert!(cloned.is_watching);
        assert_eq!(cloned.last_accessed_secs_ago, 300);
    }

    #[test]
    fn test_project_info_partial_eq() {
        let info1 = ProjectInfo {
            path: "/home/user/project".to_string(),
            project_type: "rust".to_string(),
            is_watching: true,
            last_accessed_secs_ago: 300,
        };
        let info2 = ProjectInfo {
            path: "/home/user/project".to_string(),
            project_type: "rust".to_string(),
            is_watching: true,
            last_accessed_secs_ago: 300,
        };
        let info3 = ProjectInfo {
            path: "/other/project".to_string(),
            project_type: "python".to_string(),
            is_watching: false,
            last_accessed_secs_ago: 600,
        };
        assert_eq!(info1, info2);
        assert_ne!(info1, info3);
    }

    #[test]
    fn test_daemon_status_clone_with_nested_projects() {
        let original = DaemonStatus {
            uptime_secs: 86400,
            projects: vec![
                ProjectStatus {
                    path: "/project1".to_string(),
                    file_count: 50,
                    last_indexed_secs_ago: 10,
                    quality: 0.95,
                },
                ProjectStatus {
                    path: "/project2".to_string(),
                    file_count: 100,
                    last_indexed_secs_ago: 3600,
                    quality: 0.8,
                },
            ],
            storage_bytes: 1024 * 1024,
            index_quality: 0.87,
            throttle_state: "away (full speed)".to_string(),
        };
        let cloned = original.clone();
        assert_eq!(cloned.projects.len(), 2);
        assert_eq!(cloned.projects[0].path, "/project1");
        assert_eq!(cloned.projects[1].path, "/project2");
        assert_eq!(cloned.projects[0].file_count, 50);
        assert_eq!(cloned.projects[1].file_count, 100);
    }

    #[test]
    fn test_project_status_partial_eq_field_differences() {
        let base = ProjectStatus {
            path: "/project".to_string(),
            file_count: 100,
            last_indexed_secs_ago: 60,
            quality: 0.9,
        };

        // Different path
        let diff_path = ProjectStatus {
            path: "/other".to_string(),
            ..base.clone()
        };
        assert_ne!(base, diff_path);

        // Different file_count
        let diff_count = ProjectStatus {
            file_count: 200,
            ..base.clone()
        };
        assert_ne!(base, diff_count);

        // Different last_indexed_secs_ago
        let diff_indexed = ProjectStatus {
            last_indexed_secs_ago: 120,
            ..base.clone()
        };
        assert_ne!(base, diff_indexed);

        // Different quality
        let diff_quality = ProjectStatus {
            quality: 0.5,
            ..base.clone()
        };
        assert_ne!(base, diff_quality);
    }

    #[test]
    fn test_project_info_partial_eq_field_differences() {
        let base = ProjectInfo {
            path: "/project".to_string(),
            project_type: "rust".to_string(),
            is_watching: true,
            last_accessed_secs_ago: 300,
        };

        // Different path
        let diff_path = ProjectInfo {
            path: "/other".to_string(),
            ..base.clone()
        };
        assert_ne!(base, diff_path);

        // Different project_type
        let diff_type = ProjectInfo {
            project_type: "python".to_string(),
            ..base.clone()
        };
        assert_ne!(base, diff_type);

        // Different is_watching
        let diff_watching = ProjectInfo {
            is_watching: false,
            ..base.clone()
        };
        assert_ne!(base, diff_watching);

        // Different last_accessed_secs_ago
        let diff_accessed = ProjectInfo {
            last_accessed_secs_ago: 600,
            ..base.clone()
        };
        assert_ne!(base, diff_accessed);
    }
}
