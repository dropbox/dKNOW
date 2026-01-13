//! Media download module for URL and S3 sources
//!
//! This module provides functionality to download media files from:
//! - HTTP/HTTPS URLs
//! - S3 buckets (AWS S3, `MinIO`, etc.)
//!
//! Downloaded files are stored in temporary locations and automatically cleaned up
//! when the `DownloadedFile` is dropped.

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use reqwest::Client as HttpClient;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

/// A downloaded file that will be automatically cleaned up when dropped
pub struct DownloadedFile {
    /// Path to the downloaded file
    path: PathBuf,
    /// Temporary file handle (keeps file alive until dropped)
    _temp_file: Option<NamedTempFile>,
}

impl DownloadedFile {
    /// Create a new `DownloadedFile` from a temporary file
    fn from_temp_file(temp_file: NamedTempFile) -> Self {
        let path = temp_file.path().to_path_buf();
        Self {
            path,
            _temp_file: Some(temp_file),
        }
    }

    /// Get the path to the downloaded file
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for DownloadedFile {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

/// Download a file from an HTTP/HTTPS URL
///
/// # Arguments
/// * `url` - The URL to download from
///
/// # Returns
/// A `DownloadedFile` that will be automatically cleaned up when dropped
///
/// # Errors
/// Returns an error if:
/// - The URL is invalid
/// - The HTTP request fails
/// - The response cannot be written to disk
pub async fn download_from_url(url: &str) -> Result<DownloadedFile> {
    info!("Downloading file from URL: {}", url);

    // Validate URL scheme
    if !url.starts_with("http://") && !url.starts_with("https://") {
        anyhow::bail!("Invalid URL scheme. Only http:// and https:// are supported");
    }

    // Create HTTP client with reasonable timeouts
    let client = HttpClient::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
        .build()
        .context("Failed to create HTTP client")?;

    // Start download
    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to send HTTP request")?;

    // Check response status
    if !response.status().is_success() {
        anyhow::bail!("HTTP request failed with status: {}", response.status());
    }

    // Get content length for progress tracking
    let content_length = response.content_length();
    if let Some(size) = content_length {
        debug!("Download size: {} bytes", size);
    }

    // Infer file extension from URL or content-type
    let extension = infer_extension_from_url(url)
        .or_else(|| {
            response
                .headers()
                .get("content-type")
                .and_then(|ct| ct.to_str().ok())
                .and_then(infer_extension_from_content_type)
        })
        .unwrap_or("tmp");

    // Create temporary file with appropriate extension
    let temp_file = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .context("Failed to create temporary file")?;

    let temp_path = temp_file.path().to_path_buf();
    debug!("Writing to temporary file: {}", temp_path.display());

    // Open file for writing
    let mut file = File::create(&temp_path)
        .await
        .context("Failed to open temporary file for writing")?;

    // Stream response body to file
    let bytes = response
        .bytes()
        .await
        .context("Failed to read response body")?;

    file.write_all(&bytes)
        .await
        .context("Failed to write response to file")?;

    file.flush().await.context("Failed to flush file")?;

    info!(
        "Successfully downloaded {} bytes to {}",
        bytes.len(),
        temp_path.display()
    );

    Ok(DownloadedFile::from_temp_file(temp_file))
}

/// Download a file from an S3 bucket
///
/// # Arguments
/// * `s3_location` - The S3 location in format: <s3://bucket-name/path/to/object>
///
/// # Returns
/// A `DownloadedFile` that will be automatically cleaned up when dropped
///
/// # Errors
/// Returns an error if:
/// - The S3 location format is invalid
/// - The S3 client cannot be initialized
/// - The object cannot be downloaded
pub async fn download_from_s3(s3_location: &str) -> Result<DownloadedFile> {
    info!("Downloading file from S3: {}", s3_location);

    // Parse S3 location: s3://bucket-name/path/to/object
    let s3_location = s3_location.trim();
    if !s3_location.starts_with("s3://") {
        anyhow::bail!("Invalid S3 location format. Expected: s3://bucket-name/path/to/object");
    }

    let location_without_scheme = &s3_location[5..]; // Remove "s3://"
    let parts: Vec<&str> = location_without_scheme.splitn(2, '/').collect();

    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        anyhow::bail!(
            "Invalid S3 location format. Expected: s3://bucket-name/path/to/object, got: {s3_location}"
        );
    }

    let bucket = parts[0];
    let key = parts[1];

    debug!("S3 bucket: {}, key: {}", bucket, key);

    // Initialize S3 client
    let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let s3_client = S3Client::new(&config);

    // Download object
    let response = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .context(format!("Failed to download S3 object: {s3_location}"))?;

    // Get content length and content type
    let content_length = response.content_length();
    let content_type = response
        .content_type()
        .map(std::string::ToString::to_string);

    if let Some(size) = content_length {
        debug!("S3 object size: {} bytes", size);
    }

    // Infer file extension from key or content-type
    let extension = infer_extension_from_url(key)
        .or_else(|| {
            content_type
                .as_ref()
                .and_then(|ct| infer_extension_from_content_type(ct))
        })
        .unwrap_or("tmp");

    // Create temporary file with appropriate extension
    let temp_file = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .context("Failed to create temporary file")?;

    let temp_path = temp_file.path().to_path_buf();
    debug!("Writing to temporary file: {}", temp_path.display());

    // Open file for writing
    let mut file = File::create(&temp_path)
        .await
        .context("Failed to open temporary file for writing")?;

    // Read body and write to file
    let body = response
        .body
        .collect()
        .await
        .context("Failed to read S3 object body")?;
    let bytes = body.into_bytes();

    file.write_all(&bytes)
        .await
        .context("Failed to write S3 object to file")?;

    file.flush().await.context("Failed to flush file")?;

    info!(
        "Successfully downloaded {} bytes from S3 to {}",
        bytes.len(),
        temp_path.display()
    );

    Ok(DownloadedFile::from_temp_file(temp_file))
}

/// Infer file extension from URL path
fn infer_extension_from_url(url: &str) -> Option<&str> {
    let path = url.split('?').next()?; // Remove query parameters
    let filename = path.split('/').next_back()?;

    // Check if filename contains a dot (i.e., has an extension)
    if !filename.contains('.') {
        return None;
    }

    let extension = filename.split('.').next_back()?;

    // Validate extension (only alphanumeric, max 5 chars)
    if extension.len() <= 5 && extension.chars().all(char::is_alphanumeric) {
        Some(extension)
    } else {
        None
    }
}

/// Infer file extension from content-type header
fn infer_extension_from_content_type(content_type: &str) -> Option<&str> {
    // Remove any parameters (e.g., "video/mp4; charset=utf-8" -> "video/mp4")
    let mime_type = content_type.split(';').next()?.trim();

    match mime_type {
        // Video formats
        "video/mp4" => Some("mp4"),
        "video/mpeg" => Some("mpeg"),
        "video/quicktime" => Some("mov"),
        "video/x-msvideo" => Some("avi"),
        "video/x-matroska" => Some("mkv"),
        "video/webm" => Some("webm"),
        "video/x-flv" => Some("flv"),
        "video/3gpp" => Some("3gp"),
        "video/3gpp2" => Some("3g2"),

        // Audio formats
        "audio/mpeg" => Some("mp3"),
        "audio/wav" => Some("wav"),
        "audio/wave" => Some("wav"),
        "audio/x-wav" => Some("wav"),
        "audio/ogg" => Some("ogg"),
        "audio/flac" => Some("flac"),
        "audio/aac" => Some("aac"),
        "audio/mp4" => Some("m4a"),
        "audio/webm" => Some("webm"),

        // Image formats
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/bmp" => Some("bmp"),
        "image/tiff" => Some("tiff"),

        _ => {
            warn!("Unknown content-type: {}", mime_type);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_extension_from_url() {
        assert_eq!(
            infer_extension_from_url("https://example.com/video.mp4"),
            Some("mp4")
        );
        assert_eq!(
            infer_extension_from_url("https://example.com/video.mp4?token=abc"),
            Some("mp4")
        );
        assert_eq!(
            infer_extension_from_url("https://example.com/path/to/audio.wav"),
            Some("wav")
        );
        assert_eq!(infer_extension_from_url("https://example.com/file"), None);
        assert_eq!(infer_extension_from_url("https://example.com/"), None);
        assert_eq!(
            infer_extension_from_url("https://example.com/file.toolongext"),
            None
        );
    }

    #[test]
    fn test_infer_extension_from_content_type() {
        assert_eq!(infer_extension_from_content_type("video/mp4"), Some("mp4"));
        assert_eq!(
            infer_extension_from_content_type("video/mp4; charset=utf-8"),
            Some("mp4")
        );
        assert_eq!(infer_extension_from_content_type("audio/mpeg"), Some("mp3"));
        assert_eq!(infer_extension_from_content_type("image/jpeg"), Some("jpg"));
        assert_eq!(
            infer_extension_from_content_type("application/octet-stream"),
            None
        );
    }

    #[test]
    fn test_s3_location_parsing() {
        // Valid S3 locations
        let parts: Vec<_> = "s3://my-bucket/path/to/file.mp4"[5..]
            .splitn(2, '/')
            .collect();
        assert_eq!(parts, vec!["my-bucket", "path/to/file.mp4"]);

        // Invalid formats would be caught by the validation logic in download_from_s3
    }
}
