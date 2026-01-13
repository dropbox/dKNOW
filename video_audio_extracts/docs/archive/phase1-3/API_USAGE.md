# REST API Usage Guide

The Video & Audio Processing API provides two processing modes optimized for different use cases.

## Starting the Server

```bash
# Default: Listen on 0.0.0.0:8080
cargo run --release --bin video-audio-api-server

# Custom address
API_SERVER_ADDR=127.0.0.1:3000 cargo run --release --bin video-audio-api-server

# With logging
RUST_LOG=info cargo run --release --bin video-audio-api-server
```

## API Endpoints

### Health Check

```bash
curl http://localhost:8080/health
```

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

---

## Real-Time Processing API

Optimizes for **minimum latency** with parallel CPU+GPU execution.

**Endpoint:** `POST /api/v1/process/realtime`

### Example: Process Local File

```bash
curl -X POST http://localhost:8080/api/v1/process/realtime \
  -H "Content-Type: application/json" \
  -d '{
    "source": {
      "type": "upload",
      "location": "/path/to/video.mp4"
    },
    "processing": {
      "priority": "realtime",
      "required_features": ["keyframes", "audio"],
      "optional_features": ["transcription", "objects"],
      "quality_mode": "balanced"
    }
  }'
```

### Example: Process from URL

```bash
curl -X POST http://localhost:8080/api/v1/process/realtime \
  -H "Content-Type: application/json" \
  -d '{
    "source": {
      "type": "url",
      "location": "https://example.com/sample-video.mp4"
    },
    "processing": {
      "priority": "realtime",
      "required_features": ["keyframes", "audio"],
      "optional_features": ["transcription"]
    }
  }'
```

### Example: Process from S3

```bash
curl -X POST http://localhost:8080/api/v1/process/realtime \
  -H "Content-Type: application/json" \
  -d '{
    "source": {
      "type": "s3",
      "location": "s3://my-videos/recordings/meeting-2024-01-15.mp4"
    },
    "processing": {
      "priority": "realtime",
      "required_features": ["transcription", "audio"]
    }
  }'
```

**Response:**
```json
{
  "job_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "status": "running",
  "message": "Job started successfully"
}
```

### Processing Modes

- **priority**: `"realtime"` (parallel processing) or `"bulk"` (independent parallel execution)
- **quality_mode**:
  - `"fast"` - Quick processing, lower accuracy
  - `"balanced"` - Default, good speed/accuracy tradeoff
  - `"accurate"` - Highest accuracy, slower

### Feature Configuration

**Required Features** (job fails if these fail):
- `"keyframes"` - Extract keyframes from video
- `"audio"` - Extract audio track
- `"transcription"` - Transcribe speech to text
- `"objects"` - Detect objects in video frames

**Optional Features** (job continues if these fail):
- Same options as required features

---

## Bulk Processing API

Optimizes for **maximum throughput** with independent parallel execution of multiple files.

**Endpoint:** `POST /api/v1/process/bulk`

### Example: Process Multiple Files (Mixed Sources)

```bash
curl -X POST http://localhost:8080/api/v1/process/bulk \
  -H "Content-Type: application/json" \
  -d '{
    "batch_id": "batch_2024_01",
    "files": [
      {
        "id": "file_1",
        "source": {
          "type": "upload",
          "location": "/path/to/local/video1.mp4"
        },
        "processing": {
          "priority": "bulk",
          "required_features": ["keyframes", "audio"]
        }
      },
      {
        "id": "file_2",
        "source": {
          "type": "url",
          "location": "https://example.com/recordings/video2.mp4"
        },
        "processing": {
          "priority": "bulk",
          "required_features": ["keyframes", "audio", "transcription"]
        }
      },
      {
        "id": "file_3",
        "source": {
          "type": "s3",
          "location": "s3://my-bucket/videos/video3.mp4"
        },
        "processing": {
          "priority": "bulk",
          "required_features": ["transcription"]
        }
      }
    ],
    "batch_config": {
      "priority": "bulk",
      "optimize_for": "throughput"
    }
  }'
```

**Response:**
```json
{
  "batch_id": "batch_2024_01",
  "job_ids": [
    "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "b2c3d4e5-f6a7-8901-bcde-f12345678901",
    "c3d4e5f6-a7b8-9012-cdef-123456789012"
  ],
  "message": "Batch processing started successfully"
}
```

---

## Job Status and Results

### Get Job Status

```bash
curl http://localhost:8080/api/v1/jobs/{job_id}/status
```

**Response:**
```json
{
  "job_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "status": "running",
  "total_tasks": 4,
  "completed_tasks": 2,
  "failed_tasks": 0,
  "error": null
}
```

**Status Values:**
- `"queued"` - Job is waiting to start
- `"running"` - Job is currently processing
- `"completed"` - Job finished successfully
- `"failed"` - Job encountered an error

### Get Job Results

```bash
curl http://localhost:8080/api/v1/jobs/{job_id}/result
```

**Response:**
```json
{
  "job_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "status": "completed",
  "results": {
    "ingestion": {
      "type": "ingestion",
      "format": "mp4",
      "duration": 120.5,
      "num_streams": 2
    },
    "audio_extract": {
      "type": "audio_extraction",
      "path": "/tmp/abc123_audio.wav"
    },
    "keyframes": {
      "type": "keyframe_extraction",
      "num_keyframes": 15,
      "paths": ["/tmp/abc123_keyframes/frame_0000.jpg", "..."]
    },
    "storage": {
      "type": "storage",
      "files_stored": 16,
      "metadata_records": 1,
      "embeddings_stored": 0
    }
  },
  "error": null
}
```

---

## Media Source Types

### Uploaded File (Local Path)

```json
{
  "source": {
    "type": "upload",
    "location": "/path/to/local/file.mp4"
  }
}
```

### HTTP/HTTPS URL

Download media files directly from HTTP/HTTPS URLs. Files are automatically downloaded to temporary storage and cleaned up after processing.

```json
{
  "source": {
    "type": "url",
    "location": "https://example.com/video.mp4"
  }
}
```

**Features:**
- 5-minute download timeout
- Automatic file extension detection from URL path or Content-Type header
- Support for all standard video, audio, and image formats
- Query parameters are handled correctly (e.g., `?token=xyz`)

### S3 Bucket

Download media files from AWS S3 or MinIO-compatible object storage.

```json
{
  "source": {
    "type": "s3",
    "location": "s3://my-bucket/path/to/video.mp4"
  }
}
```

**Features:**
- Compatible with AWS S3 and MinIO
- Uses AWS SDK for Rust with standard credential chain (environment variables, IAM roles, etc.)
- Automatic file extension detection from object key or Content-Type metadata
- Supports nested paths in bucket

**AWS Configuration:**
Configure AWS credentials via environment variables:
```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_REGION=us-east-1
# For MinIO, also set:
export AWS_ENDPOINT_URL=http://localhost:9000
```

---

## Error Handling

### Input File Not Found

```json
{
  "error": "Input file does not exist: /path/to/missing.mp4"
}
```
**Status Code:** `400 Bad Request`

### Job Not Found

```json
{
  "error": "Job not found: f47ac10b-58cc-4372-a567-0e02b2c3d479"
}
```
**Status Code:** `404 Not Found`

### Job Failed

When querying a failed job's result:
```json
{
  "job_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "status": "failed",
  "results": {},
  "error": "Job execution failed"
}
```

---

## Storage Configuration

The API server stores results in configured storage backends. Configure via environment variables:

```bash
# S3/MinIO Configuration
export S3_BUCKET=my-bucket
export S3_REGION=us-east-1
export S3_ENDPOINT=http://localhost:9000  # Optional, for MinIO
export AWS_ACCESS_KEY_ID=minioadmin
export AWS_SECRET_ACCESS_KEY=minioadmin

# PostgreSQL Configuration
export POSTGRES_HOST=localhost
export POSTGRES_PORT=5432
export POSTGRES_DATABASE=media_metadata
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=postgres

# Qdrant Configuration
export QDRANT_URL=http://localhost:6333
export QDRANT_API_KEY=  # Optional
export QDRANT_COLLECTION=media_embeddings
```

If storage backends are unavailable, the API server will log warnings but continue processing. Results will still be available via the job result endpoint.

---

## Architecture Notes

### Real-Time Mode
- **Optimization:** Minimum latency
- **Execution:** Parallel CPU + GPU tasks
- **Use Case:** Interactive applications, single-file uploads

### Bulk Mode
- **Optimization:** Maximum throughput
- **Execution:** Independent parallel processing (each file progresses through full pipeline)
- **Performance:** ~3.4 files/sec on Kinetics-600 dataset
- **Use Case:** Batch processing, overnight jobs, data pipelines

Both modes use the same underlying orchestrator with different task graph configurations.

---

## Next Steps

See `INTEGRATION_GUIDE.md` for storage backend setup and `AI_TECHNICAL_SPEC.md` for detailed API specifications.
