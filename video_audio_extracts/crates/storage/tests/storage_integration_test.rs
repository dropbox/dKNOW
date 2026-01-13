//! Integration tests for storage backends
//!
//! These tests require live instances of `MinIO`, `PostgreSQL`, and Qdrant.
//! Start services with: `docker-compose up -d`
//!
//! Run tests with: `cargo test --package video-audio-storage --test storage_integration_test -- --ignored --nocapture`
//!
//! All tests are marked with #[ignore] to prevent running in CI without live services.

use std::collections::HashMap;
use video_audio_storage::*;

/// Check if `MinIO` is available
async fn is_minio_available() -> bool {
    tokio::net::TcpStream::connect("127.0.0.1:9000")
        .await
        .is_ok()
}

/// Check if `PostgreSQL` is available
async fn is_postgres_available() -> bool {
    tokio::net::TcpStream::connect("127.0.0.1:5432")
        .await
        .is_ok()
}

/// Check if Qdrant is available
async fn is_qdrant_available() -> bool {
    tokio::net::TcpStream::connect("127.0.0.1:6333")
        .await
        .is_ok()
}

// ============================================================================
// MinIO Object Storage Integration Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires MinIO running on localhost:9000
async fn test_minio_store_and_retrieve() {
    if !is_minio_available().await {
        eprintln!("MinIO not available on 127.0.0.1:9000");
        eprintln!("Start with: docker-compose up -d minio");
        eprintln!("Skipping test_minio_store_and_retrieve");
        return;
    }

    // Configure MinIO client
    let config = S3Config {
        bucket: "video-audio-extracts".to_string(),
        region: "us-east-1".to_string(),
        endpoint: Some("http://localhost:9000".to_string()),
        access_key_id: "minioadmin".to_string(),
        secret_access_key: "minioadmin".to_string(),
        prefix: "test/".to_string(),
    };

    let storage = S3ObjectStorage::new(config)
        .await
        .expect("Failed to create S3 storage client");

    // Test data
    let test_key = "test-file.txt";
    let test_data = b"Hello, MinIO! This is a test file.";

    // Store file
    let stored_key = storage
        .store_file(test_key, test_data)
        .await
        .expect("Failed to store file");
    assert_eq!(stored_key, format!("test/{test_key}"));

    // Retrieve file
    let retrieved_data = storage
        .retrieve_file(test_key)
        .await
        .expect("Failed to retrieve file");
    assert_eq!(retrieved_data, test_data);

    // Check file exists
    let exists = storage
        .file_exists(test_key)
        .await
        .expect("Failed to check file existence");
    assert!(exists, "File should exist");

    // Get file size
    let size = storage
        .get_file_size(test_key)
        .await
        .expect("Failed to get file size");
    assert_eq!(size, test_data.len() as u64);

    // Clean up
    storage
        .delete_file(test_key)
        .await
        .expect("Failed to delete file");

    // Verify deletion
    let exists_after_delete = storage
        .file_exists(test_key)
        .await
        .expect("Failed to check");
    assert!(!exists_after_delete, "File should not exist after deletion");

    println!("✅ MinIO integration test passed: store, retrieve, exists, size, delete");
}

#[tokio::test]
#[ignore] // Requires MinIO running on localhost:9000
async fn test_minio_list_files() {
    if !is_minio_available().await {
        eprintln!("MinIO not available on 127.0.0.1:9000");
        eprintln!("Skipping test_minio_list_files");
        return;
    }

    let config = S3Config {
        bucket: "video-audio-extracts".to_string(),
        region: "us-east-1".to_string(),
        endpoint: Some("http://localhost:9000".to_string()),
        access_key_id: "minioadmin".to_string(),
        secret_access_key: "minioadmin".to_string(),
        prefix: "test-list/".to_string(),
    };

    let storage = S3ObjectStorage::new(config)
        .await
        .expect("Failed to create S3 storage client");

    // Store multiple files
    let test_files = vec![
        ("file1.txt", b"content 1" as &[u8]),
        ("file2.txt", b"content 2"),
        ("file3.txt", b"content 3"),
    ];

    for (key, data) in &test_files {
        storage
            .store_file(key, data)
            .await
            .expect("Failed to store file");
    }

    // List files with prefix
    let listed_files = storage
        .list_files("test-list/")
        .await
        .expect("Failed to list files");

    assert_eq!(listed_files.len(), 3, "Should list 3 files");
    assert!(listed_files.contains(&"test-list/file1.txt".to_string()));
    assert!(listed_files.contains(&"test-list/file2.txt".to_string()));
    assert!(listed_files.contains(&"test-list/file3.txt".to_string()));

    // Clean up
    for (key, _) in &test_files {
        storage
            .delete_file(key)
            .await
            .expect("Failed to delete file");
    }

    println!("✅ MinIO integration test passed: list_files");
}

#[tokio::test]
#[ignore] // Requires MinIO running on localhost:9000
async fn test_minio_store_from_path() {
    if !is_minio_available().await {
        eprintln!("MinIO not available on 127.0.0.1:9000");
        eprintln!("Skipping test_minio_store_from_path");
        return;
    }

    let config = S3Config {
        bucket: "video-audio-extracts".to_string(),
        region: "us-east-1".to_string(),
        endpoint: Some("http://localhost:9000".to_string()),
        access_key_id: "minioadmin".to_string(),
        secret_access_key: "minioadmin".to_string(),
        prefix: "test-path/".to_string(),
    };

    let storage = S3ObjectStorage::new(config)
        .await
        .expect("Failed to create S3 storage client");

    // Create temporary file
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_file_path = temp_dir.path().join("test.txt");
    let test_data = b"File uploaded from path";
    std::fs::write(&temp_file_path, test_data).expect("Failed to write temp file");

    // Store from path
    let stored_key = storage
        .store_file_from_path("uploaded.txt", &temp_file_path)
        .await
        .expect("Failed to store file from path");
    assert_eq!(stored_key, "test-path/uploaded.txt");

    // Retrieve and verify
    let retrieved_data = storage
        .retrieve_file("uploaded.txt")
        .await
        .expect("Failed to retrieve file");
    assert_eq!(retrieved_data, test_data);

    // Clean up
    storage
        .delete_file("uploaded.txt")
        .await
        .expect("Failed to delete file");

    println!("✅ MinIO integration test passed: store_file_from_path");
}

// ============================================================================
// PostgreSQL Metadata Storage Integration Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires PostgreSQL running on localhost:5432
async fn test_postgres_schema_init() {
    if !is_postgres_available().await {
        eprintln!("PostgreSQL not available on 127.0.0.1:5432");
        eprintln!("Start with: docker-compose up -d postgres");
        eprintln!("Skipping test_postgres_schema_init");
        return;
    }

    let config = PostgresConfig {
        host: "localhost".to_string(),
        port: 5432,
        database: "video_audio_extracts".to_string(),
        user: "postgres".to_string(),
        password: "postgres".to_string(),
    };

    let storage = PostgresMetadataStorage::new(config)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Initialize schema (should create tables)
    storage
        .init_schema()
        .await
        .expect("Failed to initialize schema");

    // Re-initialize should be idempotent (no error)
    storage
        .init_schema()
        .await
        .expect("Schema initialization should be idempotent");

    println!("✅ PostgreSQL integration test passed: schema initialization");
}

#[tokio::test]
#[ignore] // Requires PostgreSQL running on localhost:5432
async fn test_postgres_media_metadata() {
    if !is_postgres_available().await {
        eprintln!("PostgreSQL not available on 127.0.0.1:5432");
        eprintln!("Skipping test_postgres_media_metadata");
        return;
    }

    let config = PostgresConfig {
        host: "localhost".to_string(),
        port: 5432,
        database: "video_audio_extracts".to_string(),
        user: "postgres".to_string(),
        password: "postgres".to_string(),
    };

    let storage = PostgresMetadataStorage::new(config)
        .await
        .expect("Failed to connect to PostgreSQL");

    storage
        .init_schema()
        .await
        .expect("Failed to initialize schema");

    // Create test metadata matching actual struct definition
    let metadata = MediaMetadata {
        job_id: "test-job-123".to_string(),
        input_path: "/path/to/test_video.mp4".to_string(),
        format: "mp4".to_string(),
        duration_secs: 60.0,
        num_streams: 2,
        resolution: Some((1920, 1080)),
        frame_rate: Some(30.0),
        sample_rate: Some(48000),
        audio_channels: Some(2),
        processed_at: chrono::Utc::now(),
        extra: HashMap::new(),
    };

    // Store metadata
    let stored_job_id = storage
        .store_media_metadata(&metadata)
        .await
        .expect("Failed to store media metadata");
    assert_eq!(stored_job_id, "test-job-123");

    // Retrieve metadata
    let retrieved = storage
        .get_media_metadata("test-job-123")
        .await
        .expect("Failed to retrieve media metadata");

    assert_eq!(retrieved.job_id, metadata.job_id);
    assert_eq!(retrieved.input_path, metadata.input_path);
    assert_eq!(retrieved.format, metadata.format);
    assert_eq!(retrieved.duration_secs, metadata.duration_secs);
    assert_eq!(retrieved.resolution, metadata.resolution);

    // Clean up
    storage
        .delete_job_data("test-job-123")
        .await
        .expect("Failed to delete job data");

    println!("✅ PostgreSQL integration test passed: media metadata CRUD");
}

#[tokio::test]
#[ignore] // Requires PostgreSQL running on localhost:5432
async fn test_postgres_transcription_segments() {
    if !is_postgres_available().await {
        eprintln!("PostgreSQL not available on 127.0.0.1:5432");
        eprintln!("Skipping test_postgres_transcription_segments");
        return;
    }

    let config = PostgresConfig {
        host: "localhost".to_string(),
        port: 5432,
        database: "video_audio_extracts".to_string(),
        user: "postgres".to_string(),
        password: "postgres".to_string(),
    };

    let storage = PostgresMetadataStorage::new(config)
        .await
        .expect("Failed to connect to PostgreSQL");

    storage
        .init_schema()
        .await
        .expect("Failed to initialize schema");

    let job_id = "test-transcription-job";

    // Create test segments matching actual struct definition
    let segments = vec![
        TranscriptionSegment {
            job_id: job_id.to_string(),
            segment_id: 0,
            start_time: 0.0,
            end_time: 2.5,
            text: "Hello, world!".to_string(),
            confidence: 0.95,
            speaker_id: Some("speaker_0".to_string()),
        },
        TranscriptionSegment {
            job_id: job_id.to_string(),
            segment_id: 1,
            start_time: 2.5,
            end_time: 5.0,
            text: "This is a test.".to_string(),
            confidence: 0.98,
            speaker_id: Some("speaker_1".to_string()),
        },
    ];

    // Store segments
    let stored_count = storage
        .store_transcription_segments(&segments)
        .await
        .expect("Failed to store transcription segments");
    assert_eq!(stored_count, 2);

    // Retrieve segments
    let retrieved = storage
        .get_transcription_segments(job_id)
        .await
        .expect("Failed to retrieve transcription segments");

    assert_eq!(retrieved.len(), 2);
    assert_eq!(retrieved[0].text, "Hello, world!");
    assert_eq!(retrieved[1].text, "This is a test.");

    // Clean up
    storage
        .delete_job_data(job_id)
        .await
        .expect("Failed to delete job data");

    println!("✅ PostgreSQL integration test passed: transcription segments batch CRUD");
}

#[tokio::test]
#[ignore] // Requires PostgreSQL running on localhost:5432
async fn test_postgres_detection_results() {
    if !is_postgres_available().await {
        eprintln!("PostgreSQL not available on 127.0.0.1:5432");
        eprintln!("Skipping test_postgres_detection_results");
        return;
    }

    let config = PostgresConfig {
        host: "localhost".to_string(),
        port: 5432,
        database: "video_audio_extracts".to_string(),
        user: "postgres".to_string(),
        password: "postgres".to_string(),
    };

    let storage = PostgresMetadataStorage::new(config)
        .await
        .expect("Failed to connect to PostgreSQL");

    storage
        .init_schema()
        .await
        .expect("Failed to initialize schema");

    let job_id = "test-detection-job";

    // Create test detections matching actual struct definition
    let detections = vec![
        DetectionResult {
            job_id: job_id.to_string(),
            frame_id: "frame_001".to_string(),
            class_id: 0,
            class_name: "person".to_string(),
            confidence: 0.92,
            bbox: (0.1, 0.2, 0.15, 0.3), // normalized coordinates
        },
        DetectionResult {
            job_id: job_id.to_string(),
            frame_id: "frame_001".to_string(),
            class_id: 2,
            class_name: "car".to_string(),
            confidence: 0.88,
            bbox: (0.4, 0.5, 0.2, 0.1),
        },
    ];

    // Store detections
    let stored_count = storage
        .store_detection_results(&detections)
        .await
        .expect("Failed to store detection results");
    assert_eq!(stored_count, 2);

    // Retrieve detections
    let retrieved = storage
        .get_detection_results(job_id)
        .await
        .expect("Failed to retrieve detection results");

    assert_eq!(retrieved.len(), 2);
    assert_eq!(retrieved[0].class_name, "person");
    assert_eq!(retrieved[1].class_name, "car");

    // Clean up
    storage
        .delete_job_data(job_id)
        .await
        .expect("Failed to delete job data");

    println!("✅ PostgreSQL integration test passed: detection results batch CRUD");
}

#[tokio::test]
#[ignore] // Requires PostgreSQL running on localhost:5432
async fn test_postgres_timeline_entries() {
    if !is_postgres_available().await {
        eprintln!("PostgreSQL not available on 127.0.0.1:5432");
        eprintln!("Skipping test_postgres_timeline_entries");
        return;
    }

    let config = PostgresConfig {
        host: "localhost".to_string(),
        port: 5432,
        database: "video_audio_extracts".to_string(),
        user: "postgres".to_string(),
        password: "postgres".to_string(),
    };

    let storage = PostgresMetadataStorage::new(config)
        .await
        .expect("Failed to connect to PostgreSQL");

    storage
        .init_schema()
        .await
        .expect("Failed to initialize schema");

    let job_id = "test-timeline-job";

    // Create test timeline entries matching actual struct definition
    let entries = vec![
        TimelineEntry {
            job_id: job_id.to_string(),
            entry_type: "scene_change".to_string(),
            start_time: 0.0,
            end_time: 0.0, // instantaneous event
            data: serde_json::json!({
                "description": "Opening scene",
                "confidence": 0.95
            }),
        },
        TimelineEntry {
            job_id: job_id.to_string(),
            entry_type: "speech".to_string(),
            start_time: 5.5,
            end_time: 10.2,
            data: serde_json::json!({
                "speaker": "Speaker 1",
                "text": "Hello, world!"
            }),
        },
    ];

    // Store timeline entries
    let stored_count = storage
        .store_timeline_entries(&entries)
        .await
        .expect("Failed to store timeline entries");
    assert_eq!(stored_count, 2);

    // Retrieve timeline entries
    let retrieved = storage
        .get_timeline_entries(job_id)
        .await
        .expect("Failed to retrieve timeline entries");

    assert_eq!(retrieved.len(), 2);
    assert_eq!(retrieved[0].entry_type, "scene_change");
    assert_eq!(retrieved[1].entry_type, "speech");

    // Clean up
    storage
        .delete_job_data(job_id)
        .await
        .expect("Failed to delete job data");

    println!("✅ PostgreSQL integration test passed: timeline entries batch CRUD");
}

// ============================================================================
// Qdrant Vector Storage Integration Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant running on localhost:6333
async fn test_qdrant_collection_init() {
    if !is_qdrant_available().await {
        eprintln!("Qdrant not available on 127.0.0.1:6333");
        eprintln!("Start with: docker-compose up -d qdrant");
        eprintln!("Skipping test_qdrant_collection_init");
        return;
    }

    let config = QdrantConfig {
        url: "http://localhost:6334".to_string(),
        api_key: None,
        collection: "test_embeddings".to_string(),
        vector_dim: 512,
        distance: vector_storage::VectorDistance::Cosine,
    };

    let storage = QdrantVectorStorage::new(config)
        .await
        .expect("Failed to create Qdrant client");

    // Initialize collection (creates if not exists)
    storage
        .init_collection()
        .await
        .expect("Failed to initialize collection");

    // Re-initialize should be idempotent (no error)
    storage
        .init_collection()
        .await
        .expect("Collection initialization should be idempotent");

    println!("✅ Qdrant integration test passed: collection initialization");
}

#[tokio::test]
#[ignore] // Requires Qdrant running on localhost:6333
async fn test_qdrant_store_and_search() {
    if !is_qdrant_available().await {
        eprintln!("Qdrant not available on 127.0.0.1:6333");
        eprintln!("Skipping test_qdrant_store_and_search");
        return;
    }

    let config = QdrantConfig {
        url: "http://localhost:6334".to_string(),
        api_key: None,
        collection: "test_embeddings_search".to_string(),
        vector_dim: 128, // Smaller dimension for testing
        distance: vector_storage::VectorDistance::Cosine,
    };

    let storage = QdrantVectorStorage::new(config)
        .await
        .expect("Failed to create Qdrant client");

    storage
        .init_collection()
        .await
        .expect("Failed to initialize collection");

    // Create test embeddings matching actual struct definition
    let mut metadata1 = HashMap::new();
    metadata1.insert("frame_id".to_string(), "frame_001".to_string());

    let embedding1 = EmbeddingVector {
        job_id: "test-job-1".to_string(),
        vector_id: "vec_1".to_string(),
        embedding_type: "vision".to_string(),
        vector: vec![1.0; 128], // All 1.0s
        metadata: metadata1,
    };

    let mut metadata2 = HashMap::new();
    metadata2.insert("frame_id".to_string(), "frame_002".to_string());

    let embedding2 = EmbeddingVector {
        job_id: "test-job-2".to_string(),
        vector_id: "vec_2".to_string(),
        embedding_type: "vision".to_string(),
        vector: vec![0.5; 128], // All 0.5s (different from embedding1)
        metadata: metadata2,
    };

    let mut metadata3 = HashMap::new();
    metadata3.insert("segment_id".to_string(), "seg_001".to_string());

    let embedding3 = EmbeddingVector {
        job_id: "test-job-3".to_string(),
        vector_id: "vec_3".to_string(),
        embedding_type: "text".to_string(),
        vector: {
            let mut v = vec![1.0; 128];
            v[0] = 1.01; // Very similar to embedding1
            v
        },
        metadata: metadata3,
    };

    // Store embeddings
    let id1 = storage
        .store_embedding(&embedding1)
        .await
        .expect("Failed to store embedding 1");
    let id2 = storage
        .store_embedding(&embedding2)
        .await
        .expect("Failed to store embedding 2");
    let id3 = storage
        .store_embedding(&embedding3)
        .await
        .expect("Failed to store embedding 3");

    assert!(!id1.is_empty());
    assert!(!id2.is_empty());
    assert!(!id3.is_empty());

    // Wait a moment for Qdrant to index
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Search for similar vectors (query with embedding1's vector)
    let query_vector = vec![1.0; 128];
    let results = storage
        .search_similar(&query_vector, 3, None)
        .await
        .expect("Failed to search similar vectors");

    assert!(!results.is_empty(), "Should find similar vectors");
    // The most similar should be embedding1 or embedding3 (very similar vectors)
    assert!(
        results[0].score > 0.9,
        "Top result should have high similarity"
    );

    // Search with filter (only "vision" type)
    let mut filter = HashMap::new();
    filter.insert("embedding_type".to_string(), "vision".to_string());

    let filtered_results = storage
        .search_similar(&query_vector, 3, Some(filter))
        .await
        .expect("Failed to search with filter");

    assert_eq!(filtered_results.len(), 2, "Should find 2 vision embeddings");

    println!("✅ Qdrant integration test passed: store embeddings and similarity search");
}

#[tokio::test]
#[ignore] // Requires Qdrant running on localhost:6333
async fn test_qdrant_batch_store() {
    if !is_qdrant_available().await {
        eprintln!("Qdrant not available on 127.0.0.1:6333");
        eprintln!("Skipping test_qdrant_batch_store");
        return;
    }

    let config = QdrantConfig {
        url: "http://localhost:6334".to_string(),
        api_key: None,
        collection: "test_embeddings_batch".to_string(),
        vector_dim: 64,
        distance: vector_storage::VectorDistance::Cosine,
    };

    let storage = QdrantVectorStorage::new(config)
        .await
        .expect("Failed to create Qdrant client");

    storage
        .init_collection()
        .await
        .expect("Failed to initialize collection");

    // Create batch of embeddings
    let embeddings: Vec<EmbeddingVector> = (0..10)
        .map(|i| EmbeddingVector {
            job_id: format!("batch-job-{i}"),
            vector_id: format!("vec_{i}"),
            embedding_type: "vision".to_string(),
            vector: vec![i as f32 / 10.0; 64],
            metadata: HashMap::new(),
        })
        .collect();

    // Store batch
    let ids = storage
        .store_embeddings(&embeddings)
        .await
        .expect("Failed to store batch embeddings");

    assert_eq!(ids.len(), 10, "Should store 10 embeddings");
    assert!(
        ids.iter().all(|id| !id.is_empty()),
        "All IDs should be non-empty"
    );

    println!("✅ Qdrant integration test passed: batch embedding storage");
}
