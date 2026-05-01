use std::env;
use std::net::SocketAddr;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub s3_endpoint: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub embedding_url: String,
    pub vector_writer_url: String,
    pub vector_reader_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_addr = env::var("TSKR_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8090".to_string())
            .parse::<SocketAddr>()
            .context("TSKR_BIND_ADDR is not a valid socket address")?;

        let s3_endpoint = require_var("TSKR_S3_ENDPOINT")?;
        let s3_bucket = env::var("TSKR_S3_BUCKET").unwrap_or_else(|_| "tskr".to_string());
        let s3_region = env::var("TSKR_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let s3_access_key = require_var("TSKR_S3_ACCESS_KEY")?;
        let s3_secret_key = require_var("TSKR_S3_SECRET_KEY")?;
        let embedding_url = require_var("TSKR_EMBED_URL")?;
        let vector_writer_url = require_var("TSKR_VECTOR_WRITER_URL")?;
        let vector_reader_url = require_var("TSKR_VECTOR_READER_URL")?;

        Ok(Self {
            bind_addr,
            s3_endpoint,
            s3_bucket,
            s3_region,
            s3_access_key,
            s3_secret_key,
            embedding_url,
            vector_writer_url,
            vector_reader_url,
        })
    }
}

fn require_var(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("environment variable {name} is required"))
}
