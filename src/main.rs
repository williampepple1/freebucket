mod config;
mod error;
mod models;
mod storage;
mod handlers;
mod dashboard;

use std::sync::Arc;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::storage::StorageEngine;

pub struct AppState {
    pub storage: StorageEngine,
    pub config: Config,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "freebucket=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::default();
    let storage = StorageEngine::new(&config.data_dir).expect("Failed to initialize storage engine");

    tracing::info!("Storage directory: {}", config.data_dir);
    tracing::info!("Starting FreeBucket on http://{}:{}", config.host, config.port);

    let state = Arc::new(AppState { storage, config: config.clone() });

    let app = Router::new()
        // Dashboard routes (web UI)
        .merge(dashboard::routes())
        // API routes (nestable, no wildcards)
        .nest("/api", handlers::api_routes())
        // API wildcard routes (must be at top level)
        .merge(handlers::api_wildcard_routes())
        // S3-compatible routes (no nesting needed)
        .merge(handlers::s3_routes())
        .merge(handlers::s3_wildcard_routes())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    println!(r#"
    ╔═══════════════════════════════════════════════════════╗
    ║                                                       ║
    ║   ███████╗██████╗ ███████╗███████╗                    ║
    ║   ██╔════╝██╔══██╗██╔════╝██╔════╝                    ║
    ║   █████╗  ██████╔╝█████╗  █████╗                      ║
    ║   ██╔══╝  ██╔══██╗██╔══╝  ██╔══╝                      ║
    ║   ██║     ██║  ██║███████╗███████╗                     ║
    ║   ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝                    ║
    ║              BUCKET                                    ║
    ║                                                       ║
    ║   Local S3-Compatible Storage Service                  ║
    ║                                                       ║
    ║   Dashboard:  http://{:<30}    ║
    ║   API:        http://{:<30}    ║
    ║                                                       ║
    ╚═══════════════════════════════════════════════════════╝
    "#, &addr, format!("{}/api", &addr));

    axum::serve(listener, app).await.unwrap();
}
