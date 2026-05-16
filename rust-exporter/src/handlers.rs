use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json},
};
use std::sync::Arc;

use crate::state::{AppState, History, MetricsResponse};

static INDEX_HTML: &str = include_str!("../static/index.html");

pub async fn index() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(INDEX_HTML),
    )
}

pub async fn api_metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current = state.current.read().unwrap().clone();
    let history_points = state.history.read().unwrap().clone();
    let history = History::from(&history_points);

    let response = MetricsResponse { current, history };

    (StatusCode::OK, Json(response))
}

pub async fn health() -> impl IntoResponse {
    StatusCode::OK
}