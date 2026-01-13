# Storage Integration Guide

This guide explains how the storage layer is integrated with the orchestrator and how to use the complete pipeline.

## Architecture

The system now has a complete data flow:

```
Input Media → Ingestion → Processing (Audio/Video/Keyframes) → Storage → Output
```

### Task Graph Structure

The orchestrator creates a task graph with dependencies (real-time mode):

```
ingestion (root)
├── audio_extract (depends on: ingestion)
│   └── diarization (depends on: audio_extract)
├── keyframes (depends on: ingestion)
│   ├── face_detection (depends on: keyframes)
│   └── ocr (depends on: keyframes)
├── scene_detection (depends on: ingestion)
└── storage (depends on: ingestion, audio_extract, keyframes)
```

**Note**: Bulk processing mode (`execute_bulk`) executes each file's full pipeline independently for reliability and throughput (3.4 files/sec on Kinetics-600 dataset).

### Storage Backends

The storage layer uses three backends:

1. **Object Storage (S3/MinIO)**: Stores raw files (audio, keyframes, thumbnails)
2. **Vector Database (Qdrant)**: Stores embeddings for semantic search
3. **Metadata Database (PostgreSQL)**: Stores structured metadata, transcriptions, detections

## Configuration

Storage backends are configured via environment variables:

### S3/MinIO Configuration
```bash
export S3_BUCKET=video-audio-extracts
export S3_ENDPOINT=http://localhost:9000  # For MinIO
export AWS_ACCESS_KEY_ID=minioadmin
export AWS_SECRET_ACCESS_KEY=minioadmin
export AWS_REGION=us-east-1
```

### Qdrant Configuration
```bash
export QDRANT_URL=http://localhost:6334
export QDRANT_API_KEY=optional-api-key
```

### PostgreSQL Configuration
```bash
export POSTGRES_HOST=localhost
export POSTGRES_PORT=5432
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=password
export POSTGRES_DATABASE=video_audio_extracts
```

## Running the Pipeline

### Basic Usage

```bash
# Process a video file
cargo run --release --bin video-audio-orchestrator /path/to/video.mp4
```

The orchestrator will:
1. Ingest the media file and extract metadata
2. Extract audio stream to WAV format and run speaker diarization
3. Extract keyframes from video
4. Detect scene boundaries using FFmpeg scdet
5. Run face detection and OCR on keyframes
6. Generate semantic embeddings (CLIP vision, CLAP audio, Sentence-Transformers text)
7. Run fusion layer for cross-modal temporal alignment
8. Store all results in the configured backends (S3/MinIO, PostgreSQL, Qdrant)

### Example Output

```
Video Audio Extraction System v0.1.0
Created task graph with 4 tasks for file: test.mp4
Starting execution...
Task ingestion completed successfully
Task audio_extract completed successfully
Task keyframes completed successfully
Task storage completed successfully
Job job-123 completed successfully!

=== Processing Results ===
Job ID: job-123
Input: test.mp4
Total tasks: 4
Completed: 4
Failed: 0

Task: ingestion (ingestion)
  Status: Completed
  Format: mov,mp4,m4a,3gp,3g2,mj2, Duration: 10.50s, Streams: 2

Task: audio_extract (audio_extraction)
  Status: Completed
  Output: /tmp/abc123_audio.wav

Task: keyframes (keyframe_extraction)
  Status: Completed
  Keyframes extracted: 15

Task: storage (storage)
  Status: Completed
  Files stored: 16
  Metadata records: 1
  Embeddings: 0
```

## Storage Task Implementation

The storage task collects results from all previous tasks and stores them:

### Media Metadata
Stored in PostgreSQL:
- Job ID
- Input path
- Format, duration, streams
- Resolution, frame rate (if available)
- Processing timestamp

### Object Storage
Files stored in S3/MinIO under `{job_id}/` prefix:
- `{job_id}/audio.wav` - Extracted audio
- `{job_id}/keyframes/frame_0000.jpg` - Keyframe images
- `{job_id}/keyframes/frame_0001.jpg`
- ...

### Stored Data

The storage task stores:
- **PostgreSQL**: Media metadata, transcription segments (if transcription enabled), detection results (face, OCR), timeline entries from fusion layer
- **S3/MinIO**: Raw files (extracted audio WAV, keyframe images, thumbnails) under `{job_id}/` prefix
- **Qdrant**: Semantic embeddings (CLIP vision embeddings from keyframes, CLAP audio embeddings, Sentence-Transformers text embeddings from transcriptions)

## Error Handling

The storage task is resilient to backend failures:
- If PostgreSQL is unavailable, metadata storage fails gracefully (warning logged)
- If S3/MinIO is unavailable, file storage fails gracefully (warning logged)
- If Qdrant is unavailable, embedding storage fails gracefully (warning logged)

The task still completes successfully if at least some data was stored.

## Development Setup

### Local Development with MinIO + Qdrant + PostgreSQL

Use Docker Compose to run all backends locally:

```yaml
version: '3.8'
services:
  minio:
    image: minio/minio
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    volumes:
      - minio-data:/data

  qdrant:
    image: qdrant/qdrant
    ports:
      - "6333:6333"
      - "6334:6334"
    volumes:
      - qdrant-data:/qdrant/storage

  postgres:
    image: postgres:15
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
      POSTGRES_DB: video_audio_extracts
    volumes:
      - postgres-data:/var/lib/postgresql/data

volumes:
  minio-data:
  qdrant-data:
  postgres-data:
```

Save as `docker-compose.yml` and run:

```bash
docker-compose up -d
```

### Database Schema Setup

The PostgreSQL schema is automatically created on first use by the metadata storage module. Tables created:

- `media_metadata` - Media file metadata
- `transcription_segments` - Transcription results
- `detection_results` - Object detection results
- `timeline_entries` - Timeline of events

## Testing

Run the full test suite:

```bash
# Unit tests
cargo test --release

# Integration test (requires Docker backends running)
export INTEGRATION_TEST=1
cargo test --release -- --ignored
```

## Performance Considerations

### Object Storage
- Files are uploaded sequentially
- Future: Implement parallel uploads with semaphore
- Future: Implement chunked uploads for large files

### Metadata Storage
- Single connection per task
- Future: Connection pooling
- Future: Batch inserts for better performance

### Vector Storage
- Qdrant collection is created if it doesn't exist
- Vector dimension: 512 (configurable)
- Distance metric: Cosine similarity

## Troubleshooting

### Storage task fails with connection errors
- Check that Docker services are running: `docker-compose ps`
- Verify environment variables are set correctly
- Check network connectivity to backend services

### Files not appearing in MinIO
- Access MinIO console at http://localhost:9001
- Login with minioadmin/minioadmin
- Check the bucket exists and files are present

### PostgreSQL connection refused
- Check PostgreSQL is running: `docker-compose ps postgres`
- Verify credentials in environment variables
- Check PostgreSQL logs: `docker-compose logs postgres`

### Qdrant connection errors
- Check Qdrant is running: `docker-compose ps qdrant`
- Verify URL is correct: http://localhost:6334
- Check Qdrant logs: `docker-compose logs qdrant`

## Semantic Search

The system includes a complete semantic search API that enables multi-modal queries:

```bash
# Text query example
curl -X POST http://localhost:3000/api/v1/search/similar \
  -H "Content-Type: application/json" \
  -d '{
    "query_type": "text",
    "query_data": "people walking on beach",
    "limit": 10
  }'

# Image query example (base64-encoded image)
curl -X POST http://localhost:3000/api/v1/search/similar \
  -H "Content-Type: application/json" \
  -d '{
    "query_type": "image",
    "query_data": "<base64-encoded-image>",
    "limit": 10
  }'
```

See API_USAGE.md for complete API documentation including bulk processing, job status, and result retrieval.

## Future Enhancements

1. Streaming results support (Server-Sent Events / WebSocket) - 2-3 AI commits
2. Storage quota management and data retention policies
3. Extended benchmarking on full Kinetics-600 dataset (18,288 files)
4. Performance profiling and optimization
