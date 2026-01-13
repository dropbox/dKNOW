//! Motion tracking module using ByteTrack algorithm
//!
//! This module provides object tracking across video frames by associating
//! detections with persistent track IDs.
//!
//! # Features
//! - Persistent object tracking across frames
//! - ByteTrack algorithm (high + low confidence matching)
//! - Kalman filter for motion prediction
//! - Track lifecycle management
//!
//! # Example
//! ```no_run
//! use video_audio_motion_tracking::{MotionTracker, MotionTrackingConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = MotionTrackingConfig::default();
//! let mut tracker = MotionTracker::new(config);
//!
//! // For each frame, pass detections
//! // let detections = ... // From object detection
//! // let tracks = tracker.update(&detections, frame_idx)?;
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

/// Motion tracking errors
#[derive(Debug, Error)]
pub enum MotionTrackingError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("No detections provided")]
    NoDetections,
    #[error("Invalid frame index: {0}")]
    InvalidFrameIndex(u32),
}

/// Motion tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionTrackingConfig {
    /// IoU threshold for high-confidence matching (default: 0.5)
    pub high_confidence_threshold: f32,
    /// IoU threshold for low-confidence matching (default: 0.3)
    pub low_confidence_threshold: f32,
    /// Detection confidence threshold for high-confidence detections (default: 0.6)
    pub detection_threshold_high: f32,
    /// Detection confidence threshold for low-confidence detections (default: 0.3)
    pub detection_threshold_low: f32,
    /// Maximum frames to keep track alive without detections (default: 30)
    pub max_age: u32,
    /// Minimum number of detections to confirm track (default: 3)
    pub min_hits: u32,
}

impl Default for MotionTrackingConfig {
    fn default() -> Self {
        Self {
            high_confidence_threshold: 0.5,
            low_confidence_threshold: 0.3,
            detection_threshold_high: 0.6,
            detection_threshold_low: 0.3,
            max_age: 30,
            min_hits: 3,
        }
    }
}

/// Bounding box (copied from object-detection for independence)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl BoundingBox {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    #[inline]
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    #[inline]
    pub fn iou(&self, other: &BoundingBox) -> f32 {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        let intersection_width = (x2 - x1).max(0.0);
        let intersection_height = (y2 - y1).max(0.0);
        let intersection_area = intersection_width * intersection_height;

        let union_area = self.area() + other.area() - intersection_area;

        if union_area > 0.0 {
            intersection_area / union_area
        } else {
            0.0
        }
    }
}

/// Detection from object detector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub class_id: u8,
    pub class_name: String,
    pub confidence: f32,
    pub bbox: BoundingBox,
    pub frame_idx: u32,
}

/// Kalman filter state for tracking
#[derive(Debug, Clone)]
struct KalmanState {
    /// State vector: [x, y, w, h, vx, vy]
    state: [f32; 6],
    /// Covariance matrix (simplified diagonal)
    covariance: [f32; 6],
}

impl KalmanState {
    fn new(bbox: &BoundingBox) -> Self {
        Self {
            state: [bbox.x, bbox.y, bbox.width, bbox.height, 0.0, 0.0],
            covariance: [1.0, 1.0, 1.0, 1.0, 10.0, 10.0],
        }
    }

    /// Predict next state
    fn predict(&mut self) {
        // Update position with velocity
        self.state[0] += self.state[4]; // x += vx
        self.state[1] += self.state[5]; // y += vy

        // Increase covariance (uncertainty)
        for i in 0..6 {
            self.covariance[i] += 1.0;
        }
    }

    /// Update state with new detection
    fn update(&mut self, bbox: &BoundingBox) {
        let kalman_gain = 0.5; // Simplified Kalman gain

        // Update position and size
        self.state[0] = self.state[0] + kalman_gain * (bbox.x - self.state[0]);
        self.state[1] = self.state[1] + kalman_gain * (bbox.y - self.state[1]);
        self.state[2] = self.state[2] + kalman_gain * (bbox.width - self.state[2]);
        self.state[3] = self.state[3] + kalman_gain * (bbox.height - self.state[3]);

        // Update velocity
        self.state[4] = kalman_gain * (bbox.x - self.state[0]);
        self.state[5] = kalman_gain * (bbox.y - self.state[1]);

        // Reduce covariance (certainty)
        for i in 0..6 {
            self.covariance[i] *= 1.0 - kalman_gain;
        }
    }

    /// Get predicted bounding box
    fn get_bbox(&self) -> BoundingBox {
        BoundingBox::new(self.state[0], self.state[1], self.state[2], self.state[3])
    }
}

/// A single tracked object
#[derive(Debug, Clone)]
struct TrackedObject {
    id: u32,
    class_id: u8,
    class_name: String,
    kalman: KalmanState,
    detections: Vec<Detection>,
    age: u32,
    hits: u32,
    time_since_update: u32,
}

impl TrackedObject {
    fn new(id: u32, detection: Detection) -> Self {
        let kalman = KalmanState::new(&detection.bbox);
        let class_id = detection.class_id;
        let class_name = detection.class_name.clone();

        Self {
            id,
            class_id,
            class_name,
            kalman,
            detections: vec![detection],
            age: 0,
            hits: 1,
            time_since_update: 0,
        }
    }

    fn predict(&mut self) {
        self.kalman.predict();
        self.age += 1;
        self.time_since_update += 1;
    }

    fn update(&mut self, detection: Detection) {
        self.kalman.update(&detection.bbox);
        self.detections.push(detection);
        self.hits += 1;
        self.time_since_update = 0;
    }

    fn is_confirmed(&self, min_hits: u32) -> bool {
        self.hits >= min_hits
    }

    fn is_dead(&self, max_age: u32) -> bool {
        self.time_since_update > max_age
    }
}

/// Motion tracker state
pub struct MotionTracker {
    config: MotionTrackingConfig,
    tracks: Vec<TrackedObject>,
    next_id: u32,
}

impl MotionTracker {
    /// Create new motion tracker
    pub fn new(config: MotionTrackingConfig) -> Self {
        info!("Creating motion tracker with config: {:?}", config);
        Self {
            config,
            tracks: Vec::with_capacity(50), // Pre-allocate for typical track count
            next_id: 1,
        }
    }

    /// Update tracker with new detections for current frame
    pub fn update(&mut self, detections: &[Detection]) -> Result<Vec<Track>, MotionTrackingError> {
        debug!("Updating tracker with {} detections", detections.len());

        // Predict all existing tracks
        for track in &mut self.tracks {
            track.predict();
        }

        // Split detections by confidence
        let (high_conf_dets, low_conf_dets): (Vec<_>, Vec<_>) = detections
            .iter()
            .partition(|d| d.confidence >= self.config.detection_threshold_high);

        // Match high-confidence detections with tracks
        let matched_high =
            self.match_detections(&high_conf_dets, self.config.high_confidence_threshold);

        // Update matched tracks
        let tracks_len = self.tracks.len();
        let mut unmatched_tracks: Vec<usize> = Vec::with_capacity(tracks_len);
        unmatched_tracks.extend(0..tracks_len);
        let mut unmatched_dets: Vec<&Detection> = high_conf_dets.clone();

        for (track_idx, det_idx) in &matched_high {
            self.tracks[*track_idx].update(high_conf_dets[*det_idx].clone());
            unmatched_tracks.retain(|&idx| idx != *track_idx);
            unmatched_dets.retain(|d| !std::ptr::eq(*d, high_conf_dets[*det_idx]));
        }

        // Try matching unmatched tracks with low-confidence detections
        let mut unmatched_track_refs: Vec<&TrackedObject> =
            Vec::with_capacity(unmatched_tracks.len());
        unmatched_track_refs.extend(unmatched_tracks.iter().map(|&idx| &self.tracks[idx]));

        let matched_low = self.match_tracks_to_detections(
            &unmatched_track_refs,
            &low_conf_dets,
            self.config.low_confidence_threshold,
        );

        for (track_idx_in_unmatched, det_idx) in &matched_low {
            let track_idx = unmatched_tracks[*track_idx_in_unmatched];
            self.tracks[track_idx].update(low_conf_dets[*det_idx].clone());
            unmatched_tracks.retain(|&idx| idx != track_idx);
        }

        // Create new tracks for unmatched high-confidence detections
        for det in unmatched_dets {
            let new_track = TrackedObject::new(self.next_id, (*det).clone());
            self.next_id += 1;
            self.tracks.push(new_track);
        }

        // Remove dead tracks
        self.tracks
            .retain(|track| !track.is_dead(self.config.max_age));

        // Return confirmed tracks
        let confirmed_tracks: Vec<Track> = self
            .tracks
            .iter()
            .filter(|t| t.is_confirmed(self.config.min_hits))
            .map(|t| Track {
                id: t.id,
                class_id: t.class_id,
                class_name: t.class_name.clone(),
                detections: t.detections.clone(),
                start_frame: t.detections.first().map(|d| d.frame_idx).unwrap_or(0),
                end_frame: t.detections.last().map(|d| d.frame_idx).unwrap_or(0),
                hits: t.hits,
                age: t.age,
            })
            .collect();

        debug!("Tracker state: {} tracks", self.tracks.len());
        Ok(confirmed_tracks)
    }

    /// Match detections to tracks using IoU
    fn match_detections(
        &self,
        detections: &[&Detection],
        iou_threshold: f32,
    ) -> Vec<(usize, usize)> {
        let mut matches = Vec::with_capacity(std::cmp::min(detections.len(), self.tracks.len()));
        let mut matched_det_indices = vec![false; detections.len()];
        let mut matched_track_indices = vec![false; self.tracks.len()];

        // Compute IoU matrix
        let mut iou_matrix = vec![vec![0.0; detections.len()]; self.tracks.len()];
        for (track_idx, track) in self.tracks.iter().enumerate() {
            let predicted_bbox = track.kalman.get_bbox();
            for (det_idx, det) in detections.iter().enumerate() {
                iou_matrix[track_idx][det_idx] = predicted_bbox.iou(&det.bbox);
            }
        }

        // Greedy matching (highest IoU first)
        loop {
            let mut best_iou = iou_threshold;
            let mut best_match: Option<(usize, usize)> = None;

            for track_idx in 0..self.tracks.len() {
                if matched_track_indices[track_idx] {
                    continue;
                }
                for (det_idx, &matched) in matched_det_indices.iter().enumerate() {
                    if matched {
                        continue;
                    }
                    let iou = iou_matrix[track_idx][det_idx];
                    if iou > best_iou {
                        best_iou = iou;
                        best_match = Some((track_idx, det_idx));
                    }
                }
            }

            match best_match {
                Some((track_idx, det_idx)) => {
                    matches.push((track_idx, det_idx));
                    matched_track_indices[track_idx] = true;
                    matched_det_indices[det_idx] = true;
                }
                None => break,
            }
        }

        matches
    }

    /// Match unmatched tracks to low-confidence detections
    fn match_tracks_to_detections(
        &self,
        tracks: &[&TrackedObject],
        detections: &[&Detection],
        iou_threshold: f32,
    ) -> Vec<(usize, usize)> {
        let mut matches = Vec::with_capacity(std::cmp::min(detections.len(), tracks.len()));
        let mut matched_det_indices = vec![false; detections.len()];
        let mut matched_track_indices = vec![false; tracks.len()];

        // Compute IoU matrix
        let mut iou_matrix = vec![vec![0.0; detections.len()]; tracks.len()];
        for (track_idx, track) in tracks.iter().enumerate() {
            let predicted_bbox = track.kalman.get_bbox();
            for (det_idx, det) in detections.iter().enumerate() {
                iou_matrix[track_idx][det_idx] = predicted_bbox.iou(&det.bbox);
            }
        }

        // Greedy matching (highest IoU first)
        loop {
            let mut best_iou = iou_threshold;
            let mut best_match: Option<(usize, usize)> = None;

            for track_idx in 0..tracks.len() {
                if matched_track_indices[track_idx] {
                    continue;
                }
                for (det_idx, &matched) in matched_det_indices.iter().enumerate() {
                    if matched {
                        continue;
                    }
                    let iou = iou_matrix[track_idx][det_idx];
                    if iou > best_iou {
                        best_iou = iou;
                        best_match = Some((track_idx, det_idx));
                    }
                }
            }

            match best_match {
                Some((track_idx, det_idx)) => {
                    matches.push((track_idx, det_idx));
                    matched_track_indices[track_idx] = true;
                    matched_det_indices[det_idx] = true;
                }
                None => break,
            }
        }

        matches
    }

    /// Get all current tracks (confirmed + tentative)
    pub fn get_all_tracks(&self) -> Vec<Track> {
        self.tracks
            .iter()
            .map(|t| Track {
                id: t.id,
                class_id: t.class_id,
                class_name: t.class_name.clone(),
                detections: t.detections.clone(),
                start_frame: t.detections.first().map(|d| d.frame_idx).unwrap_or(0),
                end_frame: t.detections.last().map(|d| d.frame_idx).unwrap_or(0),
                hits: t.hits,
                age: t.age,
            })
            .collect()
    }
}

/// A confirmed track across multiple frames
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: u32,
    pub class_id: u8,
    pub class_name: String,
    pub detections: Vec<Detection>,
    pub start_frame: u32,
    pub end_frame: u32,
    pub hits: u32,
    pub age: u32,
}

impl Track {
    /// Get track duration in frames
    pub fn duration(&self) -> u32 {
        self.end_frame - self.start_frame + 1
    }

    /// Get track trajectory (center points)
    pub fn trajectory(&self) -> Vec<(f32, f32)> {
        let mut trajectory = Vec::with_capacity(self.detections.len());
        trajectory.extend(self.detections.iter().map(|d| d.bbox.center()));
        trajectory
    }
}

/// Final result with all tracks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionTrackingResult {
    pub tracks: Vec<Track>,
    pub total_frames: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    fn create_detection(
        class_id: u8,
        class_name: &str,
        confidence: f32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        frame_idx: u32,
    ) -> Detection {
        Detection {
            class_id,
            class_name: class_name.to_string(),
            confidence,
            bbox: BoundingBox::new(x, y, w, h),
            frame_idx,
        }
    }

    #[test]
    fn test_tracker_creation() {
        let config = MotionTrackingConfig::default();
        let tracker = MotionTracker::new(config);
        assert_eq!(tracker.next_id, 1);
        assert!(tracker.tracks.is_empty());
    }

    #[test]
    fn test_single_detection() {
        let config = MotionTrackingConfig {
            min_hits: 1, // Accept track immediately
            ..Default::default()
        };
        let mut tracker = MotionTracker::new(config);

        let det = create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0);
        let tracks = tracker.update(&[det]).unwrap();

        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].class_name, "person");
        assert_eq!(tracks[0].start_frame, 0);
    }

    #[test]
    fn test_track_continuity() {
        let config = MotionTrackingConfig {
            min_hits: 2,
            ..Default::default()
        };
        let mut tracker = MotionTracker::new(config);

        // Frame 0: Create track
        let det0 = create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0);
        let tracks0 = tracker.update(&[det0]).unwrap();
        assert_eq!(tracks0.len(), 0); // Not confirmed yet (min_hits=2)

        // Frame 1: Track should match (IoU > threshold)
        let det1 = create_detection(0, "person", 0.8, 0.12, 0.21, 0.3, 0.4, 1);
        let tracks1 = tracker.update(&[det1]).unwrap();
        assert_eq!(tracks1.len(), 1); // Now confirmed
        assert_eq!(tracks1[0].detections.len(), 2);
        assert_eq!(tracks1[0].start_frame, 0);
        assert_eq!(tracks1[0].end_frame, 1);
    }

    #[test]
    fn test_multiple_objects() {
        let config = MotionTrackingConfig {
            min_hits: 1,
            ..Default::default()
        };
        let mut tracker = MotionTracker::new(config);

        // Frame 0: Two separate objects
        let det0_a = create_detection(0, "person", 0.8, 0.1, 0.2, 0.2, 0.3, 0);
        let det0_b = create_detection(1, "car", 0.9, 0.6, 0.5, 0.3, 0.4, 0);
        let tracks0 = tracker.update(&[det0_a, det0_b]).unwrap();

        assert_eq!(tracks0.len(), 2);
        assert_eq!(tracks0[0].id, 1);
        assert_eq!(tracks0[1].id, 2);
    }

    #[test]
    fn test_track_death() {
        let config = MotionTrackingConfig {
            min_hits: 1,
            max_age: 2, // Track dies after 2 frames without detections
            ..Default::default()
        };
        let mut tracker = MotionTracker::new(config);

        // Frame 0: Create track
        let det0 = create_detection(0, "person", 0.8, 0.1, 0.2, 0.3, 0.4, 0);
        tracker.update(&[det0]).unwrap();

        // Frame 1-2: No detections (track ages)
        tracker.update(&[]).unwrap();
        tracker.update(&[]).unwrap();

        // Frame 3: Track should be dead
        let tracks3 = tracker.update(&[]).unwrap();
        assert_eq!(tracks3.len(), 0);
    }

    #[test]
    fn test_iou_matching() {
        let bbox1 = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
        let bbox2 = BoundingBox::new(0.25, 0.25, 0.5, 0.5); // Overlapping

        let iou = bbox1.iou(&bbox2);
        assert!(iou > 0.0);
        assert!(iou < 1.0);
    }

    #[test]
    fn test_kalman_prediction() {
        let bbox = BoundingBox::new(0.1, 0.2, 0.3, 0.4);
        let mut kalman = KalmanState::new(&bbox);

        // Predict (should maintain position initially with zero velocity)
        kalman.predict();
        let predicted = kalman.get_bbox();
        assert!((predicted.x - 0.1).abs() < 0.01);
        assert!((predicted.y - 0.2).abs() < 0.01);
    }
}
