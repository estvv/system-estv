mod collector;
mod handlers;
mod state;

use axum::{routing::get, Router};
use std::sync::Arc;
use tokio::signal;

use crate::state::AppState;

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::new());

    let collector_handle = tokio::spawn(collector::run_metrics_loop(Arc::clone(&state)));

    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/api/metrics", get(handlers::api_metrics))
        .route("/health", get(handlers::health))
        .route("/version.json", get(handlers::version_json))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .expect("Failed to bind to port 3001");

    println!("rust-exporter listening on 0.0.0.0:3001");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Failed to start server");

    collector_handle.abort();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("Shutdown signal received, stopping server...");
}