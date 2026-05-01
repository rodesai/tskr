use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let app = tskr_writer::app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn ready_returns_ok() {
    let app = tskr_writer::app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/-/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn upload_classifies_fixture() {
    let fixture = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/sessions/short-bug.jsonl"
    ))
    .unwrap();

    let app = tskr_writer::app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/sessions/upload")
                .body(Body::from(fixture))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let accepted = json["accepted"].as_u64().unwrap();
    let indexed = json["indexed"].as_u64().unwrap();
    assert!(accepted >= 1, "expected accepted >= 1, got {accepted}");
    assert!(indexed >= 1, "expected indexed >= 1, got {indexed}");
}
