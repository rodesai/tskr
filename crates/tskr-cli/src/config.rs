use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub writer_url: String,
    pub embed_url: String,
    pub vector_reader_url: String,
    pub s3_endpoint: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            writer_url: var_or("TSKR_WRITER_URL", "http://localhost:8090"),
            embed_url: var_or("TSKR_EMBED_URL", "http://localhost:9000"),
            vector_reader_url: var_or("TSKR_VECTOR_READER_URL", "http://localhost:18081"),
            s3_endpoint: var_or("TSKR_S3_ENDPOINT", "http://localhost:9100"),
            s3_bucket: var_or("TSKR_S3_BUCKET", "tskr"),
            s3_region: var_or("TSKR_S3_REGION", "us-east-1"),
            s3_access_key: var_or("TSKR_S3_ACCESS_KEY", "minioadmin"),
            s3_secret_key: var_or("TSKR_S3_SECRET_KEY", "minioadmin"),
        })
    }
}

fn var_or(name: &str, default: &str) -> String {
    match env::var(name) {
        Ok(v) if !v.is_empty() => v,
        _ => default.to_string(),
    }
}
