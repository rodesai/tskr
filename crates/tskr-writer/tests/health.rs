use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn test_app() -> axum::Router {
    std::env::set_var("TSKR_S3_ENDPOINT", "http://127.0.0.1:1");
    std::env::set_var("TSKR_S3_ACCESS_KEY", "x");
    std::env::set_var("TSKR_S3_SECRET_KEY", "x");
    std::env::set_var("TSKR_EMBED_URL", "http://127.0.0.1:1");
    std::env::set_var("TSKR_VECTOR_WRITER_URL", "http://127.0.0.1:1");
    std::env::set_var("TSKR_VECTOR_READER_URL", "http://127.0.0.1:1");
    let cfg = tskr_writer::config::Config::from_env().unwrap();
    let s3 = tskr_writer::s3::Client::new(&cfg).await.unwrap();
    let embed = tskr_writer::embed::Client::new(&cfg);
    let vector = tskr_writer::vector::Client::new(&cfg);
    let state = std::sync::Arc::new(tskr_writer::routes::AppState {
        cfg,
        s3,
        embed,
        vector,
    });
    tskr_writer::routes::app(state)
}

#[tokio::test]
async fn health_returns_ok() {
    let app = test_app().await;
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
