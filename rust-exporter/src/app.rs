use axum::{routing::get, Router};
use std::sync::Arc;
use sysinfo::System;

pub struct AppState {
    pub sys: Arc<std::sync::Mutex<System>>,
    pub host: String,
}

pub fn create_app() -> Router {
    let sys = Arc::new(std::sync::Mutex::new(System::new_all()));
    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "vps".to_string());

    let state = Arc::new(AppState { sys, host });

    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(super::metrics::collect))
        .with_state(state)
}

async fn health() -> axum::http::StatusCode {
    axum::http::StatusCode::OK
}