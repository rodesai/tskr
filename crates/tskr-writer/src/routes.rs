use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::trace::TraceLayer;
use tskr_core::event::RawEvent;
use tskr_core::{classify, Classification};

use crate::error::{Result, WriterError};

const MAX_UPLOAD_BYTES: usize = 32 * 1024 * 1024;

pub fn app() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/-/ready", get(ready))
        .route("/sessions/upload", post(upload))
        .layer(TraceLayer::new_for_http())
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn ready() -> impl IntoResponse {
    // TODO(iter 5): probe S3, embedding server, and vector deps before reporting ready.
    (StatusCode::OK, "ok")
}

async fn upload(req: Request<Body>) -> Result<impl IntoResponse> {
    let body = req.into_body();
    let bytes = to_bytes(body, MAX_UPLOAD_BYTES)
        .await
        .map_err(|e| WriterError::BadRequest(format!("failed to read body: {e}")))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|e| WriterError::BadRequest(format!("body is not valid utf-8: {e}")))?;

    let mut accepted = 0usize;
    let mut indexed = 0usize;

    for (idx, line) in text.split('\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        let raw = RawEvent::parse(line)
            .map_err(|e| WriterError::BadRequest(format!("line {}: {}", idx + 1, e)))?;
        accepted += 1;
        if matches!(classify(&raw), Classification::Index { .. }) {
            indexed += 1;
        }
    }

    // TODO(iter 5): call pipeline::run

    Ok(Json(json!({ "accepted": accepted, "indexed": indexed })))
}
