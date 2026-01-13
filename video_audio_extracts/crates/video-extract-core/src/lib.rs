//! Video Extract Core - Plugin-based media processing architecture
//!
//! This crate provides the core abstractions for a high-performance plugin-based
//! media processing system inspired by Dropbox's Riviera architecture.

pub mod cache;
pub mod context;
pub mod error;
pub mod executor;
pub mod fast_path;
pub mod image_io;
pub mod onnx_utils;
pub mod operation;
pub mod parallel_pipeline;
pub mod plugin;
pub mod registry;

pub use cache::{CacheKey, CacheMetadata, PipelineCache, PipelineCacheStats};
pub use context::{Context, ExecutionMode};
pub use error::{PluginError, RegistryError};
pub use executor::{
    BulkExecutor, BulkFastPathResult, BulkFileResult, DebugExecutor, ExecutionResult,
    PerformanceExecutor, StageResult, StreamingResult,
};
pub use operation::{DataSource, Operation, OutputSpec};
pub use plugin::{
    PartialResult, Plugin, PluginConfig, PluginRequest, PluginResponse, PluginStreamingResponse,
};
pub use registry::{Pipeline, PipelineStage, Registry};
