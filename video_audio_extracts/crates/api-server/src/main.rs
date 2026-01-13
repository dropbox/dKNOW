//! API Server Binary Entry Point

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use video_audio_api_server::{start_server, ApiState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "video_audio_api_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get bind address from environment or use default
    let addr = std::env::var("API_SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    // Create API state
    let state = ApiState::new();

    // Start server
    tracing::info!("Starting Video & Audio Processing API Server");
    start_server(&addr, state).await?;

    Ok(())
}
