//! Error types for the plugin system

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Unsupported input format: {0}")]
    UnsupportedFormat(String),

    #[error("Plugin execution timed out: {0}")]
    Timeout(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("No plugin found for output type: {0}")]
    NoPluginForOutput(String),

    #[error("No plugin found for conversion: {from} -> {to}")]
    NoPluginForConversion { from: String, to: String },

    #[error("No source specified for operation")]
    NoSource,

    #[error("Plugin error: {0}")]
    PluginError(#[from] PluginError),

    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
