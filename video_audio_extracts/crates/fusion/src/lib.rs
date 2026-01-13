//! Fusion Layer - Cross-Modal Alignment and Timeline Generation
//!
//! Combines results from all processing modules (transcription, diarization, detection, OCR, embeddings)
//! into a unified timeline structure with temporal alignment and confidence scoring.
//!
//! ## Architecture
//!
//! The fusion layer performs:
//! 1. **Temporal Alignment**: Match events across modalities by timestamp overlap
//! 2. **Unified Timeline**: Generate chronological sequence of all events
//! 3. **Confidence Scoring**: Compute quality scores for cross-modal links
//! 4. **Entity Extraction**: Identify persistent entities (speakers, objects, locations)
//!
//! ## Example
//!
//! ```rust
//! use video_audio_fusion::{fuse_results, FusionConfig, FusionInput};
//!
//! let config = FusionConfig::default();
//! let input = FusionInput {
//!     duration: 120.0,
//!     transcript: Some("Hello world".to_string()),
//!     diarization: None,
//!     scenes: vec![],
//!     objects_per_frame: vec![],
//!     faces_per_frame: vec![],
//!     text_regions_per_frame: vec![],
//!     vision_embeddings: vec![],
//!     text_embeddings: vec![],
//!     audio_embeddings: vec![],
//! };
//!
//! let timeline = fuse_results(&config, input).unwrap();
//! assert_eq!(timeline.duration, 120.0);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use video_audio_diarization::Diarization;
use video_audio_face_detection::Face;
use video_audio_ocr::TextRegion;
use video_audio_scene::Scene;

/// Fusion errors
#[derive(Error, Debug)]
pub enum FusionError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Temporal alignment failed: {0}")]
    AlignmentError(String),
    #[error("Confidence scoring failed: {0}")]
    ScoringError(String),
}

/// Configuration for fusion layer
#[derive(Debug, Clone)]
pub struct FusionConfig {
    /// Minimum overlap (seconds) to consider events temporally aligned
    pub temporal_threshold: f64,
    /// Minimum confidence score to include in timeline (0.0-1.0)
    pub confidence_threshold: f32,
    /// Enable cross-modal entity linking
    pub enable_entity_linking: bool,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            temporal_threshold: 0.5,   // 500ms overlap required
            confidence_threshold: 0.3, // Include events with >30% confidence
            enable_entity_linking: true,
        }
    }
}

/// Input data for fusion layer
#[derive(Debug, Clone)]
pub struct FusionInput {
    /// Media duration in seconds
    pub duration: f64,
    /// Transcript text (optional)
    pub transcript: Option<String>,
    /// Speaker diarization results (optional)
    pub diarization: Option<Diarization>,
    /// Scene boundaries
    pub scenes: Vec<Scene>,
    /// Object detections per keyframe (indexed by frame)
    pub objects_per_frame: Vec<usize>, // Simplified: just counts per frame
    /// Face detections per keyframe
    pub faces_per_frame: Vec<Vec<Face>>,
    /// Text regions per keyframe
    pub text_regions_per_frame: Vec<Vec<TextRegion>>,
    /// Vision embeddings per keyframe
    pub vision_embeddings: Vec<Vec<f32>>,
    /// Text embeddings per transcript segment
    pub text_embeddings: Vec<Vec<f32>>,
    /// Audio embeddings per audio clip
    pub audio_embeddings: Vec<Vec<f32>>,
}

/// Unified timeline combining all processing results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    /// Media duration in seconds
    pub duration: f64,
    /// All events in chronological order
    pub events: Vec<Event>,
    /// Persistent entities identified across timeline
    pub entities: Vec<Entity>,
    /// Cross-modal relationships between events
    pub relationships: Vec<Relationship>,
    /// Quality scores for the fusion
    pub quality_scores: QualityScores,
}

/// A single event in the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier
    pub id: String,
    /// Type of event
    pub event_type: EventType,
    /// Start time in seconds
    pub start_time: f64,
    /// End time in seconds
    pub end_time: f64,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Event-specific data (JSON)
    pub data: serde_json::Value,
}

/// Types of events in the timeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Transcript segment (speech)
    TranscriptSegment,
    /// Speaker change boundary
    SpeakerChange,
    /// Scene boundary (visual transition)
    SceneBoundary,
    /// Object detection in frame
    ObjectDetection,
    /// Face detection in frame
    FaceDetection,
    /// Text detection in frame (OCR)
    TextDetection,
    /// Audio event (not yet implemented)
    AudioEvent,
}

/// A persistent entity identified across the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique entity identifier
    pub id: String,
    /// Type of entity
    pub entity_type: EntityType,
    /// First appearance time
    pub first_seen: f64,
    /// Last appearance time
    pub last_seen: f64,
    /// Confidence that this entity is consistent (0.0-1.0)
    pub confidence: f32,
    /// Entity-specific attributes
    pub attributes: HashMap<String, String>,
}

/// Types of entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// Speaker identified by voice
    Speaker,
    /// Person identified by face
    Person,
    /// Object identified by detector
    Object,
    /// Location identified by context
    Location,
}

/// A relationship between two events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Source event ID
    pub from_event: String,
    /// Target event ID
    pub to_event: String,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Confidence in this relationship (0.0-1.0)
    pub confidence: f32,
}

/// Types of relationships between events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// Events occur simultaneously
    Simultaneous,
    /// Target event follows source event
    Sequential,
    /// Events refer to same entity
    Coreferent,
    /// Speech describes visual content
    DescribesVisual,
    /// Visual content illustrates speech
    IllustratesSpeech,
}

/// Quality scores for the fusion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScores {
    /// Overall fusion quality (0.0-1.0)
    pub overall: f32,
    /// Temporal alignment quality (0.0-1.0)
    pub temporal_alignment: f32,
    /// Cross-modal consistency (0.0-1.0)
    pub cross_modal_consistency: f32,
    /// Number of events successfully fused
    pub events_fused: usize,
    /// Number of relationships identified
    pub relationships_found: usize,
}

/// Fuse all processing results into a unified timeline
///
/// # Arguments
///
/// * `config` - Fusion configuration
/// * `input` - All processing results to fuse
///
/// # Returns
///
/// Unified timeline with events, entities, and relationships
///
/// # Errors
///
/// Returns error if temporal alignment or confidence scoring fails
pub fn fuse_results(config: &FusionConfig, input: FusionInput) -> Result<Timeline, FusionError> {
    tracing::info!(
        "Starting fusion: duration={:.2}s, scenes={}, faces={}, text_regions={}",
        input.duration,
        input.scenes.len(),
        input.faces_per_frame.len(),
        input.text_regions_per_frame.len()
    );

    // Initialize timeline
    // Estimate capacity: transcript(1) + scenes + estimated faces (2 per frame) + estimated text regions (2 per frame)
    // + diarization segments (if present)
    let estimated_capacity = 1
        + input.scenes.len()
        + input.faces_per_frame.len() * 2
        + input.text_regions_per_frame.len() * 2
        + input
            .diarization
            .as_ref()
            .map(|d| d.timeline.len())
            .unwrap_or(0);
    let mut events = Vec::with_capacity(estimated_capacity);
    let mut event_id_counter = 0;

    // Step 1: Convert transcript to events
    if let Some(transcript) = &input.transcript {
        let event = Event {
            id: format!("transcript_{event_id_counter}"),
            event_type: EventType::TranscriptSegment,
            start_time: 0.0,
            end_time: input.duration,
            confidence: 0.9, // High confidence for full transcript
            data: serde_json::json!({ "text": transcript }),
        };
        events.push(event);
        event_id_counter += 1;
    }

    // Step 2: Convert diarization to events
    if let Some(diarization) = &input.diarization {
        for segment in &diarization.timeline {
            let event = Event {
                id: format!("speaker_{event_id_counter}"),
                event_type: EventType::SpeakerChange,
                start_time: segment.start,
                end_time: segment.end,
                confidence: segment.confidence,
                data: serde_json::json!({
                    "speaker": segment.speaker
                }),
            };
            events.push(event);
            event_id_counter += 1;
        }
    }

    // Step 3: Convert scene boundaries to events
    for (idx, scene) in input.scenes.iter().enumerate() {
        let event = Event {
            id: format!("scene_{event_id_counter}"),
            event_type: EventType::SceneBoundary,
            start_time: scene.start_time,
            end_time: scene.end_time,
            confidence: scene.score,
            data: serde_json::json!({
                "scene_index": idx,
                "frame_count": scene.frame_count
            }),
        };
        events.push(event);
        event_id_counter += 1;
    }

    // Step 4: Convert face detections to events
    // Assume keyframes are evenly distributed across duration
    let num_keyframes = input.faces_per_frame.len();
    for (frame_idx, faces) in input.faces_per_frame.iter().enumerate() {
        let timestamp = (frame_idx as f64 / num_keyframes.max(1) as f64) * input.duration;
        for (face_idx, face) in faces.iter().enumerate() {
            let event = Event {
                id: format!("face_{event_id_counter}"),
                event_type: EventType::FaceDetection,
                start_time: timestamp,
                end_time: timestamp + 0.1, // Instantaneous event
                confidence: face.confidence,
                data: serde_json::json!({
                    "frame": frame_idx,
                    "face_index": face_idx,
                    "bbox": face.bbox
                }),
            };
            events.push(event);
            event_id_counter += 1;
        }
    }

    // Step 5: Convert text regions to events
    let num_text_frames = input.text_regions_per_frame.len();
    for (frame_idx, text_regions) in input.text_regions_per_frame.iter().enumerate() {
        let timestamp = (frame_idx as f64 / num_text_frames.max(1) as f64) * input.duration;
        for (region_idx, region) in text_regions.iter().enumerate() {
            let event = Event {
                id: format!("text_{event_id_counter}"),
                event_type: EventType::TextDetection,
                start_time: timestamp,
                end_time: timestamp + 0.1, // Instantaneous event
                confidence: region.confidence,
                data: serde_json::json!({
                    "frame": frame_idx,
                    "region_index": region_idx,
                    "text": region.text,
                    "bbox": region.bbox
                }),
            };
            events.push(event);
            event_id_counter += 1;
        }
    }

    // Step 6: Sort events by start time
    events.sort_by(|a, b| {
        a.start_time
            .partial_cmp(&b.start_time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Step 7: Build relationships (temporal overlap)
    let relationships = if config.enable_entity_linking {
        build_relationships(&events, config.temporal_threshold)
    } else {
        Vec::new()
    };

    // Step 8: Extract persistent entities
    let entities = extract_entities(&events, &input.diarization);

    // Step 9: Compute quality scores
    let quality_scores = compute_quality_scores(&events, &relationships);

    tracing::info!(
        "Fusion complete: {} events, {} entities, {} relationships",
        events.len(),
        entities.len(),
        relationships.len()
    );

    Ok(Timeline {
        duration: input.duration,
        events,
        entities,
        relationships,
        quality_scores,
    })
}

/// Build relationships between temporally overlapping events
fn build_relationships(events: &[Event], temporal_threshold: f64) -> Vec<Relationship> {
    // Pre-allocate with conservative estimate (roughly half of events create relationships)
    let mut relationships = Vec::with_capacity(events.len() / 2);

    for i in 0..events.len() {
        for j in (i + 1)..events.len() {
            let event_a = &events[i];
            let event_b = &events[j];

            // Skip if events are too far apart in time
            if event_b.start_time - event_a.end_time > temporal_threshold {
                break; // Events are sorted, so we can stop here
            }

            // Check for temporal overlap
            let overlap_start = event_a.start_time.max(event_b.start_time);
            let overlap_end = event_a.end_time.min(event_b.end_time);
            let overlap = (overlap_end - overlap_start).max(0.0);

            if overlap >= temporal_threshold {
                // Determine relationship type
                let relationship_type = match (&event_a.event_type, &event_b.event_type) {
                    (EventType::TranscriptSegment, EventType::FaceDetection) => {
                        RelationshipType::DescribesVisual
                    }
                    (EventType::FaceDetection, EventType::TranscriptSegment) => {
                        RelationshipType::IllustratesSpeech
                    }
                    (EventType::SpeakerChange, EventType::FaceDetection) => {
                        RelationshipType::Coreferent
                    }
                    _ => RelationshipType::Simultaneous,
                };

                let confidence = (overlap / temporal_threshold.max(1.0)).min(1.0) as f32;
                relationships.push(Relationship {
                    from_event: event_a.id.clone(),
                    to_event: event_b.id.clone(),
                    relationship_type,
                    confidence,
                });
            }
        }
    }

    relationships
}

/// Extract persistent entities from events
fn extract_entities(events: &[Event], diarization: &Option<Diarization>) -> Vec<Entity> {
    // Estimate capacity: unique speakers from diarization + 1 person entity (if faces exist)
    let estimated_capacity = diarization.as_ref().map(|d| d.timeline.len()).unwrap_or(0) + 1;
    let mut entities = Vec::with_capacity(estimated_capacity);

    // Extract speakers from diarization
    if let Some(diarization) = diarization {
        let mut speaker_appearances: HashMap<String, (f64, f64)> =
            HashMap::with_capacity(diarization.timeline.len());

        for segment in &diarization.timeline {
            let entry = speaker_appearances
                .entry(segment.speaker.clone())
                .or_insert((segment.start, segment.end));
            entry.0 = entry.0.min(segment.start); // Update first_seen
            entry.1 = entry.1.max(segment.end); // Update last_seen
        }

        for (idx, (speaker, (first_seen, last_seen))) in speaker_appearances.iter().enumerate() {
            entities.push(Entity {
                id: format!("entity_speaker_{idx}"),
                entity_type: EntityType::Speaker,
                first_seen: *first_seen,
                last_seen: *last_seen,
                confidence: 0.85,
                attributes: {
                    let mut attrs = HashMap::with_capacity(1);
                    attrs.insert("speaker_label".to_string(), speaker.clone());
                    attrs
                },
            });
        }
    }

    // Extract persons from face detections
    let face_events: Vec<&Event> = events
        .iter()
        .filter(|e| e.event_type == EventType::FaceDetection)
        .collect();

    if !face_events.is_empty() {
        // Simplified: Create single "person" entity spanning face detections
        let first_seen = face_events.first().map_or(0.0, |e| e.start_time);
        let last_seen = face_events.last().map_or(0.0, |e| e.end_time);

        entities.push(Entity {
            id: "entity_person_0".to_string(),
            entity_type: EntityType::Person,
            first_seen,
            last_seen,
            confidence: 0.75,
            attributes: {
                let mut attrs = HashMap::with_capacity(1);
                attrs.insert("face_count".to_string(), face_events.len().to_string());
                attrs
            },
        });
    }

    entities
}

/// Compute quality scores for fusion result
fn compute_quality_scores(events: &[Event], relationships: &[Relationship]) -> QualityScores {
    let events_fused = events.len();
    let relationships_found = relationships.len();

    // Compute average event confidence
    let avg_confidence = if events.is_empty() {
        0.0
    } else {
        events.iter().map(|e| e.confidence).sum::<f32>() / events.len() as f32
    };

    // Compute temporal alignment quality (based on event density)
    let temporal_alignment = if events.is_empty() {
        0.0
    } else {
        // Simple heuristic: more events = better coverage
        (events.len() as f32 / 100.0).min(1.0)
    };

    // Compute cross-modal consistency (based on relationship density)
    let cross_modal_consistency = if events.len() < 2 {
        0.0
    } else {
        let max_relationships = events.len() * (events.len() - 1) / 2;
        (relationships.len() as f32 / max_relationships as f32).min(1.0)
    };

    QualityScores {
        overall: (avg_confidence + temporal_alignment + cross_modal_consistency) / 3.0,
        temporal_alignment,
        cross_modal_consistency,
        events_fused,
        relationships_found,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuse_empty_input() {
        let config = FusionConfig::default();
        let input = FusionInput {
            duration: 60.0,
            transcript: None,
            diarization: None,
            scenes: vec![],
            objects_per_frame: vec![],
            faces_per_frame: vec![],
            text_regions_per_frame: vec![],
            vision_embeddings: vec![],
            text_embeddings: vec![],
            audio_embeddings: vec![],
        };

        let timeline = fuse_results(&config, input).unwrap();
        assert_eq!(timeline.duration, 60.0);
        assert_eq!(timeline.events.len(), 0);
        assert_eq!(timeline.entities.len(), 0);
        assert_eq!(timeline.relationships.len(), 0);
    }

    #[test]
    fn test_fuse_with_transcript() {
        let config = FusionConfig::default();
        let input = FusionInput {
            duration: 60.0,
            transcript: Some("Hello world".to_string()),
            diarization: None,
            scenes: vec![],
            objects_per_frame: vec![],
            faces_per_frame: vec![],
            text_regions_per_frame: vec![],
            vision_embeddings: vec![],
            text_embeddings: vec![],
            audio_embeddings: vec![],
        };

        let timeline = fuse_results(&config, input).unwrap();
        assert_eq!(timeline.duration, 60.0);
        assert_eq!(timeline.events.len(), 1);
        assert_eq!(timeline.events[0].event_type, EventType::TranscriptSegment);
    }

    #[test]
    fn test_fuse_with_scenes() {
        let config = FusionConfig::default();
        let input = FusionInput {
            duration: 60.0,
            transcript: None,
            diarization: None,
            scenes: vec![
                Scene {
                    start_time: 0.0,
                    end_time: 10.0,
                    start_frame: 0,
                    end_frame: 300,
                    frame_count: 300,
                    score: 0.95,
                },
                Scene {
                    start_time: 10.0,
                    end_time: 20.0,
                    start_frame: 300,
                    end_frame: 600,
                    frame_count: 300,
                    score: 0.85,
                },
            ],
            objects_per_frame: vec![],
            faces_per_frame: vec![],
            text_regions_per_frame: vec![],
            vision_embeddings: vec![],
            text_embeddings: vec![],
            audio_embeddings: vec![],
        };

        let timeline = fuse_results(&config, input).unwrap();
        assert_eq!(timeline.duration, 60.0);
        assert_eq!(timeline.events.len(), 2);
        assert_eq!(timeline.events[0].event_type, EventType::SceneBoundary);
        assert_eq!(timeline.events[1].event_type, EventType::SceneBoundary);
    }
}
