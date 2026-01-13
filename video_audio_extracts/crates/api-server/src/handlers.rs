//! HTTP request handlers for API endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::path::PathBuf;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    download::{download_from_s3, download_from_url},
    types::{JobStatus, JobStatusResponse, MediaSource},
    ApiState, BulkRequest, BulkResponse, JobResult, RealtimeRequest, RealtimeResponse,
};
use video_audio_orchestrator::TaskResult;
use video_audio_storage::VectorStorage;

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(crate::types::HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Process media file in real-time mode
///
/// Real-time mode optimizes for minimum latency:
/// - Parallel CPU + GPU execution
/// - No queuing
/// - Streaming results (if enabled)
pub async fn process_realtime(
    State(state): State<ApiState>,
    Json(request): Json<RealtimeRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Generate job ID
    let job_id = Uuid::new_v4().to_string();
    info!(
        "Real-time processing request: job_id={}, priority={:?}",
        job_id, request.processing.priority
    );

    // Extract media file path from source
    // For remote sources (URL, S3), download to temporary file
    // We need to keep the downloaded file alive until processing completes,
    // so we'll move it into the async task
    let (input_path, _downloaded_file) = match &request.source {
        MediaSource::Upload { location } => (PathBuf::from(location), None),
        MediaSource::Url { location } => {
            info!("Downloading file from URL: {}", location);
            let downloaded = download_from_url(location).await.map_err(|e| {
                error!("Failed to download from URL {}: {}", location, e);
                (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to download from URL: {e}"),
                )
            })?;
            let path = downloaded.path().to_path_buf();
            (path, Some(downloaded))
        }
        MediaSource::S3 { location } => {
            info!("Downloading file from S3: {}", location);
            let downloaded = download_from_s3(location).await.map_err(|e| {
                error!("Failed to download from S3 {}: {}", location, e);
                (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to download from S3: {e}"),
                )
            })?;
            let path = downloaded.path().to_path_buf();
            (path, Some(downloaded))
        }
    };

    // Validate input path exists
    if !input_path.exists() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Input file does not exist: {}", input_path.display()),
        ));
    }

    // Build real-time task graph
    let graph = state
        .orchestrator
        .build_realtime_graph(job_id.clone(), input_path);

    // Spawn task to execute graph asynchronously
    let orchestrator = state.orchestrator.clone();
    let results_cache = state.results.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        // Keep downloaded file alive for the duration of processing
        let _downloaded_file_guard = _downloaded_file;
        info!("Starting real-time processing for job {}", job_id_clone);

        match orchestrator.execute(graph).await {
            Ok(completed_graph) => {
                info!("Job {} completed successfully", job_id_clone);

                // Build results map
                let tasks = completed_graph.tasks();
                let mut results = std::collections::HashMap::with_capacity(tasks.len());
                for (task_id, task) in tasks {
                    if let Some(result) = &task.result {
                        results.insert(task_id.clone(), task_result_to_json(result));
                    }
                }

                // Store result
                let job_result = JobResult {
                    job_id: job_id_clone.clone(),
                    status: JobStatus::Completed,
                    results,
                    error: None,
                };
                results_cache.write().await.insert(job_id_clone, job_result);
            }
            Err(e) => {
                error!("Job {} failed: {}", job_id_clone, e);

                // Store error result
                let job_result = JobResult {
                    job_id: job_id_clone.clone(),
                    status: JobStatus::Failed,
                    results: std::collections::HashMap::new(),
                    error: Some(e.to_string()),
                };
                results_cache.write().await.insert(job_id_clone, job_result);
            }
        }
    });

    // Return immediate response
    Ok((
        StatusCode::ACCEPTED,
        Json(RealtimeResponse {
            job_id,
            status: JobStatus::Running,
            message: Some("Job started successfully".to_string()),
        }),
    ))
}

/// Process multiple media files in bulk mode
///
/// Bulk mode optimizes for maximum throughput:
/// - Staged processing (all ingestion → all CPU → all GPU → all storage)
/// - Model reuse across files (reduces load/unload overhead)
/// - Better resource utilization (90%+ CPU, 85%+ GPU efficiency target)
pub async fn process_bulk(
    State(state): State<ApiState>,
    Json(request): Json<BulkRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let batch_id = request.batch_id.clone();
    info!(
        "Bulk processing request: batch_id={}, files={}",
        batch_id,
        request.files.len()
    );

    // Validate files
    if request.files.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No files to process".to_string()));
    }

    let orchestrator = state.orchestrator.clone();
    let results_cache = state.results.clone();

    // Download all files and create task graphs
    let mut graphs = Vec::with_capacity(request.files.len());
    let mut job_ids = Vec::with_capacity(request.files.len());
    let mut downloaded_files = Vec::with_capacity(request.files.len()); // Keep files alive during processing

    for file in request.files {
        let job_id = Uuid::new_v4().to_string();

        // Extract media file path
        // For remote sources, download to temporary file
        let (input_path, downloaded_file) = match &file.source {
            MediaSource::Upload { location } => (PathBuf::from(location), None),
            MediaSource::Url { location } => {
                info!("Downloading file from URL: {}", location);
                match download_from_url(location).await {
                    Ok(downloaded) => {
                        let path = downloaded.path().to_path_buf();
                        (path, Some(downloaded))
                    }
                    Err(e) => {
                        error!("Failed to download from URL {}: {}", location, e);
                        continue;
                    }
                }
            }
            MediaSource::S3 { location } => {
                info!("Downloading file from S3: {}", location);
                match download_from_s3(location).await {
                    Ok(downloaded) => {
                        let path = downloaded.path().to_path_buf();
                        (path, Some(downloaded))
                    }
                    Err(e) => {
                        error!("Failed to download from S3 {}: {}", location, e);
                        continue;
                    }
                }
            }
        };

        // Validate input path exists
        if !input_path.exists() {
            warn!("Input file does not exist: {}", input_path.display());
            continue;
        }

        // Build task graph for this file
        let graph = orchestrator.build_realtime_graph(job_id.clone(), input_path);
        graphs.push(graph);
        job_ids.push(job_id);
        if let Some(df) = downloaded_file {
            downloaded_files.push(df);
        }
    }

    info!(
        "Created {} task graphs for bulk processing (batch_id={})",
        graphs.len(),
        batch_id
    );

    // Spawn background task for staged execution
    let batch_id_clone = batch_id.clone();
    let job_ids_clone = job_ids.clone();
    tokio::spawn(async move {
        // Keep downloaded files alive for the duration of processing
        let _downloaded_files_guard = downloaded_files;

        info!(
            "Starting bulk execution for batch {} ({} jobs)",
            batch_id_clone,
            job_ids_clone.len()
        );

        match orchestrator.execute_bulk(graphs).await {
            Ok(completed_graphs) => {
                info!(
                    "Batch {} completed: {}/{} jobs successful",
                    batch_id_clone,
                    completed_graphs.iter().filter(|g| !g.has_failed()).count(),
                    completed_graphs.len()
                );

                // Store results for each job
                for (idx, completed_graph) in completed_graphs.iter().enumerate() {
                    let job_id = &job_ids_clone[idx];

                    // Build results map
                    let tasks = completed_graph.tasks();
                    let mut results = std::collections::HashMap::with_capacity(tasks.len());
                    for (task_id, task) in tasks {
                        if let Some(result) = &task.result {
                            results.insert(task_id.clone(), task_result_to_json(result));
                        }
                    }

                    // Determine job status
                    let status = if completed_graph.has_failed() {
                        JobStatus::Failed
                    } else {
                        JobStatus::Completed
                    };

                    let error = if completed_graph.has_failed() {
                        Some(format!(
                            "{} tasks failed",
                            completed_graph.failed_tasks().len()
                        ))
                    } else {
                        None
                    };

                    // Store result
                    let job_result = JobResult {
                        job_id: job_id.clone(),
                        status,
                        results,
                        error,
                    };

                    results_cache
                        .write()
                        .await
                        .insert(job_id.clone(), job_result);
                }
            }
            Err(e) => {
                error!("Batch {} failed: {}", batch_id_clone, e);

                // Mark all jobs as failed
                for job_id in &job_ids_clone {
                    let job_result = JobResult {
                        job_id: job_id.clone(),
                        status: JobStatus::Failed,
                        results: std::collections::HashMap::new(),
                        error: Some(format!("Batch execution failed: {e}")),
                    };
                    results_cache
                        .write()
                        .await
                        .insert(job_id.clone(), job_result);
                }
            }
        }
    });

    // Return immediate response
    Ok((
        StatusCode::ACCEPTED,
        Json(BulkResponse {
            batch_id,
            job_ids,
            message: "Batch processing started successfully (staged execution mode)".to_string(),
        }),
    ))
}

/// Get job status
pub async fn get_job_status(
    State(state): State<ApiState>,
    Path(job_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check if job exists in orchestrator
    if let Some(status) = state.orchestrator.get_job_status(&job_id).await {
        // A job is considered complete if all tasks are settled (completed or failed)
        // Optional tasks (like face detection) may fail without failing the overall job
        let job_status = if status.is_complete {
            // Job is complete once all tasks are settled (completed or failed)
            // Even if some optional tasks failed, the job succeeded overall
            JobStatus::Completed
        } else {
            // Job is still running - some tasks may have failed but others are pending/running
            JobStatus::Running
        };

        Ok(Json(JobStatusResponse {
            job_id: job_id.clone(),
            status: job_status,
            total_tasks: status.total_tasks,
            completed_tasks: status.completed_tasks,
            failed_tasks: status.failed_tasks,
            error: None,
        }))
    } else {
        // Check if job is in results cache
        let results = state.results.read().await;
        if let Some(result) = results.get(&job_id) {
            Ok(Json(JobStatusResponse {
                job_id: job_id.clone(),
                status: result.status.clone(),
                total_tasks: result.results.len(),
                completed_tasks: result.results.len(),
                failed_tasks: usize::from(result.status == JobStatus::Failed),
                error: result.error.clone(),
            }))
        } else {
            Err((StatusCode::NOT_FOUND, format!("Job not found: {job_id}")))
        }
    }
}

/// Get job result
pub async fn get_job_result(
    State(state): State<ApiState>,
    Path(job_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let results = state.results.read().await;
    if let Some(result) = results.get(&job_id) {
        Ok(Json(result.clone()))
    } else {
        Err((StatusCode::NOT_FOUND, format!("Job not found: {job_id}")))
    }
}

/// Convert `TaskResult` to JSON value
fn task_result_to_json(result: &TaskResult) -> serde_json::Value {
    match result {
        TaskResult::Ingestion(info) => serde_json::json!({
            "type": "ingestion",
            "format": info.format,
            "duration": info.duration,
            "num_streams": info.streams.len(),
        }),
        TaskResult::AudioExtraction(path) => serde_json::json!({
            "type": "audio_extraction",
            "path": path.display().to_string(),
        }),
        TaskResult::KeyframeExtraction(paths) => {
            // Pre-allocate paths Vec with exact size
            let mut path_strings = Vec::with_capacity(paths.len());
            path_strings.extend(paths.iter().map(|p| p.display().to_string()));
            serde_json::json!({
                "type": "keyframe_extraction",
                "num_keyframes": paths.len(),
                "paths": path_strings,
            })
        }
        TaskResult::Transcription(text) => serde_json::json!({
            "type": "transcription",
            "text": text,
        }),
        TaskResult::Diarization(diarization) => {
            // Pre-allocate speakers Vec with exact size
            let mut speakers_json = Vec::with_capacity(diarization.speakers.len());
            speakers_json.extend(diarization.speakers.iter().map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "total_speaking_time": s.total_speaking_time,
                })
            }));
            // Pre-allocate timeline Vec with exact size
            let mut timeline_json = Vec::with_capacity(diarization.timeline.len());
            timeline_json.extend(diarization.timeline.iter().map(|seg| {
                serde_json::json!({
                    "start": seg.start,
                    "end": seg.end,
                    "speaker": seg.speaker,
                    "confidence": seg.confidence,
                })
            }));
            serde_json::json!({
                "type": "diarization",
                "num_speakers": diarization.speakers.len(),
                "num_segments": diarization.timeline.len(),
                "speakers": speakers_json,
                "timeline": timeline_json,
            })
        }
        TaskResult::ObjectDetection(count) => serde_json::json!({
            "type": "object_detection",
            "num_detections": count,
        }),
        TaskResult::FaceDetection(faces_per_keyframe) => {
            let total_faces: usize = faces_per_keyframe.iter().map(std::vec::Vec::len).sum();
            // Pre-allocate outer Vec with exact size (keyframes)
            let mut faces_json = Vec::with_capacity(faces_per_keyframe.len());
            faces_json.extend(faces_per_keyframe.iter().map(|faces| {
                // Pre-allocate inner Vec with exact size (faces per keyframe)
                let mut keyframe_faces = Vec::with_capacity(faces.len());
                keyframe_faces.extend(faces.iter().map(|face| {
                    serde_json::json!({
                        "confidence": face.confidence,
                        "bbox": {
                            "x1": face.bbox.x1,
                            "y1": face.bbox.y1,
                            "x2": face.bbox.x2,
                            "y2": face.bbox.y2,
                        },
                        "landmarks": face.landmarks.as_ref().map(|l| serde_json::json!({
                            "left_eye": [l.left_eye.0, l.left_eye.1],
                            "right_eye": [l.right_eye.0, l.right_eye.1],
                            "nose": [l.nose.0, l.nose.1],
                            "left_mouth": [l.left_mouth.0, l.left_mouth.1],
                            "right_mouth": [l.right_mouth.0, l.right_mouth.1],
                        })),
                    })
                }));
                keyframe_faces
            }));
            serde_json::json!({
                "type": "face_detection",
                "total_faces": total_faces,
                "num_keyframes": faces_per_keyframe.len(),
                "faces_per_keyframe": faces_json,
            })
        }
        TaskResult::OCR(text_regions_per_keyframe) => {
            let total_text_regions: usize = text_regions_per_keyframe
                .iter()
                .map(std::vec::Vec::len)
                .sum();
            // Pre-allocate outer Vec with exact size (keyframes)
            let mut regions_json = Vec::with_capacity(text_regions_per_keyframe.len());
            regions_json.extend(text_regions_per_keyframe.iter().map(|regions| {
                // Pre-allocate inner Vec with exact size (regions per keyframe)
                let mut keyframe_regions = Vec::with_capacity(regions.len());
                keyframe_regions.extend(regions.iter().map(|region| {
                    serde_json::json!({
                        "text": region.text,
                        "confidence": region.confidence,
                        "bbox": {
                            "top_left": [region.bbox.top_left.0, region.bbox.top_left.1],
                            "top_right": [region.bbox.top_right.0, region.bbox.top_right.1],
                            "bottom_right": [region.bbox.bottom_right.0, region.bbox.bottom_right.1],
                            "bottom_left": [region.bbox.bottom_left.0, region.bbox.bottom_left.1],
                        },
                        "direction": match region.direction {
                            video_audio_ocr::TextDirection::Horizontal => "horizontal",
                            video_audio_ocr::TextDirection::Vertical => "vertical",
                            video_audio_ocr::TextDirection::Rotated(angle) => {
                                return serde_json::json!({
                                    "text": region.text,
                                    "confidence": region.confidence,
                                    "bbox": {
                                        "top_left": [region.bbox.top_left.0, region.bbox.top_left.1],
                                        "top_right": [region.bbox.top_right.0, region.bbox.top_right.1],
                                        "bottom_right": [region.bbox.bottom_right.0, region.bbox.bottom_right.1],
                                        "bottom_left": [region.bbox.bottom_left.0, region.bbox.bottom_left.1],
                                    },
                                    "direction": {
                                        "type": "rotated",
                                        "angle": angle,
                                    }
                                });
                            }
                        }
                    })
                }));
                keyframe_regions
            }));
            serde_json::json!({
                "type": "ocr",
                "total_text_regions": total_text_regions,
                "num_keyframes": text_regions_per_keyframe.len(),
                "text_regions_per_keyframe": regions_json,
            })
        }
        TaskResult::SceneDetection(scene_result) => {
            // Pre-allocate boundaries Vec with exact size
            let mut boundaries_json = Vec::with_capacity(scene_result.boundaries.len());
            boundaries_json.extend(scene_result.boundaries.iter().map(|b| {
                serde_json::json!({
                    "timestamp": b.timestamp,
                    "score": b.score,
                })
            }));
            serde_json::json!({
                "type": "scene_detection",
                "num_scenes": scene_result.num_scenes,
                "num_boundaries": scene_result.boundaries.len(),
                "threshold": scene_result.config.threshold,
                "boundaries": boundaries_json,
            })
        }
        TaskResult::VisionEmbeddings(embeddings) => serde_json::json!({
            "type": "vision_embeddings",
            "num_embeddings": embeddings.len(),
            "embedding_dim": embeddings.first().map_or(0, std::vec::Vec::len),
        }),
        TaskResult::TextEmbeddings(embeddings) => serde_json::json!({
            "type": "text_embeddings",
            "num_embeddings": embeddings.len(),
            "embedding_dim": embeddings.first().map_or(0, std::vec::Vec::len),
        }),
        TaskResult::AudioEmbeddings(embeddings) => serde_json::json!({
            "type": "audio_embeddings",
            "num_embeddings": embeddings.len(),
            "embedding_dim": embeddings.first().map_or(0, std::vec::Vec::len),
        }),
        TaskResult::Fusion(timeline) => {
            let events: Vec<_> = timeline
                .events
                .iter()
                .map(|event| {
                    serde_json::json!({
                        "id": event.id,
                        "type": event.event_type,
                        "start_time": event.start_time,
                        "end_time": event.end_time,
                        "confidence": event.confidence,
                        "data": event.data,
                    })
                })
                .collect();

            let entities: Vec<_> = timeline
                .entities
                .iter()
                .map(|entity| {
                    serde_json::json!({
                        "id": entity.id,
                        "type": entity.entity_type,
                        "first_seen": entity.first_seen,
                        "last_seen": entity.last_seen,
                        "confidence": entity.confidence,
                        "attributes": entity.attributes,
                    })
                })
                .collect();

            let relationships: Vec<_> = timeline
                .relationships
                .iter()
                .map(|rel| {
                    serde_json::json!({
                        "from_event": rel.from_event,
                        "to_event": rel.to_event,
                        "type": rel.relationship_type,
                        "confidence": rel.confidence,
                    })
                })
                .collect();

            serde_json::json!({
                "type": "fusion",
                "duration": timeline.duration,
                "num_events": timeline.events.len(),
                "num_entities": timeline.entities.len(),
                "num_relationships": timeline.relationships.len(),
                "quality_scores": {
                    "overall": timeline.quality_scores.overall,
                    "temporal_alignment": timeline.quality_scores.temporal_alignment,
                    "cross_modal_consistency": timeline.quality_scores.cross_modal_consistency,
                    "events_fused": timeline.quality_scores.events_fused,
                    "relationships_found": timeline.quality_scores.relationships_found,
                },
                "events": events,
                "entities": entities,
                "relationships": relationships,
            })
        }
        TaskResult::Storage(stats) => serde_json::json!({
            "type": "storage",
            "files_stored": stats.files_stored,
            "metadata_records": stats.metadata_records,
            "embeddings_stored": stats.embeddings_stored,
        }),
    }
}

/// Load audio file at 48kHz mono for CLAP embeddings
fn load_audio_for_embeddings(input_path: &std::path::Path) -> Result<Vec<f32>, String> {
    use video_audio_extractor::{extract_audio, AudioConfig, AudioFormat};

    // Create temp directory for intermediate WAV file
    let temp_dir = std::env::temp_dir();
    let temp_wav = temp_dir.join(format!(
        "clap_audio_{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_millis()
    ));

    // Extract audio to 48kHz mono PCM WAV (CLAP requirement)
    let config = AudioConfig {
        sample_rate: 48000,
        channels: 1,
        format: AudioFormat::PCM,
        normalize: false,
    };

    let wav_path = extract_audio(input_path, &temp_wav, &config)
        .map_err(|e| format!("Failed to extract audio: {e}"))?;

    // Read WAV samples using hound
    let mut reader =
        hound::WavReader::open(&wav_path).map_err(|e| format!("Failed to open WAV file: {e}"))?;

    let spec = reader.spec();

    // Verify format
    if spec.sample_rate != 48000 {
        return Err(format!(
            "Expected 48kHz sample rate, got {}Hz",
            spec.sample_rate
        ));
    }

    // Read samples and convert to f32
    let samples: Result<Vec<f32>, String> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            let max_val = (1 << (bits - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| {
                    s.map(|sample| sample as f32 / max_val)
                        .map_err(|e| format!("Failed to read sample: {e}"))
                })
                .collect()
        }
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.map_err(|e| format!("Failed to read sample: {e}")))
            .collect(),
    };

    // Clean up temp file
    let _ = std::fs::remove_file(&wav_path);

    samples
}

/// Semantic search endpoint
///
/// Searches for similar content using multi-modal embeddings:
/// - Text queries: "people walking on beach"
/// - Image queries: upload or URL
/// - Audio queries: upload or URL
pub async fn semantic_search(
    State(_state): State<ApiState>,
    Json(request): Json<crate::types::SearchRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Semantic search request: limit={}", request.limit);

    // Load storage backends (graceful degradation if not available)
    let storage_config = video_audio_storage::StorageConfig::default();
    let vector_storage =
        match video_audio_storage::QdrantVectorStorage::new(storage_config.qdrant).await {
            Ok(vs) => vs,
            Err(e) => {
                error!("Failed to connect to Qdrant: {}", e);
                return Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    format!("Vector database unavailable: {e}"),
                ));
            }
        };

    // Generate query embedding based on modality
    let (query_vector, query_type) = match &request.query {
        crate::types::QueryModality::Text { query } => {
            info!("Processing text query: \"{}\"", query);

            // Load text embeddings model
            let text_config = video_audio_embeddings::TextEmbeddingConfig::default();
            let mut text_embeddings = match video_audio_embeddings::TextEmbeddings::new(text_config)
            {
                Ok(te) => te,
                Err(e) => {
                    error!("Failed to load text embeddings model: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to load text embeddings model: {e}"),
                    ));
                }
            };

            // Extract embedding
            let embeddings = match text_embeddings.extract_embeddings(std::slice::from_ref(query)) {
                Ok(embs) => embs,
                Err(e) => {
                    error!("Failed to extract text embedding: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to extract text embedding: {e}"),
                    ));
                }
            };

            let embedding = if let Some(emb) = embeddings.into_iter().next() {
                emb
            } else {
                error!("Text embedding model returned empty results");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Text embedding model returned empty results".to_string(),
                ));
            };

            (embedding, "text".to_string())
        }
        crate::types::QueryModality::Image { location } => {
            info!("Processing image query");

            // Extract image path from source
            let (image_path, _downloaded_file) = match location {
                MediaSource::Upload { location } => (PathBuf::from(location), None),
                MediaSource::Url { location } => {
                    info!("Downloading image from URL: {}", location);
                    let downloaded = download_from_url(location).await.map_err(|e| {
                        error!("Failed to download from URL {}: {}", location, e);
                        (
                            StatusCode::BAD_REQUEST,
                            format!("Failed to download from URL: {e}"),
                        )
                    })?;
                    let path = downloaded.path().to_path_buf();
                    (path, Some(downloaded))
                }
                MediaSource::S3 { location } => {
                    info!("Downloading image from S3: {}", location);
                    let downloaded = download_from_s3(location).await.map_err(|e| {
                        error!("Failed to download from S3 {}: {}", location, e);
                        (
                            StatusCode::BAD_REQUEST,
                            format!("Failed to download from S3: {e}"),
                        )
                    })?;
                    let path = downloaded.path().to_path_buf();
                    (path, Some(downloaded))
                }
            };

            // Validate input path exists
            if !image_path.exists() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Input file does not exist: {}", image_path.display()),
                ));
            }

            // Load image
            let image = match image::open(&image_path) {
                Ok(img) => img,
                Err(e) => {
                    error!("Failed to load image: {}", e);
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("Failed to load image: {e}"),
                    ));
                }
            };

            // Load vision embeddings model
            let vision_config = video_audio_embeddings::VisionEmbeddingConfig::default();
            let mut vision_embeddings =
                match video_audio_embeddings::VisionEmbeddings::new(vision_config) {
                    Ok(ve) => ve,
                    Err(e) => {
                        error!("Failed to load vision embeddings model: {}", e);
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to load vision embeddings model: {e}"),
                        ));
                    }
                };

            // Extract embedding
            let embeddings = match vision_embeddings.extract_embeddings(&[image]) {
                Ok(embs) => embs,
                Err(e) => {
                    error!("Failed to extract vision embedding: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to extract vision embedding: {e}"),
                    ));
                }
            };

            let embedding = if let Some(emb) = embeddings.into_iter().next() {
                emb
            } else {
                error!("Vision embedding model returned empty results");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Vision embedding model returned empty results".to_string(),
                ));
            };

            (embedding, "image".to_string())
        }
        crate::types::QueryModality::Audio { location } => {
            info!("Processing audio query");

            // Extract audio path from source
            let (audio_path, _downloaded_file) = match location {
                MediaSource::Upload { location } => (PathBuf::from(location), None),
                MediaSource::Url { location } => {
                    info!("Downloading audio from URL: {}", location);
                    let downloaded = download_from_url(location).await.map_err(|e| {
                        error!("Failed to download from URL {}: {}", location, e);
                        (
                            StatusCode::BAD_REQUEST,
                            format!("Failed to download from URL: {e}"),
                        )
                    })?;
                    let path = downloaded.path().to_path_buf();
                    (path, Some(downloaded))
                }
                MediaSource::S3 { location } => {
                    info!("Downloading audio from S3: {}", location);
                    let downloaded = download_from_s3(location).await.map_err(|e| {
                        error!("Failed to download from S3 {}: {}", location, e);
                        (
                            StatusCode::BAD_REQUEST,
                            format!("Failed to download from S3: {e}"),
                        )
                    })?;
                    let path = downloaded.path().to_path_buf();
                    (path, Some(downloaded))
                }
            };

            // Validate input path exists
            if !audio_path.exists() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Input file does not exist: {}", audio_path.display()),
                ));
            }

            // Load audio at 48kHz mono for CLAP
            let audio_samples = match load_audio_for_embeddings(&audio_path) {
                Ok(samples) => samples,
                Err(e) => {
                    error!("Failed to load audio: {}", e);
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("Failed to load audio: {e}"),
                    ));
                }
            };

            // Load audio embeddings model
            let audio_config = video_audio_embeddings::AudioEmbeddingConfig::default();
            let mut audio_embeddings =
                match video_audio_embeddings::AudioEmbeddings::new(audio_config) {
                    Ok(ae) => ae,
                    Err(e) => {
                        error!("Failed to load audio embeddings model: {}", e);
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to load audio embeddings model: {e}"),
                        ));
                    }
                };

            // Extract embedding
            let embeddings = match audio_embeddings.extract_embeddings(&[audio_samples]) {
                Ok(embs) => embs,
                Err(e) => {
                    error!("Failed to extract audio embedding: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to extract audio embedding: {e}"),
                    ));
                }
            };

            let embedding = if let Some(emb) = embeddings.into_iter().next() {
                emb
            } else {
                error!("Audio embedding model returned empty results");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Audio embedding model returned empty results".to_string(),
                ));
            };

            (embedding, "audio".to_string())
        }
    };

    info!(
        "Generated query embedding of dimension {} for {} query",
        query_vector.len(),
        query_type
    );

    // Build filter
    let mut filter = std::collections::HashMap::with_capacity(3);
    if let Some(embedding_type) = &request.embedding_type {
        filter.insert("embedding_type".to_string(), embedding_type.clone());
    }
    if let Some(job_id) = &request.job_id {
        filter.insert("job_id".to_string(), job_id.clone());
    }

    // Search for similar vectors
    let search_filter = if filter.is_empty() {
        None
    } else {
        Some(filter)
    };
    let similar_results = match vector_storage
        .search_similar(&query_vector, request.limit, search_filter)
        .await
    {
        Ok(results) => results,
        Err(e) => {
            error!("Failed to search vectors: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to search vectors: {e}"),
            ));
        }
    };

    info!("Found {} similar results", similar_results.len());

    // Convert results to API format
    let results: Vec<crate::types::SearchResultItem> = similar_results
        .into_iter()
        .map(|result| {
            let job_id = result.metadata.get("job_id").cloned().unwrap_or_default();
            let embedding_type = result
                .metadata
                .get("embedding_type")
                .cloned()
                .unwrap_or_default();

            crate::types::SearchResultItem {
                vector_id: result.vector_id,
                score: result.score,
                job_id,
                embedding_type,
                metadata: result.metadata,
                vector: if request.include_vectors {
                    result.embedding.map(|e| e.vector)
                } else {
                    None
                },
            }
        })
        .collect();

    let count = results.len();

    Ok(Json(crate::types::SearchResponse {
        results,
        count,
        query_type,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_audio_common::MediaInfo;

    #[test]
    fn test_task_result_to_json_ingestion() {
        let result = TaskResult::Ingestion(MediaInfo {
            format: "mp4".to_string(),
            duration: 10.0,
            streams: vec![],
            metadata: std::collections::HashMap::new(),
        });

        let json = task_result_to_json(&result);
        assert_eq!(json["type"], "ingestion");
        assert_eq!(json["format"], "mp4");
        assert_eq!(json["duration"], 10.0);
    }

    #[test]
    fn test_task_result_to_json_audio_extraction() {
        let result = TaskResult::AudioExtraction(PathBuf::from("/tmp/audio.wav"));
        let json = task_result_to_json(&result);
        assert_eq!(json["type"], "audio_extraction");
        assert!(json["path"].as_str().unwrap().contains("audio.wav"));
    }

    #[test]
    fn test_task_result_to_json_keyframe_extraction() {
        let paths = vec![
            PathBuf::from("/tmp/frame1.jpg"),
            PathBuf::from("/tmp/frame2.jpg"),
        ];
        let result = TaskResult::KeyframeExtraction(paths);
        let json = task_result_to_json(&result);
        assert_eq!(json["type"], "keyframe_extraction");
        assert_eq!(json["num_keyframes"], 2);
    }

    #[test]
    fn test_task_result_to_json_storage() {
        let stats = video_audio_orchestrator::StorageStats {
            files_stored: 5,
            metadata_records: 1,
            embeddings_stored: 0,
        };
        let result = TaskResult::Storage(stats);
        let json = task_result_to_json(&result);
        assert_eq!(json["type"], "storage");
        assert_eq!(json["files_stored"], 5);
        assert_eq!(json["metadata_records"], 1);
    }
}
