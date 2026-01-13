//! Configuration loading for sg-daemon.

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    pub daemon: Option<DaemonConfig>,
    pub resources: Option<ResourceConfig>,
    pub indexing: Option<IndexingConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct DaemonConfig {
    pub socket: Option<PathBuf>,
    pub autostart: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ResourceConfig {
    pub max_cpu_active: Option<u64>,
    pub max_cpu_idle: Option<u64>,
    pub max_total_mb: Option<u64>,
    pub max_per_project_mb: Option<u64>,
    pub max_ram_mb: Option<u64>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct IndexingConfig {
    pub idle_threshold_secs: Option<u64>,
    pub stale_project_days: Option<u64>,
}

/// Default maximum total storage in MB (2GB)
pub const DEFAULT_MAX_TOTAL_MB: u64 = 2048;

/// Default maximum storage per project in MB (500MB)
pub const DEFAULT_MAX_PER_PROJECT_MB: u64 = 500;

impl Config {
    pub fn daemon_socket_path(&self) -> Option<PathBuf> {
        self.daemon
            .as_ref()
            .and_then(|daemon| daemon.socket.clone())
    }

    pub fn stale_threshold_secs(&self) -> Option<u64> {
        self.indexing
            .as_ref()
            .and_then(|indexing| indexing.stale_project_days)
            .map(|days| days.saturating_mul(24 * 60 * 60))
    }

    pub fn idle_threshold_secs(&self) -> Option<u64> {
        self.indexing
            .as_ref()
            .and_then(|indexing| indexing.idle_threshold_secs)
    }

    /// Get maximum total storage in bytes.
    /// Returns configured value or default (2GB).
    pub fn max_total_bytes(&self) -> u64 {
        self.resources
            .as_ref()
            .and_then(|r| r.max_total_mb)
            .unwrap_or(DEFAULT_MAX_TOTAL_MB)
            .saturating_mul(1024 * 1024)
    }

    /// Get maximum storage per project in bytes.
    /// Returns configured value or default (500MB).
    pub fn max_per_project_bytes(&self) -> u64 {
        self.resources
            .as_ref()
            .and_then(|r| r.max_per_project_mb)
            .unwrap_or(DEFAULT_MAX_PER_PROJECT_MB)
            .saturating_mul(1024 * 1024)
    }
}

pub fn default_config_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "sg").context("Could not determine config directory")?;
    Ok(dirs.config_dir().join("config.toml"))
}

pub fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let config: Config =
        toml::from_str(&contents).context("Failed to parse config file as TOML")?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert!(config.stale_threshold_secs().is_none());
        assert!(config.idle_threshold_secs().is_none());
    }

    #[test]
    fn test_stale_threshold_secs() {
        let config = Config {
            indexing: Some(IndexingConfig {
                stale_project_days: Some(2),
                idle_threshold_secs: None,
            }),
            ..Default::default()
        };
        assert_eq!(config.stale_threshold_secs(), Some(2 * 24 * 60 * 60));
    }

    #[test]
    fn test_max_total_bytes_default() {
        let config = Config::default();
        // Default is 2GB = 2048 MB = 2048 * 1024 * 1024 bytes
        assert_eq!(config.max_total_bytes(), 2048 * 1024 * 1024);
    }

    #[test]
    fn test_max_total_bytes_configured() {
        let config = Config {
            resources: Some(ResourceConfig {
                max_total_mb: Some(1024), // 1GB
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(config.max_total_bytes(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_max_per_project_bytes_default() {
        let config = Config::default();
        // Default is 500 MB = 500 * 1024 * 1024 bytes
        assert_eq!(config.max_per_project_bytes(), 500 * 1024 * 1024);
    }

    #[test]
    fn test_max_per_project_bytes_configured() {
        let config = Config {
            resources: Some(ResourceConfig {
                max_per_project_mb: Some(100), // 100MB
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(config.max_per_project_bytes(), 100 * 1024 * 1024);
    }

    #[test]
    fn test_daemon_socket_path_configured() {
        let config = Config {
            daemon: Some(DaemonConfig {
                socket: Some(PathBuf::from("/tmp/sg-test.sock")),
                autostart: None,
            }),
            ..Default::default()
        };

        assert_eq!(
            config.daemon_socket_path().as_deref(),
            Some(Path::new("/tmp/sg-test.sock"))
        );
    }
}
