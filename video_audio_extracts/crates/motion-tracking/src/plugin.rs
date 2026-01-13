//! Plugin wrapper for motion tracking module

use crate::{Detection as TrackingDetection, MotionTracker, MotionTrackingConfig};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Instant;
use tracing::info;
use video_extract_core::context::Context;
use video_extract_core::error::PluginError;
use video_extract_core::plugin::{Plugin, PluginConfig, PluginData, PluginRequest, PluginResponse};
use video_extract_core::Operation;

/// Motion tracking plugin implementation
pub struct MotionTrackingPlugin {
    config: PluginConfig,
}

impl MotionTrackingPlugin {
    /// Create new motion tracking plugin
    pub fn new(config: PluginConfig) -> Self {
        Self { config }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        Ok(Self::new(config))
    }
}

#[async_trait]
impl Plugin for MotionTrackingPlugin {
    fn name(&self) -> &str {
        "motion-tracking"
    }

    fn config(&self) -> &PluginConfig {
        &self.config
    }

    fn supports_input(&self, input_type: &str) -> bool {
        input_type == "ObjectDetection"
    }

    fn produces_output(&self, output_type: &str) -> bool {
        output_type == "MotionTracking"
    }

    async fn execute(
        &self,
        ctx: &Context,
        request: &PluginRequest,
    ) -> Result<PluginResponse, PluginError> {
        let start = Instant::now();

        // Extract operation parameters
        let (
            high_confidence_threshold,
            low_confidence_threshold,
            detection_threshold_high,
            detection_threshold_low,
            max_age,
            min_hits,
        ) = match &request.operation {
            Operation::MotionTracking {
                high_confidence_threshold,
                low_confidence_threshold,
                detection_threshold_high,
                detection_threshold_low,
                max_age,
                min_hits,
            } => (
                *high_confidence_threshold,
                *low_confidence_threshold,
                *detection_threshold_high,
                *detection_threshold_low,
                *max_age,
                *min_hits,
            ),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected MotionTracking operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Motion tracking with thresholds: high_conf={:?}, low_conf={:?}",
                high_confidence_threshold, low_confidence_threshold
            );
        }

        // Get object detections from input
        let detections_per_frame: Vec<Vec<TrackingDetection>> = match &request.input {
            PluginData::Json(json_val) => {
                if ctx.verbose {
                    info!(
                        "Motion tracking: parsing JSON input, is_array={}",
                        json_val.is_array()
                    );
                }
                // Parse object detection results - handle both nested and flat formats
                // Nested format: {"detections": [[frame0], [frame1], ...]}
                // Flat format: [{det1, frame_idx: 0}, {det2, frame_idx: 0}, {det3, frame_idx: 1}, ...]
                let mut frames: Vec<Vec<TrackingDetection>> = Vec::with_capacity(100);

                // Try nested format first (legacy format with "detections" key)
                if let Some(detections_array) =
                    json_val.get("detections").and_then(|v| v.as_array())
                {
                    for (frame_idx, frame_detections) in detections_array.iter().enumerate() {
                        if let Some(dets) = frame_detections.as_array() {
                            let frame_dets: Vec<TrackingDetection> = dets
                                .iter()
                                .filter_map(|d| {
                                    let class_id = d.get("class_id")?.as_u64()? as u8;
                                    let class_name = d.get("class_name")?.as_str()?.to_string();
                                    let confidence = d.get("confidence")?.as_f64()? as f32;
                                    let bbox = d.get("bbox")?;
                                    let x = bbox.get("x")?.as_f64()? as f32;
                                    let y = bbox.get("y")?.as_f64()? as f32;
                                    let width = bbox.get("width")?.as_f64()? as f32;
                                    let height = bbox.get("height")?.as_f64()? as f32;

                                    Some(TrackingDetection {
                                        class_id,
                                        class_name,
                                        confidence,
                                        bbox: crate::BoundingBox::new(x, y, width, height),
                                        frame_idx: frame_idx as u32,
                                    })
                                })
                                .collect();

                            if frames.len() <= frame_idx {
                                frames.resize(frame_idx + 1, Vec::new());
                            }
                            frames[frame_idx] = frame_dets;
                        }
                    }
                }
                // Try flat array format (current object-detection output format)
                else if let Some(detections_array) = json_val.as_array() {
                    // Flat array of detections, each with a frame_idx field
                    // Group detections by frame_idx
                    for d in detections_array {
                        let class_id = match d.get("class_id").and_then(|v| v.as_u64()) {
                            Some(id) => id as u8,
                            None => continue, // Skip invalid detection
                        };
                        let class_name = match d.get("class_name").and_then(|v| v.as_str()) {
                            Some(name) => name.to_string(),
                            None => continue,
                        };
                        let confidence = match d.get("confidence").and_then(|v| v.as_f64()) {
                            Some(conf) => conf as f32,
                            None => continue,
                        };
                        let bbox = match d.get("bbox") {
                            Some(b) => b,
                            None => continue,
                        };
                        let x = match bbox.get("x").and_then(|v| v.as_f64()) {
                            Some(val) => val as f32,
                            None => continue,
                        };
                        let y = match bbox.get("y").and_then(|v| v.as_f64()) {
                            Some(val) => val as f32,
                            None => continue,
                        };
                        let width = match bbox.get("width").and_then(|v| v.as_f64()) {
                            Some(val) => val as f32,
                            None => continue,
                        };
                        let height = match bbox.get("height").and_then(|v| v.as_f64()) {
                            Some(val) => val as f32,
                            None => continue,
                        };

                        // Extract frame_idx if available, otherwise default to 0
                        let frame_idx =
                            d.get("frame_idx").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                        let detection = TrackingDetection {
                            class_id,
                            class_name,
                            confidence,
                            bbox: crate::BoundingBox::new(x, y, width, height),
                            frame_idx: frame_idx as u32,
                        };

                        // Ensure frames Vec is large enough
                        if frames.len() <= frame_idx {
                            frames.resize(frame_idx + 1, Vec::new());
                        }
                        frames[frame_idx].push(detection);
                    }
                }
                frames
            }
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected JSON object detection input".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Parsed detections: {} frames, total detections: {}",
                detections_per_frame.len(),
                detections_per_frame.iter().map(|f| f.len()).sum::<usize>()
            );
        }

        if detections_per_frame.is_empty() {
            return Err(PluginError::ExecutionFailed(
                "No detections found in input data".to_string(),
            ));
        }

        // Configure tracker
        let mut config = MotionTrackingConfig::default();
        if let Some(threshold) = high_confidence_threshold {
            config.high_confidence_threshold = threshold;
        }
        if let Some(threshold) = low_confidence_threshold {
            config.low_confidence_threshold = threshold;
        }
        if let Some(threshold) = detection_threshold_high {
            config.detection_threshold_high = threshold;
        }
        if let Some(threshold) = detection_threshold_low {
            config.detection_threshold_low = threshold;
        }
        if let Some(age) = max_age {
            config.max_age = age;
        }
        if let Some(hits) = min_hits {
            config.min_hits = hits;
        }

        let mut tracker = MotionTracker::new(config);

        // Process each frame
        for frame_detections in &detections_per_frame {
            tracker
                .update(frame_detections)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        }

        // Get final tracks
        let final_tracks = tracker.get_all_tracks();

        let elapsed = start.elapsed();

        // Build result JSON - pre-allocate vectors for JSON serialization
        let tracks_vec: Vec<Value> = {
            let mut tracks = Vec::with_capacity(final_tracks.len());
            tracks.extend(final_tracks.iter().map(|track| {
                let mut detections = Vec::with_capacity(track.detections.len());
                detections.extend(track.detections.iter().map(|det| {
                    json!({
                        "frame": det.frame_idx,
                        "confidence": det.confidence,
                        "bbox": {
                            "x": det.bbox.x,
                            "y": det.bbox.y,
                            "width": det.bbox.width,
                            "height": det.bbox.height,
                        }
                    })
                }));

                let trajectory = track.trajectory();
                let mut traj_vec = Vec::with_capacity(trajectory.len());
                traj_vec.extend(trajectory.iter().map(|(x, y)| json!({"x": x, "y": y})));

                json!({
                    "id": track.id,
                    "class_id": track.class_id,
                    "class_name": track.class_name,
                    "start_frame": track.start_frame,
                    "end_frame": track.end_frame,
                    "duration": track.duration(),
                    "hits": track.hits,
                    "age": track.age,
                    "detections": detections,
                    "trajectory": traj_vec,
                })
            }));
            tracks
        };

        let result = json!({
            "tracks": tracks_vec,
            "total_frames": detections_per_frame.len(),
            "total_tracks": final_tracks.len(),
        });

        info!(
            "Motion tracking complete: {} tracks, {} frames in {:.2}ms",
            final_tracks.len(),
            detections_per_frame.len(),
            elapsed.as_millis()
        );

        Ok(PluginResponse {
            output: PluginData::Json(result),
            duration: elapsed,
            warnings: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_extract_core::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};

    fn create_test_config() -> PluginConfig {
        PluginConfig {
            name: "motion-tracking".to_string(),
            description: "Track objects across video frames".to_string(),
            inputs: vec!["ObjectDetection".to_string()],
            outputs: vec!["MotionTracking".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 1000,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "1s".to_string(),
                memory_per_file_mb: 100,
                supports_streaming: false,
            },
            cache: CacheConfig {
                enabled: false,
                version: 1,
                invalidate_before: std::time::SystemTime::UNIX_EPOCH,
            },
        }
    }

    #[test]
    fn test_plugin_creation() {
        let config = create_test_config();
        let plugin = MotionTrackingPlugin::new(config);
        assert_eq!(plugin.name(), "motion-tracking");
    }

    #[test]
    fn test_plugin_supports_input() {
        let config = create_test_config();
        let plugin = MotionTrackingPlugin::new(config);

        assert!(plugin.supports_input("ObjectDetection"));
        assert!(!plugin.supports_input("Keyframes"));
    }

    #[test]
    fn test_plugin_produces_output() {
        let config = create_test_config();
        let plugin = MotionTrackingPlugin::new(config);

        assert!(plugin.produces_output("MotionTracking"));
        assert!(!plugin.produces_output("ObjectDetection"));
    }
}
