//! Batch processing for multiple PDF documents
//!
//! This module provides background processing of PDF directories with
//! progress updates for real-time visualization.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::DlvizStage;

/// Status of a document in the batch queue
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BatchDocStatus {
    /// Waiting to be processed
    #[default]
    Queued = 0,
    /// Currently being processed
    Processing = 1,
    /// Successfully completed
    Completed = 2,
    /// Processing failed
    Failed = 3,
}

impl std::fmt::Display for BatchDocStatus {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Queued => "queued",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for BatchDocStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "queued" | "pending" | "waiting" => Ok(Self::Queued),
            "processing" | "running" | "active" => Ok(Self::Processing),
            "completed" | "done" | "success" | "finished" => Ok(Self::Completed),
            "failed" | "error" | "failure" => Ok(Self::Failed),
            _ => Err(format!(
                "unknown batch status: '{s}' (expected: queued, processing, completed, failed)"
            )),
        }
    }
}

/// Progress update for batch processing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchProgress {
    /// Document index in the batch (0-indexed)
    pub doc_index: usize,
    /// Document filename
    pub doc_name: String,
    /// Current page being processed (0-indexed)
    pub page_no: usize,
    /// Total pages in document
    pub total_pages: usize,
    /// Current processing stage
    pub stage: i32, // DlvizStage as i32 for FFI
    /// Document status
    pub status: BatchDocStatus,
    /// Error message if status is Failed
    pub error_message: Option<String>,
    /// Processing time for current page in milliseconds
    pub processing_time_ms: f64,
    /// Elements detected on current page
    pub elements_detected: usize,
}

/// Control command for batch processor
#[derive(Debug, Clone, PartialEq)]
pub enum BatchControl {
    /// Pause processing
    Pause,
    /// Resume processing
    Resume,
    /// Stop processing completely
    Stop,
    /// Set playback speed multiplier (1.0 = realtime)
    SetSpeed(f64),
}

impl std::fmt::Display for BatchControl {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pause => write!(f, "pause"),
            Self::Resume => write!(f, "resume"),
            Self::Stop => write!(f, "stop"),
            Self::SetSpeed(speed) => write!(f, "set speed {speed}x"),
        }
    }
}

impl std::str::FromStr for BatchControl {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_lowercase();

        // Simple variants
        match normalized.as_str() {
            "pause" | "p" => return Ok(Self::Pause),
            "resume" | "r" | "continue" => return Ok(Self::Resume),
            "stop" | "s" | "quit" | "exit" | "halt" => return Ok(Self::Stop),
            _ => {}
        }

        // SetSpeed variants:
        // "set speed 2x", "set speed 2.0x", "speed 2", "speed:2", "2x", "x2"
        if let Some(speed) = parse_speed(&normalized) {
            return Ok(Self::SetSpeed(speed));
        }

        Err(format!(
            "unknown batch control: '{s}' (expected: pause, resume, stop, or speed value like '2x' or 'set speed 1.5x')"
        ))
    }
}

/// Parse a speed value from various formats
fn parse_speed(s: &str) -> Option<f64> {
    // Remove common prefixes
    let s = s
        .strip_prefix("set speed")
        .or_else(|| s.strip_prefix("setspeed"))
        .or_else(|| s.strip_prefix("speed"))
        .unwrap_or(s)
        .trim();

    // Handle "speed:N" or "speed=N" format
    let s = s
        .strip_prefix(':')
        .or_else(|| s.strip_prefix('='))
        .unwrap_or(s)
        .trim();

    // Handle "Nx" format (e.g., "2x", "1.5x")
    let s = s.strip_suffix('x').unwrap_or(s).trim();

    // Handle "xN" format (e.g., "x2", "x1.5")
    let s = s.strip_prefix('x').unwrap_or(s).trim();

    // Parse the number
    s.parse::<f64>().ok().filter(|&v| v > 0.0 && v.is_finite())
}

/// Shared state for batch processing thread
#[derive(Debug, Clone)]
struct BatchThreadContext {
    /// Flag to indicate if processing is running
    is_running: Arc<AtomicBool>,
    /// Flag to indicate if processing is paused
    is_paused: Arc<AtomicBool>,
    /// Current playback speed (f64 bits stored as u64)
    playback_speed: Arc<AtomicU64>,
    /// Completed documents count
    completed_docs: Arc<AtomicU64>,
    /// Failed documents count
    failed_docs: Arc<AtomicU64>,
}

/// Batch processor that runs in a background thread
pub struct BatchProcessor {
    /// Receiver for progress updates (Swift polls this)
    progress_rx: Receiver<BatchProgress>,
    /// Sender for control commands
    control_tx: Sender<BatchControl>,
    /// Processing thread handle
    thread_handle: Option<JoinHandle<()>>,
    /// Flag to indicate if processing is running
    is_running: Arc<AtomicBool>,
    /// Flag to indicate if processing is paused
    is_paused: Arc<AtomicBool>,
    /// Current playback speed (atomic for thread-safe access)
    playback_speed: Arc<AtomicU64>, // f64 bits stored as u64
    /// Total documents in batch
    total_docs: usize,
    /// Completed documents count
    completed_docs: Arc<AtomicU64>,
    /// Failed documents count
    failed_docs: Arc<AtomicU64>,
}

impl BatchProcessor {
    /// Create a new batch processor
    #[must_use = "returns a new batch processor instance"]
    pub fn new() -> Self {
        let (_, progress_rx) = channel();
        let (control_tx, _) = channel();

        Self {
            progress_rx,
            control_tx,
            thread_handle: None,
            is_running: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
            playback_speed: Arc::new(AtomicU64::new(1.0_f64.to_bits())),
            total_docs: 0,
            completed_docs: Arc::new(AtomicU64::new(0)),
            failed_docs: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Start batch processing a directory of PDFs
    pub fn start(&mut self, input_dir: PathBuf, _output_dir: PathBuf) {
        // Stop any existing processing
        self.stop();

        // Create new channels
        let (progress_tx, progress_rx) = channel();
        let (control_tx, control_rx) = channel();

        self.progress_rx = progress_rx;
        self.control_tx = control_tx;

        // Reset state
        self.is_running.store(true, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        self.completed_docs.store(0, Ordering::SeqCst);
        self.failed_docs.store(0, Ordering::SeqCst);

        // Create context for thread
        let ctx = BatchThreadContext {
            is_running: Arc::clone(&self.is_running),
            is_paused: Arc::clone(&self.is_paused),
            playback_speed: Arc::clone(&self.playback_speed),
            completed_docs: Arc::clone(&self.completed_docs),
            failed_docs: Arc::clone(&self.failed_docs),
        };

        // Spawn processing thread
        self.thread_handle = Some(thread::spawn(move || {
            batch_processing_thread(input_dir, progress_tx, control_rx, ctx);
        }));
    }

    /// Poll for the next progress update (non-blocking)
    #[inline]
    #[must_use = "returns the next progress update if available"]
    pub fn poll_progress(&self) -> Option<BatchProgress> {
        match self.progress_rx.try_recv() {
            Ok(progress) => Some(progress),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                // Thread has finished
                None
            }
        }
    }

    /// Pause batch processing
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
        let _ = self.control_tx.send(BatchControl::Pause);
    }

    /// Resume batch processing
    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        let _ = self.control_tx.send(BatchControl::Resume);
    }

    /// Stop batch processing completely
    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        let _ = self.control_tx.send(BatchControl::Stop);

        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// Set playback speed multiplier
    pub fn set_speed(&self, speed: f64) {
        let clamped = speed.clamp(0.1, 10.0);
        self.playback_speed
            .store(clamped.to_bits(), Ordering::SeqCst);
        let _ = self.control_tx.send(BatchControl::SetSpeed(clamped));
    }

    /// Check if processing is currently running
    #[inline]
    #[must_use = "returns whether batch processing is currently running"]
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Check if processing is paused
    #[inline]
    #[must_use = "returns whether batch processing is paused"]
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// Get current playback speed
    #[inline]
    #[must_use = "returns the current playback speed multiplier"]
    pub fn get_speed(&self) -> f64 {
        f64::from_bits(self.playback_speed.load(Ordering::SeqCst))
    }

    /// Get total documents in batch
    #[inline]
    #[must_use = "returns the total number of documents in the batch"]
    pub const fn total_docs(&self) -> usize {
        self.total_docs
    }

    /// Get completed documents count
    #[inline]
    #[must_use = "returns the number of successfully completed documents"]
    pub fn completed_count(&self) -> usize {
        self.completed_docs.load(Ordering::SeqCst) as usize
    }

    /// Get failed documents count
    #[inline]
    #[must_use = "returns the number of failed documents"]
    pub fn failed_count(&self) -> usize {
        self.failed_docs.load(Ordering::SeqCst) as usize
    }
}

impl Default for BatchProcessor {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BatchProcessor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Background thread that processes PDFs
fn batch_processing_thread(
    input_dir: PathBuf,
    progress_tx: Sender<BatchProgress>,
    control_rx: Receiver<BatchControl>,
    ctx: BatchThreadContext,
) {
    let BatchThreadContext {
        is_running,
        is_paused,
        playback_speed,
        completed_docs,
        failed_docs,
    } = ctx;
    // Find all PDF files in the directory
    let pdfs: Vec<PathBuf> = find_pdfs(&input_dir);

    log::info!(
        "Batch processing: found {} PDFs in {:?}",
        pdfs.len(),
        input_dir
    );

    for (doc_idx, pdf_path) in pdfs.iter().enumerate() {
        // Check for stop signal
        if !is_running.load(Ordering::SeqCst) {
            log::info!("Batch processing stopped");
            break;
        }

        // Handle pause
        while is_paused.load(Ordering::SeqCst) && is_running.load(Ordering::SeqCst) {
            // Check for control messages while paused
            match control_rx.try_recv() {
                Ok(BatchControl::Resume) => break,
                Ok(BatchControl::Stop) => {
                    is_running.store(false, Ordering::SeqCst);
                    return;
                }
                Ok(BatchControl::SetSpeed(speed)) => {
                    playback_speed.store(speed.to_bits(), Ordering::SeqCst);
                }
                _ => {}
            }
            thread::sleep(Duration::from_millis(50));
        }

        // Check for control messages
        while let Ok(cmd) = control_rx.try_recv() {
            match cmd {
                BatchControl::Stop => {
                    is_running.store(false, Ordering::SeqCst);
                    return;
                }
                BatchControl::Pause => {
                    is_paused.store(true, Ordering::SeqCst);
                }
                BatchControl::Resume => {
                    is_paused.store(false, Ordering::SeqCst);
                }
                BatchControl::SetSpeed(speed) => {
                    playback_speed.store(speed.to_bits(), Ordering::SeqCst);
                }
            }
        }

        let doc_name = pdf_path.file_name().map_or_else(
            || format!("doc_{doc_idx}"),
            |n| n.to_string_lossy().to_string(),
        );

        // Send processing started
        let _ = progress_tx.send(BatchProgress {
            doc_index: doc_idx,
            doc_name: doc_name.clone(),
            page_no: 0,
            total_pages: 0,
            stage: DlvizStage::RawPdf as i32,
            status: BatchDocStatus::Processing,
            error_message: None,
            processing_time_ms: 0.0,
            elements_detected: 0,
        });

        // Process the document
        match process_single_document(pdf_path, doc_idx, &doc_name, &progress_tx, &playback_speed) {
            Ok(()) => {
                completed_docs.fetch_add(1, Ordering::SeqCst);
                let _ = progress_tx.send(BatchProgress {
                    doc_index: doc_idx,
                    doc_name: doc_name.clone(),
                    page_no: 0,
                    total_pages: 0,
                    stage: DlvizStage::ReadingOrder as i32,
                    status: BatchDocStatus::Completed,
                    error_message: None,
                    processing_time_ms: 0.0,
                    elements_detected: 0,
                });
            }
            Err(e) => {
                failed_docs.fetch_add(1, Ordering::SeqCst);
                let _ = progress_tx.send(BatchProgress {
                    doc_index: doc_idx,
                    doc_name,
                    page_no: 0,
                    total_pages: 0,
                    stage: DlvizStage::RawPdf as i32,
                    status: BatchDocStatus::Failed,
                    error_message: Some(e),
                    processing_time_ms: 0.0,
                    elements_detected: 0,
                });
            }
        }
    }

    is_running.store(false, Ordering::SeqCst);
    log::info!("Batch processing complete");
}

/// Find all PDF files in a directory recursively
fn find_pdfs(dir: &PathBuf) -> Vec<PathBuf> {
    let mut pdfs = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Recursively search subdirectories
                pdfs.extend(find_pdfs(&path));
            } else if path.extension().map(std::ffi::OsStr::to_ascii_lowercase)
                == Some("pdf".into())
            {
                pdfs.push(path);
            }
        }
    }

    // Sort by filename for consistent ordering
    pdfs.sort_by(|a, b| {
        a.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .cmp(&b.file_name().map(|n| n.to_string_lossy().to_lowercase()))
    });

    pdfs
}

/// Process a single PDF document with ML pipeline
#[cfg(feature = "pdf-ml")]
fn process_single_document(
    pdf_path: &std::path::Path,
    doc_idx: usize,
    doc_name: &str,
    progress_tx: &Sender<BatchProgress>,
    playback_speed: &Arc<AtomicU64>,
) -> Result<(), String> {
    use crate::pdf_state::PdfState;

    let path_str = pdf_path.to_string_lossy();
    let mut pdf_state =
        PdfState::load(&path_str).map_err(|e| format!("Failed to load {}: {}", doc_name, e))?;

    let total_pages = pdf_state.page_count();

    for page_num in 0..total_pages {
        let start = Instant::now();

        // Send page processing started
        let _ = progress_tx.send(BatchProgress {
            doc_index: doc_idx,
            doc_name: doc_name.to_string(),
            page_no: page_num,
            total_pages,
            stage: DlvizStage::LayoutDetection as i32,
            status: BatchDocStatus::Processing,
            error_message: None,
            processing_time_ms: 0.0,
            elements_detected: 0,
        });

        // Run ML pipeline on page (scale = 1.5 for good quality)
        pdf_state
            .run_ml_pipeline(page_num, 1.5)
            .map_err(|e| format!("ML pipeline failed on page {}: {}", page_num, e))?;

        let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Get element count from final stage
        let elements_detected = pdf_state
            .get_ml_snapshot(page_num, DlvizStage::FinalAssembly)
            .map(|s| s.elements.len())
            .unwrap_or(0);

        // Send page completed
        let _ = progress_tx.send(BatchProgress {
            doc_index: doc_idx,
            doc_name: doc_name.to_string(),
            page_no: page_num,
            total_pages,
            stage: DlvizStage::FinalAssembly as i32,
            status: BatchDocStatus::Processing,
            error_message: None,
            processing_time_ms,
            elements_detected,
        });

        // Apply playback speed delay
        let speed = f64::from_bits(playback_speed.load(Ordering::SeqCst));
        if speed < 1.0 {
            // Slow down - add artificial delay
            let delay_ms = (processing_time_ms * (1.0 / speed - 1.0)) as u64;
            if delay_ms > 0 {
                thread::sleep(Duration::from_millis(delay_ms));
            }
        }
    }

    Ok(())
}

/// Stub implementation when pdf-ml feature is not enabled
#[cfg(not(feature = "pdf-ml"))]
fn process_single_document(
    _pdf_path: &std::path::Path,
    doc_idx: usize,
    doc_name: &str,
    progress_tx: &Sender<BatchProgress>,
    playback_speed: &Arc<AtomicU64>,
) -> Result<(), String> {
    // Without pdf-ml, we can only report that processing is not available
    // but we'll simulate some progress for UI testing
    let total_pages = 5; // Simulated

    for page_num in 0..total_pages {
        let start = Instant::now();

        // Send page processing started
        let _ = progress_tx.send(BatchProgress {
            doc_index: doc_idx,
            doc_name: doc_name.to_string(),
            page_no: page_num,
            total_pages,
            stage: DlvizStage::LayoutDetection as i32,
            status: BatchDocStatus::Processing,
            error_message: None,
            processing_time_ms: 0.0,
            elements_detected: 0,
        });

        // Simulate processing time
        thread::sleep(Duration::from_millis(100));

        let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Send page completed
        let _ = progress_tx.send(BatchProgress {
            doc_index: doc_idx,
            doc_name: doc_name.to_string(),
            page_no: page_num,
            total_pages,
            stage: DlvizStage::FinalAssembly as i32,
            status: BatchDocStatus::Processing,
            error_message: None,
            processing_time_ms,
            elements_detected: 10 + page_num * 3, // Simulated elements
        });

        // Apply playback speed delay
        let speed = f64::from_bits(playback_speed.load(Ordering::SeqCst));
        if speed < 1.0 {
            let delay_ms = (processing_time_ms * (1.0 / speed - 1.0)) as u64;
            if delay_ms > 0 {
                thread::sleep(Duration::from_millis(delay_ms));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_processor_new() {
        let processor = BatchProcessor::new();
        assert!(!processor.is_running());
        assert!(!processor.is_paused());
        assert!((processor.get_speed() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_batch_status_serialization() {
        let progress = BatchProgress {
            doc_index: 0,
            doc_name: "test.pdf".to_string(),
            page_no: 1,
            total_pages: 10,
            stage: DlvizStage::LayoutDetection as i32,
            status: BatchDocStatus::Processing,
            error_message: None,
            processing_time_ms: 150.5,
            elements_detected: 25,
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("test.pdf"));
        assert!(json.contains("Processing"));
    }

    #[test]
    fn test_speed_clamping() {
        let processor = BatchProcessor::new();

        processor.set_speed(0.01);
        assert!((processor.get_speed() - 0.1).abs() < f64::EPSILON);

        processor.set_speed(100.0);
        assert!((processor.get_speed() - 10.0).abs() < f64::EPSILON);

        processor.set_speed(2.5);
        assert!((processor.get_speed() - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_batch_doc_status_display() {
        assert_eq!(format!("{}", BatchDocStatus::Queued), "queued");
        assert_eq!(format!("{}", BatchDocStatus::Processing), "processing");
        assert_eq!(format!("{}", BatchDocStatus::Completed), "completed");
        assert_eq!(format!("{}", BatchDocStatus::Failed), "failed");
    }

    #[test]
    fn test_batch_control_display() {
        assert_eq!(format!("{}", BatchControl::Pause), "pause");
        assert_eq!(format!("{}", BatchControl::Resume), "resume");
        assert_eq!(format!("{}", BatchControl::Stop), "stop");
        assert_eq!(format!("{}", BatchControl::SetSpeed(2.0)), "set speed 2x");
        assert_eq!(format!("{}", BatchControl::SetSpeed(0.5)), "set speed 0.5x");
    }

    #[test]
    fn test_batch_doc_status_from_str() {
        use std::str::FromStr;

        // Primary names
        assert_eq!(
            BatchDocStatus::from_str("queued").unwrap(),
            BatchDocStatus::Queued
        );
        assert_eq!(
            BatchDocStatus::from_str("processing").unwrap(),
            BatchDocStatus::Processing
        );
        assert_eq!(
            BatchDocStatus::from_str("completed").unwrap(),
            BatchDocStatus::Completed
        );
        assert_eq!(
            BatchDocStatus::from_str("failed").unwrap(),
            BatchDocStatus::Failed
        );

        // Aliases
        assert_eq!(
            BatchDocStatus::from_str("pending").unwrap(),
            BatchDocStatus::Queued
        );
        assert_eq!(
            BatchDocStatus::from_str("running").unwrap(),
            BatchDocStatus::Processing
        );
        assert_eq!(
            BatchDocStatus::from_str("done").unwrap(),
            BatchDocStatus::Completed
        );
        assert_eq!(
            BatchDocStatus::from_str("error").unwrap(),
            BatchDocStatus::Failed
        );

        // Case insensitive
        assert_eq!(
            BatchDocStatus::from_str("COMPLETED").unwrap(),
            BatchDocStatus::Completed
        );

        // Error case
        assert!(BatchDocStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_batch_doc_status_roundtrip() {
        use std::str::FromStr;

        for status in [
            BatchDocStatus::Queued,
            BatchDocStatus::Processing,
            BatchDocStatus::Completed,
            BatchDocStatus::Failed,
        ] {
            let s = status.to_string();
            let parsed = BatchDocStatus::from_str(&s).unwrap();
            assert_eq!(status, parsed, "roundtrip failed for {s}");
        }
    }

    #[test]
    fn test_batch_control_from_str() {
        use std::str::FromStr;

        // Primary names
        assert_eq!(
            BatchControl::from_str("pause").unwrap(),
            BatchControl::Pause
        );
        assert_eq!(
            BatchControl::from_str("resume").unwrap(),
            BatchControl::Resume
        );
        assert_eq!(BatchControl::from_str("stop").unwrap(), BatchControl::Stop);

        // Short aliases
        assert_eq!(BatchControl::from_str("p").unwrap(), BatchControl::Pause);
        assert_eq!(BatchControl::from_str("r").unwrap(), BatchControl::Resume);
        assert_eq!(BatchControl::from_str("s").unwrap(), BatchControl::Stop);

        // Additional aliases
        assert_eq!(
            BatchControl::from_str("continue").unwrap(),
            BatchControl::Resume
        );
        assert_eq!(BatchControl::from_str("quit").unwrap(), BatchControl::Stop);
        assert_eq!(BatchControl::from_str("exit").unwrap(), BatchControl::Stop);
        assert_eq!(BatchControl::from_str("halt").unwrap(), BatchControl::Stop);

        // SetSpeed variants
        assert_eq!(
            BatchControl::from_str("2x").unwrap(),
            BatchControl::SetSpeed(2.0)
        );
        assert_eq!(
            BatchControl::from_str("1.5x").unwrap(),
            BatchControl::SetSpeed(1.5)
        );
        assert_eq!(
            BatchControl::from_str("speed 2").unwrap(),
            BatchControl::SetSpeed(2.0)
        );
        assert_eq!(
            BatchControl::from_str("speed:1.5").unwrap(),
            BatchControl::SetSpeed(1.5)
        );
        assert_eq!(
            BatchControl::from_str("set speed 2x").unwrap(),
            BatchControl::SetSpeed(2.0)
        );
        assert_eq!(
            BatchControl::from_str("set speed 0.5x").unwrap(),
            BatchControl::SetSpeed(0.5)
        );

        // Case insensitive
        assert_eq!(
            BatchControl::from_str("PAUSE").unwrap(),
            BatchControl::Pause
        );
        assert_eq!(BatchControl::from_str("STOP").unwrap(), BatchControl::Stop);
        assert_eq!(
            BatchControl::from_str("Set Speed 2X").unwrap(),
            BatchControl::SetSpeed(2.0)
        );

        // Error cases
        assert!(BatchControl::from_str("invalid").is_err());
        assert!(BatchControl::from_str("speed -1").is_err()); // Negative speed
        assert!(BatchControl::from_str("").is_err());
    }

    #[test]
    fn test_batch_control_roundtrip() {
        use std::str::FromStr;

        // Simple variants roundtrip perfectly
        for control in [
            BatchControl::Pause,
            BatchControl::Resume,
            BatchControl::Stop,
        ] {
            let s = control.to_string();
            let parsed = BatchControl::from_str(&s).unwrap();
            assert_eq!(control, parsed, "roundtrip failed for {s}");
        }

        // SetSpeed roundtrips (Display: "set speed 2x", FromStr parses it back)
        let speed_control = BatchControl::SetSpeed(2.0);
        let s = speed_control.to_string();
        assert_eq!(s, "set speed 2x");
        let parsed = BatchControl::from_str(&s).unwrap();
        assert_eq!(speed_control, parsed);

        // Test various speed values
        for speed in [0.5, 1.0, 1.5, 2.0, 5.0, 10.0] {
            let control = BatchControl::SetSpeed(speed);
            let s = control.to_string();
            let parsed = BatchControl::from_str(&s).unwrap();
            if let BatchControl::SetSpeed(parsed_speed) = parsed {
                assert!(
                    (speed - parsed_speed).abs() < f64::EPSILON,
                    "speed roundtrip failed for {speed}"
                );
            } else {
                panic!("expected SetSpeed variant");
            }
        }
    }
}
