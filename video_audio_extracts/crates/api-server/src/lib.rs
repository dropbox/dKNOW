//! REST API Server for Video & Audio Processing
//!
//! Provides two processing modes:
//! - Real-time API: Minimum latency for single file (parallel CPU+GPU)
//! - Bulk API: Maximum throughput for batch processing (staged processing, ML batching)

mod download;
mod handlers;
mod types;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use video_audio_orchestrator::Orchestrator;

pub use handlers::*;
pub use types::*;

/// API server state shared across handlers
#[derive(Clone)]
pub struct ApiState {
    /// Orchestrator for executing media processing tasks
    pub orchestrator: Arc<Orchestrator>,
    /// Job results cache (`job_id` -> `JobResult`)
    pub results: Arc<RwLock<std::collections::HashMap<String, JobResult>>>,
}

impl ApiState {
    /// Create new API state
    #[must_use]
    pub fn new() -> Self {
        Self {
            orchestrator: Arc::new(Orchestrator::new()),
            results: Arc::new(RwLock::new(std::collections::HashMap::with_capacity(100))),
        }
    }
}

impl Default for ApiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the API router with all endpoints
pub fn build_router(state: ApiState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Real-time processing API
        .route("/api/v1/process/realtime", post(process_realtime))
        // Bulk processing API
        .route("/api/v1/process/bulk", post(process_bulk))
        // Status and query endpoints
        .route("/api/v1/jobs/{job_id}/status", get(get_job_status))
        .route("/api/v1/jobs/{job_id}/result", get(get_job_result))
        // Semantic search endpoint
        .route("/api/v1/search/similar", post(handlers::semantic_search))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Start the API server
pub async fn start_server(addr: &str, state: ApiState) -> Result<(), std::io::Error> {
    tracing::info!("Starting API server on {}", addr);

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_state_creation() {
        let state = ApiState::new();
        assert_eq!(state.results.blocking_read().len(), 0);
    }

    #[test]
    fn test_api_state_default() {
        let state = ApiState::default();
        assert_eq!(state.results.blocking_read().len(), 0);
    }
}
