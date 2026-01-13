//! Integration tests for the daemon
//!
//! These tests verify the daemon IPC protocol and client-server communication.

use sg_daemon::{default_socket_path, Client, Request, Response};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use tempfile::tempdir;

/// Test that client can connect and get an error response for invalid JSON
#[test]
#[ignore] // Requires running daemon
fn test_invalid_request() {
    let socket_path = default_socket_path();
    let client = Client::with_default_socket();

    // Check if daemon is actually running (not just if socket file exists)
    if !client.is_daemon_running() {
        eprintln!("Skipping test: daemon not running");
        return;
    }

    let mut stream = UnixStream::connect(&socket_path).expect("Failed to connect");

    // Send invalid JSON
    stream.write_all(b"not valid json\n").unwrap();
    stream.flush().unwrap();

    // Read response
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).unwrap();

    // Should be an error response
    let resp: Response = serde_json::from_str(&response).expect("Failed to parse response");
    match resp {
        Response::Error(msg) => {
            assert!(msg.contains("Invalid request"));
        }
        _ => panic!("Expected error response"),
    }
}

/// Test client status request
#[test]
#[ignore] // Requires running daemon
fn test_client_status() {
    let client = Client::with_default_socket();

    if !client.is_daemon_running() {
        eprintln!("Skipping test: daemon not running");
        return;
    }

    let status = client.status().expect("Failed to get status");

    // Basic validation (uptime_secs is u64, always >= 0)
    assert!(status.index_quality >= 0.0 && status.index_quality <= 1.0);
}

/// Test that protocol types serialize/deserialize correctly
#[test]
fn test_protocol_serde() {
    use sg_core::SearchOptions;

    // Test Request serialization
    let request = Request::Search {
        query: "test query".to_string(),
        options: SearchOptions {
            top_k: 10,
            threshold: 0.5,
            hybrid: false,
            root: Some(PathBuf::from("/test")),
            context: 2,
            file_types: vec![],
            exclude_file_types: vec![],
            ..SearchOptions::default()
        },
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize");
    let parsed: Request = serde_json::from_str(&json).expect("Failed to deserialize");

    match parsed {
        Request::Search { query, options } => {
            assert_eq!(query, "test query");
            assert_eq!(options.top_k, 10);
            assert_eq!(options.threshold, 0.5);
        }
        _ => panic!("Wrong request type"),
    }

    // Test Response serialization
    let response = Response::Status(sg_daemon::DaemonStatus {
        uptime_secs: 3600,
        projects: vec![],
        storage_bytes: 1024,
        index_quality: 0.9,
        throttle_state: "idle".to_string(),
    });

    let json = serde_json::to_string(&response).expect("Failed to serialize");
    let parsed: Response = serde_json::from_str(&json).expect("Failed to deserialize");

    match parsed {
        Response::Status(status) => {
            assert_eq!(status.uptime_secs, 3600);
            assert_eq!(status.index_quality, 0.9);
        }
        _ => panic!("Wrong response type"),
    }
}

/// Test serialization of all Request variants
#[test]
fn test_request_variants_serde() {
    use sg_core::SearchOptions;

    // Watch request
    let request = Request::Watch {
        path: PathBuf::from("/home/user/project"),
    };
    let json = serde_json::to_string(&request).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();
    match parsed {
        Request::Watch { path } => assert_eq!(path, PathBuf::from("/home/user/project")),
        _ => panic!("Expected Watch request"),
    }

    // Unwatch request
    let request = Request::Unwatch {
        path: PathBuf::from("/tmp/test"),
    };
    let json = serde_json::to_string(&request).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();
    match parsed {
        Request::Unwatch { path } => assert_eq!(path, PathBuf::from("/tmp/test")),
        _ => panic!("Expected Unwatch request"),
    }

    // ForceIndex request
    let request = Request::ForceIndex {
        path: PathBuf::from("/code/repo"),
    };
    let json = serde_json::to_string(&request).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();
    match parsed {
        Request::ForceIndex { path } => assert_eq!(path, PathBuf::from("/code/repo")),
        _ => panic!("Expected ForceIndex request"),
    }

    // DetectRoot request
    let request = Request::DetectRoot {
        path: PathBuf::from("/code/repo/src/main.rs"),
    };
    let json = serde_json::to_string(&request).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();
    match parsed {
        Request::DetectRoot { path } => {
            assert_eq!(path, PathBuf::from("/code/repo/src/main.rs"))
        }
        _ => panic!("Expected DetectRoot request"),
    }

    // DiscoverProjects request
    let request = Request::DiscoverProjects;
    let json = serde_json::to_string(&request).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, Request::DiscoverProjects));

    // ListProjects request
    let request = Request::ListProjects;
    let json = serde_json::to_string(&request).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, Request::ListProjects));

    // Status request
    let request = Request::Status;
    let json = serde_json::to_string(&request).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, Request::Status));

    // Shutdown request
    let request = Request::Shutdown;
    let json = serde_json::to_string(&request).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, Request::Shutdown));

    // Search request with all options
    let request = Request::Search {
        query: "find errors".to_string(),
        options: SearchOptions {
            top_k: 5,
            threshold: 0.7,
            hybrid: true,
            root: None,
            context: 3,
            file_types: vec!["rs".to_string(), "py".to_string()],
            exclude_file_types: vec!["md".to_string()],
            ..SearchOptions::default()
        },
    };
    let json = serde_json::to_string(&request).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();
    match parsed {
        Request::Search { query, options } => {
            assert_eq!(query, "find errors");
            assert_eq!(options.top_k, 5);
            assert!(options.hybrid);
            assert!(options.root.is_none());
            assert_eq!(options.file_types, vec!["rs", "py"]);
            assert_eq!(options.exclude_file_types, vec!["md"]);
        }
        _ => panic!("Expected Search request"),
    }
}

/// Test serialization of all Response variants
#[test]
fn test_response_variants_serde() {
    use sg_daemon::{ProjectInfo, ProjectStatus, SearchResultWire};

    // SearchResults response
    let response = Response::SearchResults(vec![
        SearchResultWire {
            score: 0.95,
            path: "/code/main.rs".to_string(),
            line: 42,
            snippet: "fn main() {".to_string(),
            header_context: "".to_string(),
            language: Some("rust".to_string()),
            links: vec![],
        },
        SearchResultWire {
            score: 0.85,
            path: "/code/lib.rs".to_string(),
            line: 10,
            snippet: "pub fn init()".to_string(),
            header_context: "".to_string(),
            language: None,
            links: vec![],
        },
    ]);
    let json = serde_json::to_string(&response).unwrap();
    let parsed: Response = serde_json::from_str(&json).unwrap();
    match parsed {
        Response::SearchResults(results) => {
            assert_eq!(results.len(), 2);
            assert_eq!(results[0].score, 0.95);
            assert_eq!(results[0].path, "/code/main.rs");
            assert_eq!(results[0].line, 42);
            assert_eq!(results[1].score, 0.85);
        }
        _ => panic!("Expected SearchResults response"),
    }

    // Status response with projects
    let response = Response::Status(sg_daemon::DaemonStatus {
        uptime_secs: 7200,
        projects: vec![
            ProjectStatus {
                path: "/project1".to_string(),
                file_count: 100,
                last_indexed_secs_ago: 60,
                quality: 0.95,
            },
            ProjectStatus {
                path: "/project2".to_string(),
                file_count: 50,
                last_indexed_secs_ago: 300,
                quality: 0.8,
            },
        ],
        storage_bytes: 1024 * 1024,
        index_quality: 0.87,
        throttle_state: "active (throttled)".to_string(),
    });
    let json = serde_json::to_string(&response).unwrap();
    let parsed: Response = serde_json::from_str(&json).unwrap();
    match parsed {
        Response::Status(status) => {
            assert_eq!(status.uptime_secs, 7200);
            assert_eq!(status.projects.len(), 2);
            assert_eq!(status.projects[0].file_count, 100);
            assert_eq!(status.storage_bytes, 1024 * 1024);
            assert_eq!(status.throttle_state, "active (throttled)");
        }
        _ => panic!("Expected Status response"),
    }

    // Projects response
    let response = Response::Projects(vec![
        ProjectInfo {
            path: "/home/user/code/project1".to_string(),
            project_type: "rust".to_string(),
            is_watching: true,
            last_accessed_secs_ago: 120,
        },
        ProjectInfo {
            path: "/home/user/code/project2".to_string(),
            project_type: "python".to_string(),
            is_watching: false,
            last_accessed_secs_ago: 86400,
        },
    ]);
    let json = serde_json::to_string(&response).unwrap();
    let parsed: Response = serde_json::from_str(&json).unwrap();
    match parsed {
        Response::Projects(projects) => {
            assert_eq!(projects.len(), 2);
            assert_eq!(projects[0].project_type, "rust");
            assert!(projects[0].is_watching);
            assert!(!projects[1].is_watching);
        }
        _ => panic!("Expected Projects response"),
    }

    // ProjectRoot response with Some
    let response = Response::ProjectRoot(Some("/home/user/code/project".to_string()));
    let json = serde_json::to_string(&response).unwrap();
    let parsed: Response = serde_json::from_str(&json).unwrap();
    match parsed {
        Response::ProjectRoot(root) => {
            assert_eq!(root, Some("/home/user/code/project".to_string()));
        }
        _ => panic!("Expected ProjectRoot response"),
    }

    // ProjectRoot response with None
    let response = Response::ProjectRoot(None);
    let json = serde_json::to_string(&response).unwrap();
    let parsed: Response = serde_json::from_str(&json).unwrap();
    match parsed {
        Response::ProjectRoot(root) => assert!(root.is_none()),
        _ => panic!("Expected ProjectRoot response"),
    }

    // Ok response
    let response = Response::Ok;
    let json = serde_json::to_string(&response).unwrap();
    let parsed: Response = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, Response::Ok));

    // Error response
    let response = Response::Error("Connection failed: timeout".to_string());
    let json = serde_json::to_string(&response).unwrap();
    let parsed: Response = serde_json::from_str(&json).unwrap();
    match parsed {
        Response::Error(msg) => assert_eq!(msg, "Connection failed: timeout"),
        _ => panic!("Expected Error response"),
    }
}

/// Test client helper functions
#[test]
fn test_client_helpers() {
    use std::time::Duration;

    // Test client creation with timeout chaining
    let _client = Client::with_default_socket().with_timeout(Duration::from_secs(10));

    // Verify is_daemon_running returns false when daemon isn't running
    // (assuming daemon is not running during unit tests)
    let temp = tempdir().unwrap();
    let fake_socket = temp.path().join("nonexistent.sock");
    let client = Client::new(&fake_socket);
    assert!(!client.is_daemon_running());
}
