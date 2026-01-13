/// Main orchestrator binary
use std::path::PathBuf;
use tracing::{error, info};
use video_audio_ingestion::init;
use video_audio_orchestrator::Orchestrator;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Video Audio Extraction System v0.1.0");

    // Initialize FFmpeg
    if let Err(e) = init() {
        error!("Failed to initialize FFmpeg: {}", e);
        std::process::exit(1);
    }

    // Check for command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-media-file>", args[0]);
        std::process::exit(1);
    }

    let path = PathBuf::from(&args[1]);

    if !path.exists() {
        error!("File not found: {}", path.display());
        std::process::exit(1);
    }

    // Create orchestrator and build task graph
    let orchestrator = Orchestrator::new();
    let job_id = format!("job-{}", uuid::Uuid::new_v4());
    let graph = orchestrator.build_realtime_graph(job_id.clone(), path.clone());

    info!(
        "Created task graph with {} tasks for file: {}",
        graph.tasks().len(),
        path.display()
    );

    // Execute task graph
    info!("Starting execution...");
    match orchestrator.execute(graph).await {
        Ok(final_graph) => {
            info!("Job {} completed successfully!", job_id);
            println!("\n=== Processing Results ===");
            println!("Job ID: {}", final_graph.job_id);
            println!("Input: {}", final_graph.input_path.display());
            println!("Total tasks: {}", final_graph.tasks().len());
            println!("Completed: {}", final_graph.completed_tasks().len());
            println!("Failed: {}", final_graph.failed_tasks().len());

            // Print results for each task
            for (task_id, task) in final_graph.tasks() {
                println!("\nTask: {} ({})", task_id, task.task_type.name());
                println!("  Status: {:?}", task.state);
                if let Some(result) = &task.result {
                    match result {
                        video_audio_orchestrator::TaskResult::Ingestion(info) => {
                            println!(
                                "  Format: {}, Duration: {:.2}s, Streams: {}",
                                info.format,
                                info.duration,
                                info.streams.len()
                            );
                        }
                        video_audio_orchestrator::TaskResult::AudioExtraction(path) => {
                            println!("  Output: {}", path.display());
                        }
                        video_audio_orchestrator::TaskResult::KeyframeExtraction(paths) => {
                            println!("  Keyframes extracted: {}", paths.len());
                        }
                        video_audio_orchestrator::TaskResult::Transcription(text) => {
                            println!("  Transcript: {text}");
                        }
                        video_audio_orchestrator::TaskResult::Diarization(diarization) => {
                            println!("  Speakers identified: {}", diarization.speakers.len());
                            println!("  Speaker segments: {}", diarization.timeline.len());
                            for speaker in &diarization.speakers {
                                println!(
                                    "    {}: {:.2}s total",
                                    speaker.id, speaker.total_speaking_time
                                );
                            }
                        }
                        video_audio_orchestrator::TaskResult::ObjectDetection(count) => {
                            println!("  Objects detected: {count}");
                        }
                        video_audio_orchestrator::TaskResult::FaceDetection(faces) => {
                            let total_faces: usize = faces.iter().map(std::vec::Vec::len).sum();
                            println!(
                                "  Faces detected: {} across {} keyframes",
                                total_faces,
                                faces.len()
                            );
                        }
                        video_audio_orchestrator::TaskResult::OCR(text_regions) => {
                            let total_text_regions: usize =
                                text_regions.iter().map(std::vec::Vec::len).sum();
                            println!(
                                "  Text regions detected: {} across {} keyframes",
                                total_text_regions,
                                text_regions.len()
                            );
                        }
                        video_audio_orchestrator::TaskResult::SceneDetection(scene_result) => {
                            println!("  Total scenes: {}", scene_result.num_scenes);
                            println!("  Scene boundaries: {}", scene_result.boundaries.len());
                            for (i, boundary) in scene_result.boundaries.iter().enumerate() {
                                println!(
                                    "    Boundary {}: {:.2}s (score: {:.2})",
                                    i + 1,
                                    boundary.timestamp,
                                    boundary.score
                                );
                            }
                        }
                        video_audio_orchestrator::TaskResult::VisionEmbeddings(embeddings) => {
                            let dim = embeddings.first().map_or(0, std::vec::Vec::len);
                            println!(
                                "  Vision embeddings extracted: {} embeddings ({}-dim)",
                                embeddings.len(),
                                dim
                            );
                        }
                        video_audio_orchestrator::TaskResult::TextEmbeddings(embeddings) => {
                            let dim = embeddings.first().map_or(0, std::vec::Vec::len);
                            println!(
                                "  Text embeddings extracted: {} embeddings ({}-dim)",
                                embeddings.len(),
                                dim
                            );
                        }
                        video_audio_orchestrator::TaskResult::AudioEmbeddings(embeddings) => {
                            let dim = embeddings.first().map_or(0, std::vec::Vec::len);
                            println!(
                                "  Audio embeddings extracted: {} embeddings ({}-dim)",
                                embeddings.len(),
                                dim
                            );
                        }
                        video_audio_orchestrator::TaskResult::Fusion(timeline) => {
                            println!("  Timeline duration: {:.2}s", timeline.duration);
                            println!("  Events: {}", timeline.events.len());
                            println!("  Entities: {}", timeline.entities.len());
                            println!("  Relationships: {}", timeline.relationships.len());
                            println!("  Quality scores:");
                            println!("    Overall: {:.2}", timeline.quality_scores.overall);
                            println!(
                                "    Temporal alignment: {:.2}",
                                timeline.quality_scores.temporal_alignment
                            );
                            println!(
                                "    Cross-modal consistency: {:.2}",
                                timeline.quality_scores.cross_modal_consistency
                            );
                        }
                        video_audio_orchestrator::TaskResult::Storage(stats) => {
                            println!("  Files stored: {}", stats.files_stored);
                            println!("  Metadata records: {}", stats.metadata_records);
                            println!("  Embeddings: {}", stats.embeddings_stored);
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Job execution failed: {}", e);

            // Print partial results if available
            if let Some(status) = orchestrator.get_job_status(&job_id).await {
                println!("\n=== Partial Results ===");
                println!(
                    "Completed: {}/{}",
                    status.completed_tasks, status.total_tasks
                );
                println!("Failed: {}", status.failed_tasks);
            }

            std::process::exit(1);
        }
    }
}
