use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(thiserror::Error, Debug)]
pub enum WriterError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for WriterError {
    fn into_response(self) -> Response {
        match self {
            WriterError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
            }
            WriterError::Internal(err) => {
                tracing::error!(error = ?err, "internal writer error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal" })),
                )
                    .into_response()
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, WriterError>;
