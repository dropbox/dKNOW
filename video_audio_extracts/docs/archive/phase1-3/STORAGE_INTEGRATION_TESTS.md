# Storage Integration Tests

Comprehensive integration tests for all three storage backends: MinIO (S3), PostgreSQL, and Qdrant.

## Overview

The storage integration tests validate end-to-end functionality of the storage layer with real backend services. All tests use Docker Compose for consistent, reproducible local environments.

**Test Coverage**:
- **MinIO (S3-compatible object storage)**: 3 tests
- **PostgreSQL (relational metadata storage)**: 5 tests
- **Qdrant (vector database)**: 3 tests
- **Total**: 11 integration tests

## Quick Start

### 1. Start Storage Services

```bash
# Start all storage backends (MinIO, PostgreSQL, Qdrant)
docker-compose up -d

# Verify services are running
docker-compose ps

# View service logs
docker-compose logs -f
```

### 2. Run Integration Tests

```bash
# Run all storage integration tests
cargo test --package video-audio-storage --test storage_integration_test -- --ignored --nocapture

# Run specific test
cargo test --package video-audio-storage --test storage_integration_test test_minio_store_and_retrieve -- --ignored --nocapture

# Run only MinIO tests
cargo test --package video-audio-storage --test storage_integration_test test_minio -- --ignored --nocapture

# Run only PostgreSQL tests
cargo test --package video-audio-storage --test storage_integration_test test_postgres -- --ignored --nocapture

# Run only Qdrant tests
cargo test --package video-audio-storage --test storage_integration_test test_qdrant -- --ignored --nocapture
```

### 3. Stop Storage Services

```bash
# Stop services (data persists in Docker volumes)
docker-compose down

# Stop and remove all data
docker-compose down -v
```

## Test Details

### MinIO Object Storage Tests

**Test 1: `test_minio_store_and_retrieve`**
- Store a file via S3 API
- Retrieve the file and verify contents
- Check file existence
- Get file size
- Delete file and verify deletion

**Test 2: `test_minio_list_files`**
- Store multiple files with prefix
- List files by prefix
- Verify all files are listed correctly
- Clean up

**Test 3: `test_minio_store_from_path`**
- Create temporary local file
- Upload from filesystem path
- Retrieve and verify contents
- Clean up

### PostgreSQL Metadata Storage Tests

**Test 1: `test_postgres_schema_init`**
- Initialize database schema (CREATE TABLES)
- Verify idempotency (re-run should not error)

**Test 2: `test_postgres_media_metadata`**
- Store media metadata (job_id, duration, resolution, etc.)
- Retrieve metadata by job_id
- Verify all fields match
- Delete job data

**Test 3: `test_postgres_transcription_segments`**
- Store batch of transcription segments (2 segments)
- Retrieve segments by job_id
- Verify text content and ordering
- Delete job data

**Test 4: `test_postgres_detection_results`**
- Store batch of object detection results (2 detections)
- Retrieve detections by job_id
- Verify class names and bounding boxes
- Delete job data

**Test 5: `test_postgres_timeline_entries`**
- Store batch of timeline entries (scene changes, speech events)
- Retrieve timeline by job_id
- Verify entry types and timestamps
- Delete job data

### Qdrant Vector Storage Tests

**Test 1: `test_qdrant_collection_init`**
- Create collection with specified vector dimensions
- Verify idempotency (re-run should not error)

**Test 2: `test_qdrant_store_and_search`**
- Store 3 embeddings (128-dimensional vectors)
- Search for similar vectors (no filter)
- Verify similarity scores are correct
- Search with filter (embedding_type="vision")
- Verify filtered results

**Test 3: `test_qdrant_batch_store`**
- Store batch of 10 embeddings
- Verify all point IDs are returned
- Verify no errors

## Service Configuration

All services use default configurations suitable for testing:

### MinIO
- **Console URL**: http://localhost:9001
- **API Endpoint**: http://localhost:9000
- **Username**: minioadmin
- **Password**: minioadmin
- **Default Bucket**: video-audio-extracts (auto-created)

### PostgreSQL
- **Host**: localhost
- **Port**: 5432
- **Database**: video_audio_extracts (auto-created)
- **Username**: postgres
- **Password**: postgres

### Qdrant
- **HTTP API**: http://localhost:6333
- **gRPC API**: http://localhost:6334
- **Web UI**: http://localhost:6333/dashboard

## Troubleshooting

### Tests are skipped

If you see messages like "MinIO not available on 127.0.0.1:9000", the services are not running:

```bash
docker-compose up -d
# Wait 5-10 seconds for services to start
cargo test --package video-audio-storage --test storage_integration_test -- --ignored --nocapture
```

### Port conflicts

If ports are already in use, modify `docker-compose.yml`:

```yaml
# Change MinIO ports
ports:
  - "9002:9000"  # API
  - "9003:9001"  # Console

# Then update test connection strings accordingly
```

### Clean slate testing

To ensure clean state between test runs:

```bash
# Remove all data and restart
docker-compose down -v
docker-compose up -d
# Wait 5-10 seconds
cargo test --package video-audio-storage --test storage_integration_test -- --ignored --nocapture
```

### View service logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f minio
docker-compose logs -f postgres
docker-compose logs -f qdrant
```

## Architecture

### Storage Layer Structure

```
crates/storage/
├── src/
│   ├── lib.rs                   # Public API and data structures
│   ├── object_storage.rs        # S3/MinIO implementation
│   ├── metadata_storage.rs      # PostgreSQL implementation
│   └── vector_storage.rs        # Qdrant implementation
└── tests/
    └── storage_integration_test.rs  # 11 integration tests
```

### Data Structures

**MediaMetadata**: Video/audio file metadata (duration, resolution, codecs)
**TranscriptionSegment**: Speech-to-text segments with timestamps
**DetectionResult**: Object/face detection bounding boxes
**TimelineEntry**: Unified timeline events (scenes, speech, detections)
**EmbeddingVector**: High-dimensional vectors for semantic search

### Test Isolation

- Each test uses unique job IDs to avoid conflicts
- Tests clean up after themselves (delete operations)
- Services are stateless between test runs (fresh containers)
- Tests marked `#[ignore]` to prevent accidental CI runs without services

## Integration with API Server

The storage backends are used by the API server for:
- **MinIO**: Raw media files, extracted audio, keyframes, thumbnails
- **PostgreSQL**: Job metadata, transcriptions, detections, timelines
- **Qdrant**: Semantic embeddings for similarity search

The API server tests (`crates/api-server/tests/integration_test.rs`) include semantic search tests that use Qdrant if available, but gracefully degrade if not.

## CI/CD Considerations

These tests are marked `#[ignore]` and require explicit opt-in:

```bash
# CI environments should run:
docker-compose up -d
sleep 10  # Wait for services to be ready
cargo test --workspace --test storage_integration_test -- --ignored
docker-compose down -v
```

**Recommendation**: Run storage integration tests in a separate CI job from unit tests for faster feedback loops.

## Performance Notes

- **Test Duration**: ~2-5 seconds per test (depends on Docker startup)
- **Total Runtime**: ~30-60 seconds for all 11 tests
- **Docker Resources**: ~500 MB memory, minimal CPU
- **Storage**: ~100 MB for Docker images and volumes

## Future Enhancements

1. **Stress Testing**: Add tests with large batches (1000+ embeddings, 10,000+ segments)
2. **Concurrency Testing**: Verify thread-safety with parallel operations
3. **Failure Recovery**: Test reconnection logic and retry mechanisms
4. **Cloud Integration**: Add tests for AWS S3, RDS, managed Qdrant
5. **Backup/Restore**: Validate data migration and disaster recovery
