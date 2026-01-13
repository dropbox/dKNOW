//! Integration tests for sg-core
//!
//! These tests verify end-to-end functionality:
//! - Index a directory
//! - Search and verify results
//! - Test incremental updates
//!
//! Run with: cargo test --test integration_test -- --ignored --nocapture

use sg_core::{
    index_directory_with_options, index_file, search, Embedder, IndexDirectoryOptions,
    SearchOptions, DB,
};
use std::fs;
use tempfile::TempDir;

/// Create test files with known content
fn create_test_files(dir: &std::path::Path) {
    // File about embeddings and ML
    fs::write(
        dir.join("embedder.rs"),
        r"
/// Generate embeddings for text using a transformer model
/// This module handles embedding generation using the XTR model
pub fn generate_embeddings(text: &str) -> Vec<f32> {
    // Uses T5 encoder with linear projection
    // Output is L2-normalized 128-dimensional vectors
    unimplemented!()
}

/// MaxSim scoring function for retrieval
pub fn maxsim(query: &[f32], doc: &[f32]) -> f32 {
    // Compute maximum similarity between query and document tokens
    0.0
}
",
    )
    .unwrap();

    // File about search and indexing
    fs::write(
        dir.join("search.rs"),
        r"
/// Search indexed documents using semantic similarity
/// Returns ranked results based on MaxSim scoring
pub fn search_documents(query: &str) -> Vec<SearchResult> {
    // 1. Embed the query
    // 2. Score against all documents
    // 3. Return top-k results
    vec![]
}

/// Index a directory of source code files
pub fn index_directory(path: &Path) {
    // Walk directory tree
    // Skip node_modules, target, .git
    // Embed each source file
}
",
    )
    .unwrap();

    // File about database storage
    fs::write(
        dir.join("storage.rs"),
        r"
/// SQLite storage for document embeddings
/// Uses hash-based change detection for incremental updates
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Store document content and metadata
    pub fn add_document(&self, path: &str, content: &str) -> u32 {
        // Compute SHA-256 hash for change detection
        // Insert into documents table
        0
    }

    /// Store embedding vectors as blob
    pub fn add_embeddings(&self, doc_id: u32, embeddings: &[f32]) {
        // Serialize embeddings to bytes
        // Insert into embeddings table
    }
}
",
    )
    .unwrap();

    // File about authentication (for varied content)
    fs::write(
        dir.join("auth.rs"),
        r"
/// User authentication and session management
pub struct Auth {
    secret_key: String,
}

impl Auth {
    /// Validate user credentials against database
    pub fn authenticate(username: &str, password: &str) -> Result<User, AuthError> {
        // Hash password with bcrypt
        // Compare against stored hash
        // Generate JWT token on success
        Err(AuthError::InvalidCredentials)
    }

    /// Verify JWT token signature
    pub fn verify_token(token: &str) -> Option<Claims> {
        None
    }
}
",
    )
    .unwrap();
}

#[test]
#[ignore] // Requires model download, run with: cargo test --test integration_test -- --ignored
fn test_index_and_search() {
    // Setup
    let test_dir = TempDir::new().unwrap();
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.db");

    create_test_files(test_dir.path());

    // Create DB and embedder
    let db = DB::new(&db_path).expect("Failed to create database");
    let device = sg_core::make_device();
    let mut embedder = Embedder::new(&device).expect("Failed to load embedder");

    // Index the test directory (allow temp paths for testing)
    let options = IndexDirectoryOptions {
        allow_temp_paths: true,
        ..Default::default()
    };
    let stats = index_directory_with_options(&db, &mut embedder, test_dir.path(), None, options)
        .expect("Failed to index directory");

    assert_eq!(stats.indexed_files, 4, "Should index 4 test files");
    assert_eq!(stats.failed_files, 0, "No files should fail");

    // Search for embedding-related content
    let results = search(
        &db,
        &mut embedder,
        "generate embeddings transformer model",
        SearchOptions {
            top_k: 4,
            ..Default::default()
        },
    )
    .expect("Search failed");

    assert!(
        !results.is_empty(),
        "Should find results for embeddings query"
    );

    // The embedder.rs file should be in top results
    let found_embedder = results.iter().any(|r| {
        r.path
            .file_name()
            .map(|n| n.to_string_lossy().contains("embedder"))
            .unwrap_or(false)
    });
    assert!(
        found_embedder,
        "embedder.rs should be in results for embedding query"
    );

    // Search for authentication-related content
    let auth_results = search(
        &db,
        &mut embedder,
        "user login password validation",
        SearchOptions {
            top_k: 4,
            ..Default::default()
        },
    )
    .expect("Search failed");

    assert!(
        !auth_results.is_empty(),
        "Should find results for auth query"
    );

    // The auth.rs file should be in top results
    let found_auth = auth_results.iter().any(|r| {
        r.path
            .file_name()
            .map(|n| n.to_string_lossy().contains("auth"))
            .unwrap_or(false)
    });
    assert!(found_auth, "auth.rs should be in results for auth query");

    // Search for database/storage content
    let storage_results = search(
        &db,
        &mut embedder,
        "SQLite database document storage",
        SearchOptions {
            top_k: 4,
            ..Default::default()
        },
    )
    .expect("Search failed");

    assert!(
        !storage_results.is_empty(),
        "Should find results for storage query"
    );

    let found_storage = storage_results.iter().any(|r| {
        r.path
            .file_name()
            .map(|n| n.to_string_lossy().contains("storage"))
            .unwrap_or(false)
    });
    assert!(
        found_storage,
        "storage.rs should be in results for storage query"
    );

    println!("Integration test passed!");
    println!("  Indexed {} files", stats.indexed_files);
    println!("  Total {} lines", stats.total_lines);
}

#[test]
#[ignore]
fn test_incremental_index() {
    // Setup
    let test_dir = TempDir::new().unwrap();
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.db");

    // Create initial file
    let test_file = test_dir.path().join("test.rs");
    fs::write(&test_file, "fn original() { /* original content */ }").unwrap();

    // Create DB and embedder
    let db = DB::new(&db_path).expect("Failed to create database");
    let device = sg_core::make_device();
    let mut embedder = Embedder::new(&device).expect("Failed to load embedder");

    // Index initial file
    let doc_id1 = index_file(&db, &mut embedder, &test_file)
        .expect("Failed to index file")
        .expect("Should return doc_id");

    // Index again without changes - should return same ID, not re-embed
    let doc_id2 = index_file(&db, &mut embedder, &test_file)
        .expect("Failed to index file")
        .expect("Should return doc_id");

    assert_eq!(doc_id1, doc_id2, "Same content should return same doc_id");

    // Modify file
    fs::write(
        &test_file,
        "fn modified() { /* new content with changes */ }",
    )
    .unwrap();

    // Index again - should re-embed
    let doc_id3 = index_file(&db, &mut embedder, &test_file)
        .expect("Failed to index file")
        .expect("Should return doc_id");

    // Doc ID should be the same (updated in place)
    assert_eq!(doc_id1, doc_id3, "Updated content should update same doc");

    // Verify the new content is searchable
    let results = search(
        &db,
        &mut embedder,
        "modified new changes",
        SearchOptions {
            top_k: 1,
            ..Default::default()
        },
    )
    .expect("Search failed");

    assert_eq!(results.len(), 1, "Should find the modified file");

    println!("Incremental index test passed!");
}

#[test]
#[ignore]
fn test_search_empty_index() {
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.db");

    let db = DB::new(&db_path).expect("Failed to create database");
    let device = sg_core::make_device();
    let mut embedder = Embedder::new(&device).expect("Failed to load embedder");

    let results = search(&db, &mut embedder, "anything", SearchOptions::default())
        .expect("Search should succeed on empty index");

    assert!(results.is_empty(), "Empty index should return no results");

    println!("Empty index search test passed!");
}
