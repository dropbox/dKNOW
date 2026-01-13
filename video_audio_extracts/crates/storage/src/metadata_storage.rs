//! Metadata storage implementation using `PostgreSQL`
//!
//! This module provides an interface for storing structured metadata about media files,
//! including transcription segments, object detections, and timeline entries.

use crate::{
    DetectionResult, MediaMetadata, StorageError, StorageResult, TimelineEntry,
    TranscriptionSegment,
};
use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, NoTls};

/// `PostgreSQL` configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Database host
    pub host: String,

    /// Database port
    pub port: u16,

    /// Database name
    pub database: String,

    /// Database user
    pub user: String,

    /// Database password
    pub password: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("POSTGRES_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(5432),
            database: std::env::var("POSTGRES_DB")
                .unwrap_or_else(|_| "video_audio_extracts".to_string()),
            user: std::env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string()),
            password: std::env::var("POSTGRES_PASSWORD").unwrap_or_default(),
        }
    }
}

impl PostgresConfig {
    /// Build connection string
    #[must_use]
    pub fn connection_string(&self) -> String {
        format!(
            "host={} port={} dbname={} user={} password={}",
            self.host, self.port, self.database, self.user, self.password
        )
    }
}

/// Metadata storage trait
#[async_trait::async_trait]
pub trait MetadataStorage: Send + Sync {
    /// Initialize database schema (create tables if not exist)
    async fn init_schema(&self) -> StorageResult<()>;

    /// Store media metadata
    async fn store_media_metadata(&self, metadata: &MediaMetadata) -> StorageResult<String>;

    /// Retrieve media metadata by job ID
    async fn get_media_metadata(&self, job_id: &str) -> StorageResult<MediaMetadata>;

    /// Store transcription segments (batch)
    async fn store_transcription_segments(
        &self,
        segments: &[TranscriptionSegment],
    ) -> StorageResult<usize>;

    /// Retrieve transcription segments for a job
    async fn get_transcription_segments(
        &self,
        job_id: &str,
    ) -> StorageResult<Vec<TranscriptionSegment>>;

    /// Store object detection results (batch)
    async fn store_detection_results(&self, detections: &[DetectionResult])
        -> StorageResult<usize>;

    /// Retrieve object detection results for a job
    async fn get_detection_results(&self, job_id: &str) -> StorageResult<Vec<DetectionResult>>;

    /// Store timeline entries (batch)
    async fn store_timeline_entries(&self, entries: &[TimelineEntry]) -> StorageResult<usize>;

    /// Retrieve timeline entries for a job
    async fn get_timeline_entries(&self, job_id: &str) -> StorageResult<Vec<TimelineEntry>>;

    /// Delete all data for a job
    async fn delete_job_data(&self, job_id: &str) -> StorageResult<()>;

    /// Query timeline entries by time range
    async fn query_timeline_by_time(
        &self,
        job_id: &str,
        start_time: f64,
        end_time: f64,
    ) -> StorageResult<Vec<TimelineEntry>>;
}

/// `PostgreSQL` metadata storage implementation
pub struct PostgresMetadataStorage {
    client: Client,
}

impl PostgresMetadataStorage {
    /// Create a new `PostgreSQL` metadata storage client
    pub async fn new(config: PostgresConfig) -> StorageResult<Self> {
        let (client, connection) = tokio_postgres::connect(&config.connection_string(), NoTls)
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        // Spawn connection in background
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("PostgreSQL connection error: {}", e);
            }
        });

        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl MetadataStorage for PostgresMetadataStorage {
    async fn init_schema(&self) -> StorageResult<()> {
        // Create media_metadata table
        self.client
            .execute(
                r"
                CREATE TABLE IF NOT EXISTS media_metadata (
                    job_id TEXT PRIMARY KEY,
                    input_path TEXT NOT NULL,
                    format TEXT NOT NULL,
                    duration_secs DOUBLE PRECISION NOT NULL,
                    num_streams INTEGER NOT NULL,
                    resolution_width INTEGER,
                    resolution_height INTEGER,
                    frame_rate DOUBLE PRECISION,
                    sample_rate INTEGER,
                    audio_channels SMALLINT,
                    processed_at TIMESTAMP WITH TIME ZONE NOT NULL,
                    extra JSONB
                )
                ",
                &[],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        // Create transcription_segments table
        self.client
            .execute(
                r"
                CREATE TABLE IF NOT EXISTS transcription_segments (
                    id SERIAL PRIMARY KEY,
                    job_id TEXT NOT NULL,
                    segment_id INTEGER NOT NULL,
                    start_time DOUBLE PRECISION NOT NULL,
                    end_time DOUBLE PRECISION NOT NULL,
                    text TEXT NOT NULL,
                    confidence REAL NOT NULL,
                    speaker_id TEXT,
                    UNIQUE (job_id, segment_id)
                )
                ",
                &[],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        // Create detection_results table
        self.client
            .execute(
                r"
                CREATE TABLE IF NOT EXISTS detection_results (
                    id SERIAL PRIMARY KEY,
                    job_id TEXT NOT NULL,
                    frame_id TEXT NOT NULL,
                    class_id INTEGER NOT NULL,
                    class_name TEXT NOT NULL,
                    confidence REAL NOT NULL,
                    bbox_x REAL NOT NULL,
                    bbox_y REAL NOT NULL,
                    bbox_width REAL NOT NULL,
                    bbox_height REAL NOT NULL
                )
                ",
                &[],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        // Create timeline_entries table
        self.client
            .execute(
                r"
                CREATE TABLE IF NOT EXISTS timeline_entries (
                    id SERIAL PRIMARY KEY,
                    job_id TEXT NOT NULL,
                    entry_type TEXT NOT NULL,
                    start_time DOUBLE PRECISION NOT NULL,
                    end_time DOUBLE PRECISION NOT NULL,
                    data JSONB NOT NULL
                )
                ",
                &[],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        // Create indexes
        self.client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_transcription_job_id ON transcription_segments(job_id)",
                &[],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        self.client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_detection_job_id ON detection_results(job_id)",
                &[],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        self.client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_timeline_job_id ON timeline_entries(job_id)",
                &[],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        self.client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_timeline_time ON timeline_entries(job_id, start_time, end_time)",
                &[],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        tracing::info!("PostgreSQL schema initialized");

        Ok(())
    }

    async fn store_media_metadata(&self, metadata: &MediaMetadata) -> StorageResult<String> {
        let extra_json = serde_json::to_value(&metadata.extra)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        self.client
            .execute(
                r"
                INSERT INTO media_metadata
                (job_id, input_path, format, duration_secs, num_streams,
                 resolution_width, resolution_height, frame_rate, sample_rate,
                 audio_channels, processed_at, extra)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (job_id) DO UPDATE SET
                    input_path = EXCLUDED.input_path,
                    format = EXCLUDED.format,
                    duration_secs = EXCLUDED.duration_secs,
                    num_streams = EXCLUDED.num_streams,
                    resolution_width = EXCLUDED.resolution_width,
                    resolution_height = EXCLUDED.resolution_height,
                    frame_rate = EXCLUDED.frame_rate,
                    sample_rate = EXCLUDED.sample_rate,
                    audio_channels = EXCLUDED.audio_channels,
                    processed_at = EXCLUDED.processed_at,
                    extra = EXCLUDED.extra
                ",
                &[
                    &metadata.job_id,
                    &metadata.input_path,
                    &metadata.format,
                    &metadata.duration_secs,
                    &(metadata.num_streams as i32),
                    &metadata.resolution.map(|(w, _)| w as i32),
                    &metadata.resolution.map(|(_, h)| h as i32),
                    &metadata.frame_rate,
                    &metadata.sample_rate.map(|s| s as i32),
                    &metadata.audio_channels.map(|c| c as i16),
                    &metadata.processed_at,
                    &extra_json,
                ],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        Ok(metadata.job_id.clone())
    }

    async fn get_media_metadata(&self, job_id: &str) -> StorageResult<MediaMetadata> {
        let row = self
            .client
            .query_one(
                r"
                SELECT job_id, input_path, format, duration_secs, num_streams,
                       resolution_width, resolution_height, frame_rate, sample_rate,
                       audio_channels, processed_at, extra
                FROM media_metadata
                WHERE job_id = $1
                ",
                &[&job_id],
            )
            .await
            .map_err(|e| {
                if e.to_string().contains("no rows") {
                    StorageError::NotFound(job_id.to_string())
                } else {
                    StorageError::PostgresError(e.to_string())
                }
            })?;

        let resolution_width: Option<i32> = row.get(5);
        let resolution_height: Option<i32> = row.get(6);
        let resolution = match (resolution_width, resolution_height) {
            (Some(w), Some(h)) => Some((w as u32, h as u32)),
            _ => None,
        };

        let extra_json: serde_json::Value = row.get(11);
        let extra: std::collections::HashMap<String, String> =
            serde_json::from_value(extra_json)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        Ok(MediaMetadata {
            job_id: row.get(0),
            input_path: row.get(1),
            format: row.get(2),
            duration_secs: row.get(3),
            num_streams: row.get::<_, i32>(4) as usize,
            resolution,
            frame_rate: row.get(7),
            sample_rate: row.get::<_, Option<i32>>(8).map(|s| s as u32),
            audio_channels: row.get::<_, Option<i16>>(9).map(|c| c as u16),
            processed_at: row.get(10),
            extra,
        })
    }

    async fn store_transcription_segments(
        &self,
        segments: &[TranscriptionSegment],
    ) -> StorageResult<usize> {
        if segments.is_empty() {
            return Ok(0);
        }

        let mut count = 0;

        for segment in segments {
            self.client
                .execute(
                    r"
                    INSERT INTO transcription_segments
                    (job_id, segment_id, start_time, end_time, text, confidence, speaker_id)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (job_id, segment_id) DO UPDATE SET
                        start_time = EXCLUDED.start_time,
                        end_time = EXCLUDED.end_time,
                        text = EXCLUDED.text,
                        confidence = EXCLUDED.confidence,
                        speaker_id = EXCLUDED.speaker_id
                    ",
                    &[
                        &segment.job_id,
                        &(segment.segment_id as i32),
                        &segment.start_time,
                        &segment.end_time,
                        &segment.text,
                        &segment.confidence,
                        &segment.speaker_id,
                    ],
                )
                .await
                .map_err(|e| StorageError::PostgresError(e.to_string()))?;

            count += 1;
        }

        Ok(count)
    }

    async fn get_transcription_segments(
        &self,
        job_id: &str,
    ) -> StorageResult<Vec<TranscriptionSegment>> {
        let rows = self
            .client
            .query(
                r"
                SELECT job_id, segment_id, start_time, end_time, text, confidence, speaker_id
                FROM transcription_segments
                WHERE job_id = $1
                ORDER BY segment_id
                ",
                &[&job_id],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        let segments = rows
            .into_iter()
            .map(|row| TranscriptionSegment {
                job_id: row.get(0),
                segment_id: row.get::<_, i32>(1) as usize,
                start_time: row.get(2),
                end_time: row.get(3),
                text: row.get(4),
                confidence: row.get(5),
                speaker_id: row.get(6),
            })
            .collect();

        Ok(segments)
    }

    async fn store_detection_results(
        &self,
        detections: &[DetectionResult],
    ) -> StorageResult<usize> {
        if detections.is_empty() {
            return Ok(0);
        }

        let mut count = 0;

        for detection in detections {
            self.client
                .execute(
                    r"
                    INSERT INTO detection_results
                    (job_id, frame_id, class_id, class_name, confidence, bbox_x, bbox_y, bbox_width, bbox_height)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    ",
                    &[
                        &detection.job_id,
                        &detection.frame_id,
                        &(detection.class_id as i32),
                        &detection.class_name,
                        &detection.confidence,
                        &detection.bbox.0,
                        &detection.bbox.1,
                        &detection.bbox.2,
                        &detection.bbox.3,
                    ],
                )
                .await
                .map_err(|e| StorageError::PostgresError(e.to_string()))?;

            count += 1;
        }

        Ok(count)
    }

    async fn get_detection_results(&self, job_id: &str) -> StorageResult<Vec<DetectionResult>> {
        let rows = self
            .client
            .query(
                r"
                SELECT job_id, frame_id, class_id, class_name, confidence, bbox_x, bbox_y, bbox_width, bbox_height
                FROM detection_results
                WHERE job_id = $1
                ",
                &[&job_id],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        let detections = rows
            .into_iter()
            .map(|row| DetectionResult {
                job_id: row.get(0),
                frame_id: row.get(1),
                class_id: row.get::<_, i32>(2) as u32,
                class_name: row.get(3),
                confidence: row.get(4),
                bbox: (row.get(5), row.get(6), row.get(7), row.get(8)),
            })
            .collect();

        Ok(detections)
    }

    async fn store_timeline_entries(&self, entries: &[TimelineEntry]) -> StorageResult<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let mut count = 0;

        for entry in entries {
            self.client
                .execute(
                    r"
                    INSERT INTO timeline_entries
                    (job_id, entry_type, start_time, end_time, data)
                    VALUES ($1, $2, $3, $4, $5)
                    ",
                    &[
                        &entry.job_id,
                        &entry.entry_type,
                        &entry.start_time,
                        &entry.end_time,
                        &entry.data,
                    ],
                )
                .await
                .map_err(|e| StorageError::PostgresError(e.to_string()))?;

            count += 1;
        }

        Ok(count)
    }

    async fn get_timeline_entries(&self, job_id: &str) -> StorageResult<Vec<TimelineEntry>> {
        let rows = self
            .client
            .query(
                r"
                SELECT job_id, entry_type, start_time, end_time, data
                FROM timeline_entries
                WHERE job_id = $1
                ORDER BY start_time
                ",
                &[&job_id],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        let entries = rows
            .into_iter()
            .map(|row| TimelineEntry {
                job_id: row.get(0),
                entry_type: row.get(1),
                start_time: row.get(2),
                end_time: row.get(3),
                data: row.get(4),
            })
            .collect();

        Ok(entries)
    }

    async fn delete_job_data(&self, job_id: &str) -> StorageResult<()> {
        self.client
            .execute("DELETE FROM media_metadata WHERE job_id = $1", &[&job_id])
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        self.client
            .execute(
                "DELETE FROM transcription_segments WHERE job_id = $1",
                &[&job_id],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        self.client
            .execute(
                "DELETE FROM detection_results WHERE job_id = $1",
                &[&job_id],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        self.client
            .execute("DELETE FROM timeline_entries WHERE job_id = $1", &[&job_id])
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        Ok(())
    }

    async fn query_timeline_by_time(
        &self,
        job_id: &str,
        start_time: f64,
        end_time: f64,
    ) -> StorageResult<Vec<TimelineEntry>> {
        let rows = self
            .client
            .query(
                r"
                SELECT job_id, entry_type, start_time, end_time, data
                FROM timeline_entries
                WHERE job_id = $1 AND start_time >= $2 AND end_time <= $3
                ORDER BY start_time
                ",
                &[&job_id, &start_time, &end_time],
            )
            .await
            .map_err(|e| StorageError::PostgresError(e.to_string()))?;

        let entries = rows
            .into_iter()
            .map(|row| TimelineEntry {
                job_id: row.get(0),
                entry_type: row.get(1),
                start_time: row.get(2),
                end_time: row.get(3),
                data: row.get(4),
            })
            .collect();

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_config_default() {
        let config = PostgresConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.database, "video_audio_extracts");
        assert_eq!(config.user, "postgres");
    }

    #[test]
    fn test_postgres_connection_string() {
        let config = PostgresConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            user: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let conn_str = config.connection_string();
        assert!(conn_str.contains("host=localhost"));
        assert!(conn_str.contains("port=5432"));
        assert!(conn_str.contains("dbname=testdb"));
        assert!(conn_str.contains("user=testuser"));
        assert!(conn_str.contains("password=testpass"));
    }
}
