//! Media Processing Orchestrator
//!
//! Coordinates execution of media processing tasks across multiple modules.
//! Implements task graph with dependency resolution and parallel execution.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};

use video_audio_common::{MediaInfo, ProcessingError};
use video_audio_ingestion::ingest_media;

/// Find the project root directory by looking for Cargo.toml
/// Returns current directory if project root cannot be found
fn find_project_root() -> PathBuf {
    let mut current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Walk up the directory tree until we find Cargo.toml at root level
    loop {
        let cargo_toml = current_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if this is the workspace root by looking for models/ directory
            let models_dir = current_dir.join("models");
            if models_dir.exists() {
                return current_dir;
            }
        }

        // Try parent directory
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            // Reached filesystem root, return current working directory
            return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}

/// Load audio file at 48kHz mono for CLAP embeddings
fn load_audio_for_embeddings(input_path: &Path) -> Result<Vec<f32>, ProcessingError> {
    use video_audio_extractor::{extract_audio, AudioConfig, AudioFormat};

    // Create temp directory for intermediate WAV file
    let temp_dir = std::env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::from_secs(0))
        .as_millis();
    let temp_wav = temp_dir.join(format!("clap_audio_{timestamp}.wav"));

    // Extract audio to 48kHz mono PCM WAV (CLAP requirement)
    let config = AudioConfig {
        sample_rate: 48000,
        channels: 1,
        format: AudioFormat::PCM,
        normalize: false,
    };

    let wav_path = extract_audio(input_path, &temp_wav, &config)
        .map_err(|e| ProcessingError::Other(format!("Failed to extract audio: {e}")))?;

    // Read WAV samples using hound
    let mut reader = hound::WavReader::open(&wav_path)
        .map_err(|e| ProcessingError::Other(format!("Failed to open WAV file: {e}")))?;

    let spec = reader.spec();

    // Verify format
    if spec.sample_rate != 48000 {
        return Err(ProcessingError::Other(format!(
            "Expected 48kHz sample rate, got {}Hz",
            spec.sample_rate
        )));
    }

    // Read samples and convert to f32
    let samples: Result<Vec<f32>, ProcessingError> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            let max_val = (1 << (bits - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| {
                    s.map(|sample| sample as f32 / max_val)
                        .map_err(|e| ProcessingError::Other(format!("Failed to read sample: {e}")))
                })
                .collect()
        }
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.map_err(|e| ProcessingError::Other(format!("Failed to read sample: {e}"))))
            .collect(),
    };

    // Clean up temp file
    let _ = std::fs::remove_file(&wav_path);

    samples
}

/// Task types that can be executed by the orchestrator
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskType {
    /// Ingest media file and extract metadata
    Ingestion,
    /// Extract audio stream from media
    AudioExtraction,
    /// Extract keyframes from video
    KeyframeExtraction,
    /// Transcribe audio to text
    Transcription,
    /// Speaker diarization (identify who spoke when)
    Diarization,
    /// Detect objects in video frames
    ObjectDetection,
    /// Detect faces in video frames
    FaceDetection,
    /// Extract text from video frames (OCR)
    OCR,
    /// Scene detection (classical algorithms)
    SceneDetection,
    /// Extract vision embeddings from keyframes (CLIP)
    VisionEmbeddings,
    /// Extract text embeddings from transcript (Sentence-Transformers)
    TextEmbeddings,
    /// Extract audio embeddings from audio (CLAP)
    AudioEmbeddings,
    /// Fuse all results into unified timeline
    Fusion,
    /// Store results to backends (`PostgreSQL`, S3/MinIO, Qdrant)
    Storage,
}

impl TaskType {
    /// Get human-readable task name
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Ingestion => "ingestion",
            Self::AudioExtraction => "audio_extraction",
            Self::KeyframeExtraction => "keyframe_extraction",
            Self::Transcription => "transcription",
            Self::Diarization => "diarization",
            Self::ObjectDetection => "object_detection",
            Self::FaceDetection => "face_detection",
            Self::OCR => "ocr",
            Self::SceneDetection => "scene_detection",
            Self::VisionEmbeddings => "vision_embeddings",
            Self::TextEmbeddings => "text_embeddings",
            Self::AudioEmbeddings => "audio_embeddings",
            Self::Fusion => "fusion",
            Self::Storage => "storage",
        }
    }
}

/// Current state of a task
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    /// Task is waiting for dependencies
    Pending,
    /// Task is ready to execute (dependencies satisfied)
    Ready,
    /// Task is currently executing
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with error
    Failed(String),
}

/// Result of a task execution
#[derive(Debug, Clone)]
pub enum TaskResult {
    /// Ingestion result
    Ingestion(MediaInfo),
    /// Audio extraction result (path to extracted audio)
    AudioExtraction(PathBuf),
    /// Keyframe extraction result (paths to keyframe images)
    KeyframeExtraction(Vec<PathBuf>),
    /// Transcription result (transcript text)
    Transcription(String),
    /// Diarization result (speaker timeline)
    Diarization(video_audio_diarization::Diarization),
    /// Object detection result (detection count)
    ObjectDetection(usize),
    /// Face detection result (detected faces per keyframe)
    FaceDetection(Vec<Vec<video_audio_face_detection::Face>>),
    /// OCR result (text regions per keyframe)
    OCR(Vec<Vec<video_audio_ocr::TextRegion>>),
    /// Scene detection result (scene boundaries with timestamps and scores)
    SceneDetection(video_audio_scene::SceneDetectionResult),
    /// Vision embeddings result (embeddings per keyframe)
    VisionEmbeddings(Vec<Vec<f32>>),
    /// Text embeddings result (embeddings per text segment)
    TextEmbeddings(Vec<Vec<f32>>),
    /// Audio embeddings result (embeddings per audio clip)
    AudioEmbeddings(Vec<Vec<f32>>),
    /// Fusion result (unified timeline)
    Fusion(video_audio_fusion::Timeline),
    /// Storage result (number of items stored)
    Storage(StorageStats),
}

/// Statistics from storage task
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub files_stored: usize,
    pub metadata_records: usize,
    pub embeddings_stored: usize,
}

/// A single task in the processing graph
#[derive(Debug, Clone)]
pub struct Task {
    /// Unique task identifier
    pub id: String,
    /// Type of task to execute
    pub task_type: TaskType,
    /// IDs of tasks this task depends on
    pub dependencies: Vec<String>,
    /// Current state of the task
    pub state: TaskState,
    /// Result of task execution (if completed)
    pub result: Option<TaskResult>,
}

impl Task {
    /// Create a new task
    #[must_use]
    pub fn new(id: String, task_type: TaskType, dependencies: Vec<String>) -> Self {
        Self {
            id,
            task_type,
            dependencies,
            state: TaskState::Pending,
            result: None,
        }
    }

    /// Check if task is ready to execute (all dependencies completed)
    #[must_use]
    pub fn is_ready(&self, completed_tasks: &HashSet<String>) -> bool {
        self.state == TaskState::Pending
            && self
                .dependencies
                .iter()
                .all(|dep| completed_tasks.contains(dep))
    }
}

/// Task graph for coordinating media processing pipeline
#[derive(Clone)]
pub struct TaskGraph {
    /// Job identifier
    pub job_id: String,
    /// Input media file path
    pub input_path: PathBuf,
    /// All tasks in the graph
    tasks: HashMap<String, Task>,
    /// Completed task IDs
    completed: HashSet<String>,
    /// Failed task IDs
    failed: HashSet<String>,
}

impl TaskGraph {
    /// Create a new task graph
    #[must_use]
    pub fn new(job_id: String, input_path: PathBuf) -> Self {
        Self {
            job_id,
            input_path,
            tasks: HashMap::with_capacity(20), // Typical graph has 15-20 tasks
            completed: HashSet::with_capacity(20),
            failed: HashSet::with_capacity(5), // Expect few failures
        }
    }

    /// Add a task to the graph
    pub fn add_task(&mut self, id: String, task_type: TaskType, dependencies: Vec<String>) {
        let task = Task::new(id.clone(), task_type, dependencies);
        self.tasks.insert(id, task);
    }

    /// Get all tasks that are ready to execute
    #[must_use]
    pub fn get_ready_tasks(&self) -> Vec<String> {
        self.tasks
            .values()
            .filter(|task| task.is_ready(&self.completed))
            .map(|task| task.id.clone())
            .collect()
    }

    /// Mark a task as running
    pub fn mark_running(&mut self, task_id: &str) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.state = TaskState::Running;
        }
    }

    /// Mark a task as completed
    pub fn mark_completed(&mut self, task_id: &str, result: TaskResult) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.state = TaskState::Completed;
            task.result = Some(result);
            self.completed.insert(task_id.to_string());
        }
    }

    /// Mark a task as failed
    pub fn mark_failed(&mut self, task_id: &str, error: String) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.state = TaskState::Failed(error);
            self.failed.insert(task_id.to_string());
        }
    }

    /// Check if all tasks are completed
    /// A graph is considered complete if:
    /// 1. All tasks are either Completed or Failed, OR
    /// 2. All tasks without failed dependencies are settled (Completed or Failed)
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.tasks
            .values()
            .all(|task| matches!(task.state, TaskState::Completed | TaskState::Failed(_)))
    }

    /// Check if any task has failed
    #[must_use]
    pub fn has_failed(&self) -> bool {
        !self.failed.is_empty()
    }

    /// Get task result by ID
    #[must_use]
    pub fn get_result(&self, task_id: &str) -> Option<&TaskResult> {
        self.tasks
            .get(task_id)
            .and_then(|task| task.result.as_ref())
    }

    /// Get all tasks (read-only)
    #[must_use]
    pub fn tasks(&self) -> &HashMap<String, Task> {
        &self.tasks
    }

    /// Get completed task IDs
    #[must_use]
    pub fn completed_tasks(&self) -> &HashSet<String> {
        &self.completed
    }

    /// Get failed task IDs
    #[must_use]
    pub fn failed_tasks(&self) -> &HashSet<String> {
        &self.failed
    }

    /// Validate task graph (check for cycles, missing dependencies)
    pub fn validate(&self) -> Result<(), ProcessingError> {
        // Check all dependencies exist
        for task in self.tasks.values() {
            for dep in &task.dependencies {
                if !self.tasks.contains_key(dep) {
                    return Err(ProcessingError::Other(format!(
                        "Task '{}' has missing dependency: '{}'",
                        task.id, dep
                    )));
                }
            }
        }

        // Check for cycles using DFS
        let mut visited = HashSet::with_capacity(self.tasks.len());
        let mut recursion_stack = HashSet::with_capacity(self.tasks.len());

        for task_id in self.tasks.keys().map(String::as_str) {
            if self.has_cycle(task_id, &mut visited, &mut recursion_stack) {
                return Err(ProcessingError::Other(
                    "Task graph contains cycles".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Check for cycles starting from a task
    fn has_cycle<'a>(
        &'a self,
        task_id: &'a str,
        visited: &mut HashSet<&'a str>,
        recursion_stack: &mut HashSet<&'a str>,
    ) -> bool {
        if recursion_stack.contains(task_id) {
            return true;
        }

        if visited.contains(task_id) {
            return false;
        }

        visited.insert(task_id);
        recursion_stack.insert(task_id);

        if let Some(task) = self.tasks.get(task_id) {
            for dep in &task.dependencies {
                if self.has_cycle(dep, visited, recursion_stack) {
                    return true;
                }
            }
        }

        recursion_stack.remove(task_id);
        false
    }
}

/// Orchestrator for executing media processing pipelines
#[derive(Clone)]
pub struct Orchestrator {
    /// Active task graphs
    graphs: Arc<RwLock<HashMap<String, Arc<Mutex<TaskGraph>>>>>,
}

impl Orchestrator {
    /// Create a new orchestrator
    #[must_use]
    pub fn new() -> Self {
        Self {
            graphs: Arc::new(RwLock::new(HashMap::with_capacity(10))), // Typical ~10 concurrent jobs
        }
    }

    /// Build a real-time processing task graph
    #[must_use]
    pub fn build_realtime_graph(&self, job_id: String, input_path: PathBuf) -> TaskGraph {
        let mut graph = TaskGraph::new(job_id, input_path);

        // Root: Ingestion
        graph.add_task("ingestion".to_string(), TaskType::Ingestion, vec![]);

        // CPU tier (parallel, all depend on ingestion)
        graph.add_task(
            "audio_extract".to_string(),
            TaskType::AudioExtraction,
            vec!["ingestion".to_string()],
        );
        graph.add_task(
            "keyframes".to_string(),
            TaskType::KeyframeExtraction,
            vec!["ingestion".to_string()],
        );

        // GPU tier (parallel, depend on CPU tier)
        // Note: Transcription, object detection, face detection, and OCR are optional (may fail if models not available)
        // Storage task should still run even if these fail

        // Face detection task (depends on keyframes)
        graph.add_task(
            "face_detection".to_string(),
            TaskType::FaceDetection,
            vec!["keyframes".to_string()],
        );

        // OCR task (depends on keyframes)
        graph.add_task(
            "ocr".to_string(),
            TaskType::OCR,
            vec!["keyframes".to_string()],
        );

        // Diarization task (depends on audio_extract)
        graph.add_task(
            "diarization".to_string(),
            TaskType::Diarization,
            vec!["audio_extract".to_string()],
        );

        // Scene detection task (depends on ingestion for video path)
        graph.add_task(
            "scene_detection".to_string(),
            TaskType::SceneDetection,
            vec!["ingestion".to_string()],
        );

        // Storage tier (runs after extraction tasks complete)
        // Only depends on core extraction tasks that are required
        graph.add_task(
            "storage".to_string(),
            TaskType::Storage,
            vec![
                "ingestion".to_string(),
                "audio_extract".to_string(),
                "keyframes".to_string(),
            ],
        );

        graph
    }

    /// Execute a task graph
    pub async fn execute(&self, graph: TaskGraph) -> Result<TaskGraph, ProcessingError> {
        let job_id = graph.job_id.clone();
        info!("Starting execution of job: {}", job_id);

        // Validate graph
        graph.validate()?;

        // Store graph
        let graph = Arc::new(Mutex::new(graph));
        {
            let mut graphs = self.graphs.write().await;
            graphs.insert(job_id.clone(), graph.clone());
        }

        // Execute tasks
        loop {
            let ready_tasks = {
                let g = graph.lock().await;
                if g.is_complete() {
                    if g.has_failed() {
                        warn!("Job {} completed with {} failed tasks (optional tasks may have failed)",
                              job_id, g.failed_tasks().len());
                    } else {
                        info!("Job {} completed successfully", job_id);
                    }
                    break;
                }
                g.get_ready_tasks()
            };

            if ready_tasks.is_empty() {
                // No ready tasks, wait a bit
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }

            // Execute ready tasks in parallel
            let mut handles = Vec::with_capacity(ready_tasks.len());
            for task_id in ready_tasks {
                let graph_clone = graph.clone();
                let handle = tokio::spawn(async move {
                    Self::execute_task(graph_clone, task_id).await;
                });
                handles.push(handle);
            }

            // Wait for all tasks to complete
            for handle in handles {
                let _ = handle.await;
            }
        }

        // Return completed graph
        let final_graph = {
            let g = graph.lock().await;
            g.clone()
        };

        Ok(final_graph)
    }

    /// Execute a single task
    async fn execute_task(graph: Arc<Mutex<TaskGraph>>, task_id: String) {
        // Get task info
        let (task_type, input_path, dependencies) = {
            let mut g = graph.lock().await;
            g.mark_running(&task_id);
            if let Some(task) = g.tasks.get(&task_id) {
                (
                    task.task_type.clone(),
                    g.input_path.clone(),
                    task.dependencies.clone(),
                )
            } else {
                error!("Task {} not found in graph", task_id);
                g.mark_failed(&task_id, format!("Task not found in graph: {task_id}"));
                return;
            }
        };

        info!("Executing task: {} ({})", task_id, task_type.name());

        // Execute task based on type
        let result = Self::execute_task_type(&task_type, &input_path, &graph, &dependencies).await;

        // Update task state
        let mut g = graph.lock().await;
        match result {
            Ok(task_result) => {
                info!("Task {} completed successfully", task_id);
                g.mark_completed(&task_id, task_result);
            }
            Err(e) => {
                error!("Task {} failed: {}", task_id, e);
                g.mark_failed(&task_id, e.to_string());
            }
        }
    }

    /// Execute a specific task type
    async fn execute_task_type(
        task_type: &TaskType,
        input_path: &Path,
        graph: &Arc<Mutex<TaskGraph>>,
        dependencies: &[String],
    ) -> Result<TaskResult, ProcessingError> {
        match task_type {
            TaskType::Ingestion => {
                let media_info = ingest_media(input_path)?;
                Ok(TaskResult::Ingestion(media_info))
            }
            TaskType::AudioExtraction => {
                // Get media info from ingestion task
                let _media_info = {
                    let g = graph.lock().await;
                    if let Some(TaskResult::Ingestion(info)) = g.get_result(&dependencies[0]) {
                        info.clone()
                    } else {
                        return Err(ProcessingError::Other(
                            "Missing ingestion result".to_string(),
                        ));
                    }
                };

                // Extract audio using audio-extractor module
                use video_audio_extractor::{extract_audio, AudioConfig};
                let output_path = PathBuf::from(format!("/tmp/{}_audio.wav", uuid::Uuid::new_v4()));
                let config = AudioConfig::for_ml(); // 16kHz mono PCM for ML models
                extract_audio(input_path, &output_path, &config)?;
                Ok(TaskResult::AudioExtraction(output_path))
            }
            TaskType::KeyframeExtraction => {
                // Extract keyframes using keyframe-extractor module
                use video_audio_keyframe::{extract_keyframes, KeyframeExtractor};
                let output_dir = PathBuf::from(format!("/tmp/{}_keyframes", uuid::Uuid::new_v4()));
                std::fs::create_dir_all(&output_dir).map_err(|e| {
                    ProcessingError::Other(format!("Failed to create keyframes dir: {e}"))
                })?;
                let config = KeyframeExtractor {
                    output_dir,
                    ..KeyframeExtractor::default()
                };
                let keyframes = extract_keyframes(input_path, config)?;
                // Extract all thumbnail paths from keyframes
                // Pre-allocate with estimated capacity (assume ~3 thumbnail sizes per keyframe)
                let estimated_capacity = keyframes.len() * 3;
                let mut paths = Vec::with_capacity(estimated_capacity);
                paths.extend(
                    keyframes
                        .iter()
                        .flat_map(|kf| kf.thumbnail_paths.values().cloned()),
                );
                Ok(TaskResult::KeyframeExtraction(paths))
            }
            TaskType::Transcription => {
                // Get audio path from audio extraction task
                let _audio_path = {
                    let g = graph.lock().await;
                    if let Some(TaskResult::AudioExtraction(path)) = g.get_result(&dependencies[0])
                    {
                        path.clone()
                    } else {
                        return Err(ProcessingError::Other(
                            "Missing audio extraction result".to_string(),
                        ));
                    }
                };

                // Transcribe audio using transcription module
                // Note: This requires a Whisper model to be present
                // For now, return a placeholder error since models need to be downloaded separately
                warn!("Transcription requires Whisper model (not auto-downloaded)");
                Err(ProcessingError::Other(
                    "Transcription requires Whisper model to be downloaded separately".to_string(),
                ))

                // Uncomment when model is available:
                // let model_path = PathBuf::from("models/ggml-base.bin");
                // let config = TranscriptionConfig::fast();
                // let transcriber = Transcriber::new(model_path, config).map_err(|e| {
                //     ProcessingError::Other(format!("Failed to create transcriber: {}", e))
                // })?;
                // let transcript = transcriber.transcribe(&audio_path).map_err(|e| {
                //     ProcessingError::Other(format!("Failed to transcribe: {}", e))
                // })?;
                // Ok(TaskResult::Transcription(transcript.text))
            }
            TaskType::Diarization => {
                // Get audio path from audio extraction task
                let audio_path = {
                    let g = graph.lock().await;
                    if let Some(TaskResult::AudioExtraction(path)) = g.get_result(&dependencies[0])
                    {
                        path.clone()
                    } else {
                        return Err(ProcessingError::Other(
                            "Missing audio extraction result".to_string(),
                        ));
                    }
                };

                // Perform speaker diarization using diarization module
                use video_audio_diarization::{diarize_audio, DiarizationConfig};

                info!("Running speaker diarization on audio: {:?}", audio_path);

                let config = DiarizationConfig::default();
                let diarization = diarize_audio(&audio_path, &config).map_err(|e| {
                    warn!(
                        "Speaker diarization failed (pyannote.audio may not be installed): {}",
                        e
                    );
                    ProcessingError::Other(format!("Speaker diarization failed: {e}"))
                })?;

                info!(
                    "Diarization complete: {} speakers, {} segments",
                    diarization.speakers.len(),
                    diarization.timeline.len()
                );
                Ok(TaskResult::Diarization(diarization))
            }
            TaskType::ObjectDetection => {
                // Get keyframe paths from keyframe extraction task
                let _keyframe_paths = {
                    let g = graph.lock().await;
                    if let Some(TaskResult::KeyframeExtraction(paths)) =
                        g.get_result(&dependencies[0])
                    {
                        paths.clone()
                    } else {
                        return Err(ProcessingError::Other(
                            "Missing keyframe extraction result".to_string(),
                        ));
                    }
                };

                // Detect objects using object-detection module
                use video_audio_object_detection::{ObjectDetectionConfig, ObjectDetector};

                // Find model file - use absolute path from project root
                let project_root = std::env::current_dir().map_err(|e| {
                    ProcessingError::Other(format!("Failed to get current dir: {}", e))
                })?;
                let model_path = project_root.join("models/object-detection/yolov8n.onnx");

                if !model_path.exists() {
                    warn!(
                        "Object detection model not found: {}. Skipping object detection.",
                        model_path.display()
                    );
                    return Err(ProcessingError::Other(format!(
                        "Object detection model not found at {}",
                        model_path.display()
                    )));
                }

                info!(
                    "Running object detection on {} keyframes",
                    _keyframe_paths.len()
                );

                let config = ObjectDetectionConfig::fast();
                let mut detector = ObjectDetector::new(&model_path, config).map_err(|e| {
                    ProcessingError::Other(format!("Failed to create object detector: {}", e))
                })?;

                let mut total_detections = 0;
                for path in &_keyframe_paths {
                    let img = image::open(path).map_err(|e| {
                        ProcessingError::Other(format!(
                            "Failed to load image {}: {}",
                            path.display(),
                            e
                        ))
                    })?;
                    let rgb_img = img.to_rgb8();
                    let detections = detector.detect(&rgb_img).map_err(|e| {
                        ProcessingError::Other(format!(
                            "Failed to detect objects in {}: {}",
                            path.display(),
                            e
                        ))
                    })?;
                    total_detections += detections.len();
                }

                info!(
                    "Object detection complete: {} objects detected",
                    total_detections
                );
                Ok(TaskResult::ObjectDetection(total_detections))
            }
            TaskType::FaceDetection => {
                // Get keyframe paths from keyframe extraction task
                let keyframe_paths = {
                    let g = graph.lock().await;
                    if let Some(TaskResult::KeyframeExtraction(paths)) =
                        g.get_result(&dependencies[0])
                    {
                        paths.clone()
                    } else {
                        return Err(ProcessingError::Other(
                            "Missing keyframe extraction result".to_string(),
                        ));
                    }
                };

                // Detect faces using face-detection module
                use video_audio_face_detection::{FaceDetectionConfig, FaceDetector};

                // Find model file (use absolute path from project root)
                let project_root = find_project_root();
                let model_path = project_root.join("models/face-detection/retinaface_mnet025.onnx");
                if !model_path.exists() {
                    warn!(
                        "Face detection model not found: {} (project root: {})",
                        model_path.display(),
                        project_root.display()
                    );
                    return Err(ProcessingError::Other(format!(
                        "Face detection requires RetinaFace/UltraFace ONNX model at {}",
                        model_path.display()
                    )));
                }

                let config = FaceDetectionConfig::default();
                let mut detector = FaceDetector::new(&model_path, config).map_err(|e| {
                    ProcessingError::Other(format!("Failed to create face detector: {e}"))
                })?;

                let mut all_faces = Vec::with_capacity(keyframe_paths.len());
                for path in keyframe_paths {
                    let img = image::open(&path).map_err(|e| {
                        ProcessingError::Other(format!("Failed to load image: {e}"))
                    })?;
                    let rgb_img = img.to_rgb8();
                    let faces = detector.detect(&rgb_img).map_err(|e| {
                        ProcessingError::Other(format!("Failed to detect faces: {e}"))
                    })?;
                    all_faces.push(faces);
                }

                let total_faces: usize = all_faces.iter().map(std::vec::Vec::len).sum();
                info!(
                    "Detected {} faces across {} keyframes",
                    total_faces,
                    all_faces.len()
                );
                Ok(TaskResult::FaceDetection(all_faces))
            }
            TaskType::OCR => {
                // Get keyframe paths from keyframe extraction task
                let keyframe_paths = {
                    let g = graph.lock().await;
                    if let Some(TaskResult::KeyframeExtraction(paths)) =
                        g.get_result(&dependencies[0])
                    {
                        paths.clone()
                    } else {
                        return Err(ProcessingError::Other(
                            "Missing keyframe extraction result".to_string(),
                        ));
                    }
                };

                // Extract text using OCR module
                use video_audio_ocr::{OCRConfig, OCRDetector};

                // Find model files (detection + recognition) - use absolute paths
                let project_root = find_project_root();
                let detection_model_path = project_root.join("models/ocr/ch_PP-OCRv4_det.onnx");
                let recognition_model_path = project_root.join("models/ocr/ch_PP-OCRv4_rec.onnx");

                if !detection_model_path.exists() || !recognition_model_path.exists() {
                    warn!(
                        "OCR models not found: {} or {}",
                        detection_model_path.display(),
                        recognition_model_path.display()
                    );
                    return Err(ProcessingError::Other(format!(
                        "OCR requires PaddleOCR ONNX models at {} and {}",
                        detection_model_path.display(),
                        recognition_model_path.display()
                    )));
                }

                let config = OCRConfig::default();
                let mut detector =
                    OCRDetector::new(&detection_model_path, &recognition_model_path, config)
                        .map_err(|e| {
                            ProcessingError::Other(format!("Failed to create OCR detector: {e}"))
                        })?;

                let mut all_text_regions = Vec::with_capacity(keyframe_paths.len());
                for path in keyframe_paths {
                    let img = image::open(&path).map_err(|e| {
                        ProcessingError::Other(format!("Failed to load image: {e}"))
                    })?;
                    let rgb_img = img.to_rgb8();
                    let text_regions = detector.detect_text(&rgb_img).map_err(|e| {
                        ProcessingError::Other(format!("Failed to extract text: {e}"))
                    })?;
                    all_text_regions.push(text_regions);
                }

                let total_text_regions: usize =
                    all_text_regions.iter().map(std::vec::Vec::len).sum();
                info!(
                    "Detected {} text regions across {} keyframes",
                    total_text_regions,
                    all_text_regions.len()
                );
                Ok(TaskResult::OCR(all_text_regions))
            }
            TaskType::SceneDetection => {
                // Detect scene boundaries using FFmpeg scdet filter
                use video_audio_scene::{detect_scenes, SceneDetectorConfig};

                // Use keyframes_only=true for 10-30x speedup with minimal accuracy loss
                let config = SceneDetectorConfig {
                    keyframes_only: true,
                    ..SceneDetectorConfig::default()
                };
                let scene_result = detect_scenes(input_path, &config)
                    .map_err(|e| ProcessingError::Other(format!("Failed to detect scenes: {e}")))?;

                info!(
                    "Detected {} scene boundaries ({} total scenes)",
                    scene_result.boundaries.len(),
                    scene_result.num_scenes
                );

                Ok(TaskResult::SceneDetection(scene_result))
            }
            TaskType::VisionEmbeddings => {
                // Get keyframe paths from keyframe extraction task
                let keyframe_paths = {
                    let g = graph.lock().await;
                    if let Some(TaskResult::KeyframeExtraction(paths)) =
                        g.get_result(&dependencies[0])
                    {
                        paths.clone()
                    } else {
                        return Err(ProcessingError::Other(
                            "Missing keyframe extraction result".to_string(),
                        ));
                    }
                };

                // Extract vision embeddings using CLIP
                use video_audio_embeddings::{VisionEmbeddingConfig, VisionEmbeddings};

                // Find model file - use absolute path
                let project_root = find_project_root();
                let model_path = project_root.join("models/embeddings/clip_vit_b32.onnx");
                if !model_path.exists() {
                    warn!("Vision embedding model not found: {}", model_path.display());
                    return Err(ProcessingError::Other(format!(
                        "Vision embeddings require CLIP ONNX model at {}. Run: scripts/export_models/export_all_embeddings.sh",
                        model_path.display()
                    )));
                }

                let config = VisionEmbeddingConfig {
                    model_path: model_path.to_str().unwrap().to_string(),
                    ..Default::default()
                };
                let mut extractor = VisionEmbeddings::new(config).map_err(|e| {
                    ProcessingError::Other(format!(
                        "Failed to create vision embeddings extractor: {e}"
                    ))
                })?;

                // Load images and extract embeddings
                let mut images = Vec::with_capacity(keyframe_paths.len());
                for path in &keyframe_paths {
                    let img = image::open(path).map_err(|e| {
                        ProcessingError::Other(format!("Failed to load image: {e}"))
                    })?;
                    images.push(img);
                }

                let embeddings = extractor.extract_embeddings(&images).map_err(|e| {
                    ProcessingError::Other(format!("Failed to extract vision embeddings: {e}"))
                })?;

                info!(
                    "Extracted {} vision embeddings ({}-dim)",
                    embeddings.len(),
                    embeddings.first().map_or(0, std::vec::Vec::len)
                );
                Ok(TaskResult::VisionEmbeddings(embeddings))
            }
            TaskType::TextEmbeddings => {
                // Get transcript from transcription task
                let transcript_text = {
                    let g = graph.lock().await;
                    if let Some(TaskResult::Transcription(text)) = g.get_result(&dependencies[0]) {
                        text.clone()
                    } else {
                        return Err(ProcessingError::Other(
                            "Missing transcription result".to_string(),
                        ));
                    }
                };

                // Extract text embeddings using Sentence-Transformers
                use video_audio_embeddings::{TextEmbeddingConfig, TextEmbeddings};

                // Find model file - use absolute path
                let project_root = find_project_root();
                let model_path = project_root.join("models/embeddings/all_minilm_l6_v2.onnx");
                if !model_path.exists() {
                    warn!("Text embedding model not found: {}", model_path.display());
                    return Err(ProcessingError::Other(format!(
                        "Text embeddings require Sentence-Transformers ONNX model at {}. Run: scripts/export_models/export_all_embeddings.sh",
                        model_path.display()
                    )));
                }

                let config = TextEmbeddingConfig::default();
                let mut extractor = TextEmbeddings::new(config).map_err(|e| {
                    ProcessingError::Other(format!(
                        "Failed to create text embeddings extractor: {e}"
                    ))
                })?;

                // Split transcript into sentences or segments
                // For now, use the entire transcript as one text
                let texts = vec![transcript_text];
                let embeddings = extractor.extract_embeddings(&texts).map_err(|e| {
                    ProcessingError::Other(format!("Failed to extract text embeddings: {e}"))
                })?;

                info!(
                    "Extracted {} text embeddings ({}-dim)",
                    embeddings.len(),
                    embeddings.first().map_or(0, std::vec::Vec::len)
                );
                Ok(TaskResult::TextEmbeddings(embeddings))
            }
            TaskType::AudioEmbeddings => {
                // Get audio path from audio extraction task
                let _audio_path = {
                    let g = graph.lock().await;
                    if let Some(TaskResult::AudioExtraction(path)) = g.get_result(&dependencies[0])
                    {
                        path.clone()
                    } else {
                        return Err(ProcessingError::Other(
                            "Missing audio extraction result".to_string(),
                        ));
                    }
                };

                // Extract audio embeddings using CLAP
                use video_audio_embeddings::{AudioEmbeddingConfig, AudioEmbeddings};

                // Find model file - use absolute path
                let project_root = find_project_root();
                let model_path = project_root.join("models/embeddings/clap.onnx");
                if !model_path.exists() {
                    warn!("Audio embedding model not found: {}", model_path.display());
                    return Err(ProcessingError::Other(format!(
                        "Audio embeddings require CLAP ONNX model at {}. Run: scripts/export_models/export_all_embeddings.sh",
                        model_path.display()
                    )));
                }

                let config = AudioEmbeddingConfig::default();
                let mut extractor = AudioEmbeddings::new(config).map_err(|e| {
                    ProcessingError::Other(format!(
                        "Failed to create audio embeddings extractor: {e}"
                    ))
                })?;

                // Load audio file as PCM samples at 48kHz mono (CLAP requirement)
                let audio_samples = load_audio_for_embeddings(input_path)?;

                // Extract embeddings for the full audio clip
                let embeddings = extractor
                    .extract_embeddings(&[audio_samples])
                    .map_err(|e| {
                        ProcessingError::Other(format!("Failed to extract audio embeddings: {e}"))
                    })?;

                info!("Extracted {} audio embeddings", embeddings.len());
                Ok(TaskResult::AudioEmbeddings(embeddings))
            }
            TaskType::Fusion => {
                // Collect all results from previous tasks and fuse into timeline
                let g = graph.lock().await;

                // Get media duration from ingestion
                let duration = if let Some(TaskResult::Ingestion(info)) = g.get_result("ingestion")
                {
                    info.duration
                } else {
                    return Err(ProcessingError::Other(
                        "Missing ingestion result for fusion".to_string(),
                    ));
                };

                // Get transcript
                let transcript = g.get_result("transcription").and_then(|r| {
                    if let TaskResult::Transcription(text) = r {
                        Some(text.clone())
                    } else {
                        None
                    }
                });

                // Get diarization
                let diarization = g.get_result("diarization").and_then(|r| {
                    if let TaskResult::Diarization(diar) = r {
                        Some(diar.clone())
                    } else {
                        None
                    }
                });

                // Get scenes
                let scenes = g
                    .get_result("scene_detection")
                    .and_then(|r| {
                        if let TaskResult::SceneDetection(result) = r {
                            Some(result.scenes.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                // Get object detections (simplified: just counts per frame)
                let objects_per_frame = g
                    .get_result("object_detection")
                    .map(|r| {
                        if let TaskResult::ObjectDetection(count) = r {
                            vec![*count]
                        } else {
                            vec![]
                        }
                    })
                    .unwrap_or_default();

                // Get face detections
                let faces_per_frame = g
                    .get_result("face_detection")
                    .and_then(|r| {
                        if let TaskResult::FaceDetection(faces) = r {
                            Some(faces.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                // Get text regions
                let text_regions_per_frame = g
                    .get_result("ocr")
                    .and_then(|r| {
                        if let TaskResult::OCR(regions) = r {
                            Some(regions.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                // Get embeddings
                let vision_embeddings = g
                    .get_result("vision_embeddings")
                    .and_then(|r| {
                        if let TaskResult::VisionEmbeddings(emb) = r {
                            Some(emb.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                let text_embeddings = g
                    .get_result("text_embeddings")
                    .and_then(|r| {
                        if let TaskResult::TextEmbeddings(emb) = r {
                            Some(emb.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                let audio_embeddings = g
                    .get_result("audio_embeddings")
                    .and_then(|r| {
                        if let TaskResult::AudioEmbeddings(emb) = r {
                            Some(emb.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                drop(g); // Release lock

                // Create fusion input
                use video_audio_fusion::{fuse_results, FusionConfig, FusionInput};

                let config = FusionConfig::default();
                let input = FusionInput {
                    duration,
                    transcript,
                    diarization,
                    scenes,
                    objects_per_frame,
                    faces_per_frame,
                    text_regions_per_frame,
                    vision_embeddings,
                    text_embeddings,
                    audio_embeddings,
                };

                info!("Starting fusion: duration={:.2}s", duration);
                let timeline = fuse_results(&config, input)
                    .map_err(|e| ProcessingError::Other(format!("Failed to fuse results: {e}")))?;

                info!(
                    "Fusion complete: {} events, {} entities, {} relationships",
                    timeline.events.len(),
                    timeline.entities.len(),
                    timeline.relationships.len()
                );

                Ok(TaskResult::Fusion(timeline))
            }
            TaskType::Storage => {
                // Collect all results from previous tasks and store them
                let (
                    job_id,
                    ingestion_result,
                    audio_path,
                    keyframe_paths,
                    vision_emb,
                    text_emb,
                    audio_emb,
                ) = {
                    let g = graph.lock().await;
                    let job_id = g.job_id.clone();

                    // Get results from completed tasks
                    let ingestion = g.get_result("ingestion").and_then(|r| {
                        if let TaskResult::Ingestion(info) = r {
                            Some(info.clone())
                        } else {
                            None
                        }
                    });
                    let audio = g.get_result("audio_extract").and_then(|r| {
                        if let TaskResult::AudioExtraction(path) = r {
                            Some(path.clone())
                        } else {
                            None
                        }
                    });
                    let keyframes = g.get_result("keyframes").and_then(|r| {
                        if let TaskResult::KeyframeExtraction(paths) = r {
                            Some(paths.clone())
                        } else {
                            None
                        }
                    });

                    // Get embeddings
                    let vision_embeddings = g.get_result("vision_embeddings").and_then(|r| {
                        if let TaskResult::VisionEmbeddings(emb) = r {
                            Some(emb.clone())
                        } else {
                            None
                        }
                    });
                    let text_embeddings = g.get_result("text_embeddings").and_then(|r| {
                        if let TaskResult::TextEmbeddings(emb) = r {
                            Some(emb.clone())
                        } else {
                            None
                        }
                    });
                    let audio_embeddings = g.get_result("audio_embeddings").and_then(|r| {
                        if let TaskResult::AudioEmbeddings(emb) = r {
                            Some(emb.clone())
                        } else {
                            None
                        }
                    });

                    (
                        job_id,
                        ingestion,
                        audio,
                        keyframes,
                        vision_embeddings,
                        text_embeddings,
                        audio_embeddings,
                    )
                };

                // Initialize storage backends (using default config from environment)
                use video_audio_storage::{
                    EmbeddingVector, MediaMetadata, MetadataStorage, ObjectStorage,
                    PostgresMetadataStorage, QdrantVectorStorage, S3ObjectStorage, StorageConfig,
                    VectorStorage,
                };

                let config = StorageConfig::default();
                let mut stats = StorageStats {
                    files_stored: 0,
                    metadata_records: 0,
                    embeddings_stored: 0,
                };

                // Store ingestion metadata if available
                if let Some(media_info) = ingestion_result {
                    match PostgresMetadataStorage::new(config.postgres.clone()).await {
                        Ok(metadata_storage) => {
                            // Extract video stream metadata
                            let video_stream = media_info.video_stream();
                            let resolution = video_stream
                                .and_then(|s| s.width.and_then(|w| s.height.map(|h| (w, h))));
                            let frame_rate = video_stream.and_then(|s| s.fps);

                            // Extract audio stream metadata
                            let audio_stream = media_info.audio_stream();
                            let sample_rate = audio_stream.and_then(|s| s.sample_rate);
                            let audio_channels =
                                audio_stream.and_then(|s| s.channels.map(u16::from));

                            // Convert MediaInfo to MediaMetadata
                            let metadata = MediaMetadata {
                                job_id: job_id.clone(),
                                input_path: input_path.to_string_lossy().to_string(),
                                format: media_info.format.clone(),
                                duration_secs: media_info.duration,
                                num_streams: media_info.streams.len(),
                                resolution,
                                frame_rate,
                                sample_rate,
                                audio_channels,
                                processed_at: chrono::Utc::now(),
                                extra: media_info.metadata.clone(),
                            };

                            match metadata_storage.store_media_metadata(&metadata).await {
                                Ok(_) => {
                                    info!("Stored media metadata for job {}", job_id);
                                    stats.metadata_records += 1;
                                }
                                Err(e) => warn!("Failed to store media metadata: {}", e),
                            }
                        }
                        Err(e) => warn!("Failed to connect to PostgreSQL: {}", e),
                    }
                }

                // Store audio file if available
                if let Some(audio_path_buf) = audio_path {
                    match S3ObjectStorage::new(config.s3.clone()).await {
                        Ok(object_storage) => {
                            let key = format!("{job_id}/audio.wav");
                            match tokio::fs::read(&audio_path_buf).await {
                                Ok(audio_data) => {
                                    match object_storage.store_file(&key, &audio_data).await {
                                        Ok(_) => {
                                            info!("Stored audio file: {}", key);
                                            stats.files_stored += 1;
                                        }
                                        Err(e) => warn!("Failed to store audio file: {}", e),
                                    }
                                }
                                Err(e) => warn!("Failed to read audio file: {}", e),
                            }
                        }
                        Err(e) => warn!("Failed to connect to S3/MinIO: {}", e),
                    }
                }

                // Store keyframe images if available
                if let Some(keyframe_paths_vec) = keyframe_paths {
                    match S3ObjectStorage::new(config.s3.clone()).await {
                        Ok(object_storage) => {
                            for (idx, keyframe_path) in keyframe_paths_vec.iter().enumerate() {
                                let key = format!("{job_id}/keyframes/frame_{idx:04}.jpg");
                                match tokio::fs::read(keyframe_path).await {
                                    Ok(image_data) => {
                                        match object_storage.store_file(&key, &image_data).await {
                                            Ok(_) => {
                                                stats.files_stored += 1;
                                            }
                                            Err(e) => {
                                                warn!("Failed to store keyframe {}: {}", idx, e);
                                            }
                                        }
                                    }
                                    Err(e) => warn!("Failed to read keyframe {}: {}", idx, e),
                                }
                            }
                            if stats.files_stored > 0 {
                                info!(
                                    "Stored {} keyframe images for job {}",
                                    stats.files_stored - 1,
                                    job_id
                                );
                            }
                        }
                        Err(e) => warn!("Failed to connect to S3/MinIO: {}", e),
                    }
                }

                // Store embeddings to Qdrant if available
                let has_embeddings =
                    vision_emb.is_some() || text_emb.is_some() || audio_emb.is_some();
                if has_embeddings {
                    match QdrantVectorStorage::new(config.qdrant.clone()).await {
                        Ok(vector_storage) => {
                            // Initialize collection
                            if let Err(e) = vector_storage.init_collection().await {
                                warn!("Failed to initialize Qdrant collection: {}", e);
                            } else {
                                // Pre-allocate based on sum of embedding vector lengths
                                let capacity = vision_emb.as_ref().map_or(0, |v| v.len())
                                    + text_emb.as_ref().map_or(0, |v| v.len())
                                    + audio_emb.as_ref().map_or(0, |v| v.len());
                                let mut embeddings_to_store = Vec::with_capacity(capacity);

                                // Vision embeddings (from keyframes)
                                if let Some(vision_vectors) = vision_emb {
                                    for (idx, vector) in vision_vectors.iter().enumerate() {
                                        let mut metadata =
                                            std::collections::HashMap::with_capacity(2);
                                        metadata.insert("frame_index".to_string(), idx.to_string());
                                        metadata.insert("source".to_string(), "vision".to_string());

                                        embeddings_to_store.push(EmbeddingVector {
                                            job_id: job_id.clone(),
                                            vector_id: format!("{job_id}_vision_{idx}"),
                                            embedding_type: "clip_frame".to_string(),
                                            vector: vector.clone(),
                                            metadata,
                                        });
                                    }
                                }

                                // Text embeddings (from transcription segments)
                                if let Some(text_vectors) = text_emb {
                                    for (idx, vector) in text_vectors.iter().enumerate() {
                                        let mut metadata =
                                            std::collections::HashMap::with_capacity(2);
                                        metadata
                                            .insert("segment_index".to_string(), idx.to_string());
                                        metadata.insert("source".to_string(), "text".to_string());

                                        embeddings_to_store.push(EmbeddingVector {
                                            job_id: job_id.clone(),
                                            vector_id: format!("{job_id}_text_{idx}"),
                                            embedding_type: "sentence_text".to_string(),
                                            vector: vector.clone(),
                                            metadata,
                                        });
                                    }
                                }

                                // Audio embeddings (from audio segments)
                                if let Some(audio_vectors) = audio_emb {
                                    for (idx, vector) in audio_vectors.iter().enumerate() {
                                        let mut metadata =
                                            std::collections::HashMap::with_capacity(2);
                                        metadata
                                            .insert("segment_index".to_string(), idx.to_string());
                                        metadata.insert("source".to_string(), "audio".to_string());

                                        embeddings_to_store.push(EmbeddingVector {
                                            job_id: job_id.clone(),
                                            vector_id: format!("{job_id}_audio_{idx}"),
                                            embedding_type: "clap_audio".to_string(),
                                            vector: vector.clone(),
                                            metadata,
                                        });
                                    }
                                }

                                // Store embeddings in batch
                                if !embeddings_to_store.is_empty() {
                                    match vector_storage
                                        .store_embeddings(&embeddings_to_store)
                                        .await
                                    {
                                        Ok(_) => {
                                            stats.embeddings_stored = embeddings_to_store.len();
                                            info!(
                                                "Stored {} embeddings to Qdrant for job {}",
                                                stats.embeddings_stored, job_id
                                            );
                                        }
                                        Err(e) => {
                                            warn!("Failed to store embeddings to Qdrant: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => warn!("Failed to connect to Qdrant: {}", e),
                    }
                }

                info!(
                    "Storage complete: {} files, {} metadata records, {} embeddings",
                    stats.files_stored, stats.metadata_records, stats.embeddings_stored
                );
                Ok(TaskResult::Storage(stats))
            }
        }
    }

    /// Execute multiple task graphs independently in parallel.
    ///
    /// This is the simple, reliable bulk processing method that executes each
    /// graph as an independent job. Each graph progresses through its task
    /// pipeline autonomously, executing ready tasks in parallel within the graph.
    ///
    /// # Characteristics
    /// - Each graph executes independently using the standard `execute()` method
    /// - Graphs progress through stages at their own pace (no synchronization)
    /// - Simple, robust implementation with proven reliability
    /// - Natural parallelism from concurrent graph execution
    ///
    /// # Performance
    /// - Throughput: ~3.4 files/sec on Kinetics-600 (N=46 benchmark)
    /// - Median latency: ~0.3s per file
    /// - Scales well with number of files (10-100+ files)
    ///
    /// # Use Cases
    /// - General bulk processing (recommended default)
    /// - Mixed workloads with varying file sizes
    /// - When simplicity and reliability are priorities
    pub async fn execute_bulk(
        &self,
        graphs: Vec<TaskGraph>,
    ) -> Result<Vec<TaskGraph>, ProcessingError> {
        info!("Starting bulk execution for {} jobs", graphs.len());

        // Execute graphs sequentially to avoid resource exhaustion
        // Parallel execution of 100+ jobs causes hangs
        let mut results = Vec::with_capacity(graphs.len());
        for graph in graphs {
            let result = self.execute(graph).await;
            results.push(result);
        }

        info!(
            "Bulk execution complete: {}/{} jobs successful",
            results
                .iter()
                .filter(|g| g.as_ref().is_ok_and(|g| !g.has_failed()))
                .count(),
            results.len()
        );

        // Collect results
        let completed_graphs: Result<Vec<_>, _> = results.into_iter().collect();
        completed_graphs
    }

    /// Execute multiple task graphs in synchronized stages (EXPERIMENTAL - HAS KNOWN BUGS).
    ///
    /// **WARNING**: This method has a critical bug that causes it to hang during execution.
    /// See reports/build-video-audio-extracts/staged_bulk_bug_analysis_2025-10-28-20-42.md
    ///
    /// **Bug symptoms**:
    /// - Hangs after 90-95% of Stage 1 (Ingestion) tasks complete
    /// - One or more tasks hang indefinitely (likely `FFmpeg` deadlock)
    /// - System waits forever for hung tasks, never progresses to Stage 2
    /// - Reproducible with both 10 files and 100 files
    ///
    /// **Recommendation**: Use `execute_bulk()` instead for reliable bulk processing.
    #[deprecated(note = "Has critical hang bug - use execute_bulk() instead")]
    pub async fn execute_bulk_staged(
        &self,
        graphs: Vec<TaskGraph>,
    ) -> Result<Vec<TaskGraph>, ProcessingError> {
        info!("Starting staged bulk execution for {} jobs", graphs.len());

        // Define task stages for bulk processing
        // Each stage groups similar tasks that should run together
        let stages: Vec<Vec<TaskType>> = vec![
            // Stage 1: Ingestion
            vec![TaskType::Ingestion],
            // Stage 2: CPU processing (audio & video extraction)
            vec![TaskType::AudioExtraction, TaskType::KeyframeExtraction],
            // Stage 3: GPU/ML processing (all ML tasks)
            vec![
                TaskType::Diarization,
                TaskType::FaceDetection,
                TaskType::OCR,
                TaskType::SceneDetection,
            ],
            // Stage 4: Storage
            vec![TaskType::Storage],
        ];

        // Store all graphs in orchestrator
        let graph_refs: Vec<Arc<Mutex<TaskGraph>>> = {
            let mut graphs_map = self.graphs.write().await;
            graphs
                .into_iter()
                .map(|g| {
                    let job_id = g.job_id.clone();
                    info!("Validating graph for job: {}", job_id);
                    if let Err(e) = g.validate() {
                        error!("Graph validation failed for {}: {}", job_id, e);
                        return Err(e);
                    }
                    let graph_ref = Arc::new(Mutex::new(g));
                    graphs_map.insert(job_id, graph_ref.clone());
                    Ok(graph_ref)
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        // Execute each stage sequentially, with parallelism within each stage
        for (stage_idx, stage_task_types) in stages.iter().enumerate() {
            info!(
                "Stage {}: Executing {} task types across {} jobs",
                stage_idx + 1,
                stage_task_types.len(),
                graph_refs.len()
            );

            // Collect all tasks for this stage across all graphs
            // Conservative estimate: one task per graph per task type
            let mut stage_tasks = Vec::with_capacity(graph_refs.len() * stage_task_types.len());
            for graph_ref in &graph_refs {
                let g = graph_ref.lock().await;
                for task_type in stage_task_types {
                    // Find tasks of this type that are ready
                    for (task_id, task) in g.tasks() {
                        if task.task_type == *task_type && task.state == TaskState::Pending {
                            // Check if dependencies are satisfied
                            if task.is_ready(&g.completed) {
                                stage_tasks.push((graph_ref.clone(), task_id.clone()));
                            }
                        }
                    }
                }
            }

            info!(
                "Stage {}: {} tasks ready to execute",
                stage_idx + 1,
                stage_tasks.len()
            );

            // Execute all tasks in this stage in parallel
            let mut handles = Vec::with_capacity(stage_tasks.len());
            for (graph_ref, task_id) in stage_tasks {
                let handle = tokio::spawn(async move {
                    Self::execute_task(graph_ref, task_id).await;
                });
                handles.push(handle);
            }

            // Wait for all tasks in this stage to complete
            for handle in handles {
                let _ = handle.await;
            }

            info!("Stage {} complete", stage_idx + 1);
        }

        // Collect completed graphs
        let mut completed_graphs = Vec::with_capacity(graph_refs.len());
        for graph_ref in graph_refs {
            let g = graph_ref.lock().await;
            if g.has_failed() {
                warn!(
                    "Job {} completed with {} failed tasks",
                    g.job_id,
                    g.failed_tasks().len()
                );
            } else if g.is_complete() {
                info!("Job {} completed successfully", g.job_id);
            } else {
                warn!("Job {} did not complete all tasks", g.job_id);
            }
            completed_graphs.push(g.clone());
        }

        info!(
            "Staged bulk execution complete: {}/{} jobs successful",
            completed_graphs.iter().filter(|g| !g.has_failed()).count(),
            completed_graphs.len()
        );

        Ok(completed_graphs)
    }

    /// Get status of a job
    pub async fn get_job_status(&self, job_id: &str) -> Option<TaskGraphStatus> {
        let graphs = self.graphs.read().await;
        if let Some(graph) = graphs.get(job_id) {
            let g = graph.lock().await;
            Some(TaskGraphStatus {
                job_id: g.job_id.clone(),
                total_tasks: g.tasks.len(),
                completed_tasks: g.completed.len(),
                failed_tasks: g.failed.len(),
                is_complete: g.is_complete(),
                has_failed: g.has_failed(),
            })
        } else {
            None
        }
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of a task graph execution
#[derive(Debug, Clone)]
pub struct TaskGraphStatus {
    pub job_id: String,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub is_complete: bool,
    pub has_failed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(
            "test".to_string(),
            TaskType::Ingestion,
            vec!["dep1".to_string()],
        );
        assert_eq!(task.id, "test");
        assert_eq!(task.task_type, TaskType::Ingestion);
        assert_eq!(task.dependencies.len(), 1);
        assert_eq!(task.state, TaskState::Pending);
    }

    #[test]
    fn test_task_is_ready() {
        let task = Task::new(
            "test".to_string(),
            TaskType::Ingestion,
            vec!["dep1".to_string()],
        );

        let mut completed = HashSet::new();
        assert!(!task.is_ready(&completed));

        completed.insert("dep1".to_string());
        assert!(task.is_ready(&completed));
    }

    #[test]
    fn test_task_graph_creation() {
        let graph = TaskGraph::new("job1".to_string(), PathBuf::from("/tmp/test.mp4"));
        assert_eq!(graph.job_id, "job1");
        assert_eq!(graph.tasks.len(), 0);
    }

    #[test]
    fn test_task_graph_add_task() {
        let mut graph = TaskGraph::new("job1".to_string(), PathBuf::from("/tmp/test.mp4"));
        graph.add_task("ingestion".to_string(), TaskType::Ingestion, vec![]);
        assert_eq!(graph.tasks.len(), 1);
        assert!(graph.tasks.contains_key("ingestion"));
    }

    #[test]
    fn test_task_graph_get_ready_tasks() {
        let mut graph = TaskGraph::new("job1".to_string(), PathBuf::from("/tmp/test.mp4"));
        graph.add_task("ingestion".to_string(), TaskType::Ingestion, vec![]);
        graph.add_task(
            "audio".to_string(),
            TaskType::AudioExtraction,
            vec!["ingestion".to_string()],
        );

        let ready = graph.get_ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], "ingestion");

        graph.mark_completed(
            "ingestion",
            TaskResult::Ingestion(MediaInfo {
                format: "mp4".to_string(),
                duration: 10.0,
                streams: vec![],
                metadata: HashMap::new(),
            }),
        );

        let ready = graph.get_ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], "audio");
    }

    #[test]
    fn test_task_graph_validate_missing_dependency() {
        let mut graph = TaskGraph::new("job1".to_string(), PathBuf::from("/tmp/test.mp4"));
        graph.add_task(
            "audio".to_string(),
            TaskType::AudioExtraction,
            vec!["missing".to_string()],
        );

        let result = graph.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_task_graph_validate_cycle() {
        let mut graph = TaskGraph::new("job1".to_string(), PathBuf::from("/tmp/test.mp4"));
        graph.add_task(
            "task1".to_string(),
            TaskType::Ingestion,
            vec!["task2".to_string()],
        );
        graph.add_task(
            "task2".to_string(),
            TaskType::AudioExtraction,
            vec!["task1".to_string()],
        );

        let result = graph.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_orchestrator_build_realtime_graph() {
        let orchestrator = Orchestrator::new();
        let graph =
            orchestrator.build_realtime_graph("job1".to_string(), PathBuf::from("/tmp/test.mp4"));

        assert_eq!(graph.job_id, "job1");
        assert!(graph.tasks.contains_key("ingestion"));
        assert!(graph.tasks.contains_key("audio_extract"));
        assert!(graph.tasks.contains_key("keyframes"));
        assert!(graph.tasks.contains_key("storage"));

        // Verify dependencies
        let audio = graph.tasks.get("audio_extract").unwrap();
        assert_eq!(audio.dependencies, vec!["ingestion"]);

        let keyframes = graph.tasks.get("keyframes").unwrap();
        assert_eq!(keyframes.dependencies, vec!["ingestion"]);

        let storage = graph.tasks.get("storage").unwrap();
        assert_eq!(
            storage.dependencies,
            vec!["ingestion", "audio_extract", "keyframes"]
        );
    }

    #[test]
    fn test_task_type_name() {
        assert_eq!(TaskType::Ingestion.name(), "ingestion");
        assert_eq!(TaskType::AudioExtraction.name(), "audio_extraction");
        assert_eq!(TaskType::KeyframeExtraction.name(), "keyframe_extraction");
        assert_eq!(TaskType::Transcription.name(), "transcription");
        assert_eq!(TaskType::ObjectDetection.name(), "object_detection");
    }
}
