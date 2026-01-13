//! Object storage implementation using S3/MinIO
//!
//! This module provides an interface for storing and retrieving large binary files
//! such as raw media files, extracted audio, keyframes, and thumbnails.

use crate::{StorageError, StorageResult};
use aws_sdk_s3::{
    config::{Credentials, Region},
    primitives::ByteStream,
    Client,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// S3/MinIO configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,

    /// AWS region (e.g., "us-west-2") or "us-east-1" for `MinIO`
    pub region: String,

    /// S3 endpoint (custom for `MinIO`, empty for AWS S3)
    pub endpoint: Option<String>,

    /// AWS access key ID
    pub access_key_id: String,

    /// AWS secret access key
    pub secret_access_key: String,

    /// Path prefix for all objects (e.g., "video-extracts/")
    pub prefix: String,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            bucket: "video-audio-extracts".to_string(),
            region: "us-west-2".to_string(),
            endpoint: None,
            access_key_id: std::env::var("AWS_ACCESS_KEY_ID").unwrap_or_default(),
            secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").unwrap_or_default(),
            prefix: String::new(),
        }
    }
}

/// Object storage trait
#[async_trait::async_trait]
pub trait ObjectStorage: Send + Sync {
    /// Store a file from bytes
    async fn store_file(&self, key: &str, data: &[u8]) -> StorageResult<String>;

    /// Store a file from local path
    async fn store_file_from_path(&self, key: &str, path: &Path) -> StorageResult<String>;

    /// Retrieve a file as bytes
    async fn retrieve_file(&self, key: &str) -> StorageResult<Vec<u8>>;

    /// Retrieve a file and save to local path
    async fn retrieve_file_to_path(&self, key: &str, path: &Path) -> StorageResult<()>;

    /// List files with prefix
    async fn list_files(&self, prefix: &str) -> StorageResult<Vec<String>>;

    /// Delete a file
    async fn delete_file(&self, key: &str) -> StorageResult<()>;

    /// Check if a file exists
    async fn file_exists(&self, key: &str) -> StorageResult<bool>;

    /// Get file size in bytes
    async fn get_file_size(&self, key: &str) -> StorageResult<u64>;
}

/// S3/MinIO object storage implementation
pub struct S3ObjectStorage {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3ObjectStorage {
    /// Create a new S3 object storage client
    pub async fn new(config: S3Config) -> StorageResult<Self> {
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,
            None,
            "video-audio-storage",
        );

        let region = Region::new(config.region.clone());

        let mut s3_config_builder = aws_sdk_s3::Config::builder()
            .credentials_provider(credentials)
            .region(region)
            .behavior_version_latest();

        // Set custom endpoint for MinIO
        if let Some(endpoint) = config.endpoint {
            s3_config_builder = s3_config_builder
                .endpoint_url(endpoint)
                .force_path_style(true); // Required for MinIO
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket: config.bucket,
            prefix: config.prefix,
        })
    }

    /// Combine prefix with key
    fn full_key(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}{}", self.prefix, key)
        }
    }
}

#[async_trait::async_trait]
impl ObjectStorage for S3ObjectStorage {
    async fn store_file(&self, key: &str, data: &[u8]) -> StorageResult<String> {
        let full_key = self.full_key(key);
        let byte_stream = ByteStream::from(data.to_vec());

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .body(byte_stream)
            .send()
            .await
            .map_err(|e| StorageError::S3Error(e.to_string()))?;

        Ok(full_key)
    }

    async fn store_file_from_path(&self, key: &str, path: &Path) -> StorageResult<String> {
        let data = tokio::fs::read(path).await?;
        self.store_file(key, &data).await
    }

    async fn retrieve_file(&self, key: &str) -> StorageResult<Vec<u8>> {
        let full_key = self.full_key(key);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NoSuchKey") {
                    StorageError::NotFound(full_key.clone())
                } else {
                    StorageError::S3Error(e.to_string())
                }
            })?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::S3Error(e.to_string()))?;

        Ok(bytes.to_vec())
    }

    async fn retrieve_file_to_path(&self, key: &str, path: &Path) -> StorageResult<()> {
        let data = self.retrieve_file(key).await?;
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    async fn list_files(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let full_prefix = self.full_key(prefix);

        let response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&full_prefix)
            .send()
            .await
            .map_err(|e| StorageError::S3Error(e.to_string()))?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(std::string::ToString::to_string))
            .collect();

        Ok(keys)
    }

    async fn delete_file(&self, key: &str) -> StorageResult<()> {
        let full_key = self.full_key(key);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
            .map_err(|e| StorageError::S3Error(e.to_string()))?;

        Ok(())
    }

    async fn file_exists(&self, key: &str) -> StorageResult<bool> {
        let full_key = self.full_key(key);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(StorageError::S3Error(e.to_string()))
                }
            }
        }
    }

    async fn get_file_size(&self, key: &str) -> StorageResult<u64> {
        let full_key = self.full_key(key);

        let response = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NotFound") {
                    StorageError::NotFound(full_key.clone())
                } else {
                    StorageError::S3Error(e.to_string())
                }
            })?;

        Ok(response.content_length().unwrap_or(0) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_config_default() {
        let config = S3Config::default();
        assert_eq!(config.bucket, "video-audio-extracts");
        assert_eq!(config.region, "us-west-2");
        assert_eq!(config.endpoint, None);
    }

    #[test]
    fn test_s3_config_with_minio() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
            access_key_id: "minioadmin".to_string(),
            secret_access_key: "minioadmin".to_string(),
            prefix: "test/".to_string(),
        };

        assert_eq!(config.endpoint, Some("http://localhost:9000".to_string()));
        assert_eq!(config.prefix, "test/");
    }

    #[test]
    fn test_full_key_with_prefix() {
        let config = S3Config {
            prefix: "video-extracts/".to_string(),
            ..Default::default()
        };

        let storage = S3ObjectStorage {
            client: Client::from_conf(
                aws_sdk_s3::Config::builder()
                    .behavior_version_latest()
                    .build(),
            ),
            bucket: config.bucket,
            prefix: config.prefix,
        };

        assert_eq!(storage.full_key("test.txt"), "video-extracts/test.txt");
    }

    #[test]
    fn test_full_key_without_prefix() {
        let config = S3Config::default();

        let storage = S3ObjectStorage {
            client: Client::from_conf(
                aws_sdk_s3::Config::builder()
                    .behavior_version_latest()
                    .build(),
            ),
            bucket: config.bucket,
            prefix: config.prefix,
        };

        assert_eq!(storage.full_key("test.txt"), "test.txt");
    }
}
