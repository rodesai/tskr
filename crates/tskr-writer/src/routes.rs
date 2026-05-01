use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::trace::TraceLayer;
use tskr_core::event::RawEvent;

use crate::error::{Result, WriterError};
use crate::pipeline::{self, UploadRequest};

const MAX_UPLOAD_BYTES: usize = 32 * 1024 * 1024;

pub struct AppState {
    pub cfg: crate::config::Config,
    pub s3: crate::s3::Client,
    pub embed: crate::embed::Client,
    pub vector: crate::vector::Client,
}

pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/-/ready", get(ready))
        .route("/sessions/upload", post(upload))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn ready(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let (s3_res, embed_res, writer_res, reader_res) = tokio::join!(
        state.s3.ready(),
        state.embed.ready(),
        state.vector.ready_writer(),
        state.vector.ready_reader(),
    );

    let mut failed: Vec<&'static str> = Vec::new();
    if s3_res.is_err() {
        failed.push("s3");
    }
    if embed_res.is_err() {
        failed.push("embed");
    }
    if writer_res.is_err() {
        failed.push("vector_writer");
    }
    if reader_res.is_err() {
        failed.push("vector_reader");
    }

    if failed.is_empty() {
        (StatusCode::OK, Json(json!({ "status": "ready" })))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready", "failed": failed })),
        )
    }
}

async fn upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    req: Request<Body>,
) -> Result<impl IntoResponse> {
    let author = required_header(&headers, "X-Tskr-Author")?;
    let repo = required_header(&headers, "X-Tskr-Repo")?;
    let host = required_header(&headers, "X-Tskr-Host")?;
    let start_event_index = match headers.get("X-Tskr-Start-Event-Index") {
        Some(value) => {
            let s = value.to_str().map_err(|_| {
                WriterError::BadRequest("X-Tskr-Start-Event-Index is not valid ascii".to_string())
            })?;
            s.parse::<usize>()
                .map_err(|e| WriterError::BadRequest(format!("X-Tskr-Start-Event-Index: {e}")))?
        }
        None => 0,
    };

    let body = req.into_body();
    let bytes = to_bytes(body, MAX_UPLOAD_BYTES)
        .await
        .map_err(|e| WriterError::BadRequest(format!("failed to read body: {e}")))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|e| WriterError::BadRequest(format!("body is not valid utf-8: {e}")))?;

    let mut events: Vec<RawEvent> = Vec::new();
    for (idx, line) in text.split('\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        let raw = RawEvent::parse(line)
            .map_err(|e| WriterError::BadRequest(format!("line {}: {}", idx + 1, e)))?;
        events.push(raw);
    }

    let session_id = events
        .iter()
        .find_map(tskr_core::session_id)
        .map(|s| s.to_string())
        .ok_or_else(|| WriterError::BadRequest("could not determine session_id".to_string()))?;

    let upload_req = UploadRequest {
        session_id,
        author,
        repo,
        host,
        start_event_index,
        events,
    };

    let outcome = pipeline::run(upload_req, &state)
        .await
        .map_err(WriterError::Internal)?;

    Ok(Json(
        json!({ "accepted": outcome.accepted, "indexed": outcome.indexed }),
    ))
}

fn required_header(headers: &HeaderMap, name: &str) -> Result<String> {
    let value = headers
        .get(name)
        .ok_or_else(|| WriterError::BadRequest(format!("missing header {name}")))?;
    let s = value
        .to_str()
        .map_err(|_| WriterError::BadRequest(format!("header {name} is not valid ascii")))?;
    Ok(s.to_string())
}
