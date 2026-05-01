use anyhow::Context;
use serde::Serialize;
use serde_json::Value;

pub struct Client {
    http: reqwest::Client,
    writer_base_url: String,
    reader_base_url: String,
    writer_url: String,
}

#[derive(Serialize)]
pub struct UpsertRow {
    pub id: String,
    pub vector: Vec<f32>,
    pub session_id: String,
    pub event_index: i64,
    pub segment_index: i64,
    pub author: String,
    pub repo: Option<String>,
    pub model: Option<String>,
    pub role: String,
    pub timestamp: String,
    pub text: String,
}

impl Client {
    pub fn new(cfg: &crate::config::Config) -> Self {
        let writer_base_url = cfg.vector_writer_url.trim_end_matches('/').to_string();
        let reader_base_url = cfg.vector_reader_url.trim_end_matches('/').to_string();
        let writer_url = format!("{writer_base_url}/api/v1/vector/write");
        Self {
            http: reqwest::Client::new(),
            writer_base_url,
            reader_base_url,
            writer_url,
        }
    }

    pub async fn upsert(&self, rows: Vec<UpsertRow>) -> anyhow::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let payload = serde_json::json!({
            "upsertVectors": rows.into_iter().map(row_to_json).collect::<Vec<_>>()
        });
        let resp = self
            .http
            .post(&self.writer_url)
            .header("content-type", "application/protobuf+json")
            .json(&payload)
            .send()
            .await
            .with_context(|| format!("POST {} failed", self.writer_url))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("POST {} returned {}: {}", self.writer_url, status, body);
        }
        Ok(())
    }

    pub async fn ready_writer(&self) -> anyhow::Result<()> {
        let url = format!("{}/-/ready", self.writer_base_url);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET {url} failed"))?;
        if !resp.status().is_success() {
            anyhow::bail!("GET {url} returned {}", resp.status());
        }
        Ok(())
    }

    pub async fn ready_reader(&self) -> anyhow::Result<()> {
        let url = format!("{}/-/ready", self.reader_base_url);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET {url} failed"))?;
        if !resp.status().is_success() {
            anyhow::bail!("GET {url} returned {}", resp.status());
        }
        Ok(())
    }
}

fn row_to_json(row: UpsertRow) -> Value {
    let mut attrs = serde_json::Map::new();
    attrs.insert("vector".into(), serde_json::to_value(&row.vector).unwrap());
    attrs.insert("session_id".into(), Value::String(row.session_id));
    attrs.insert("event_index".into(), Value::from(row.event_index));
    attrs.insert("segment_index".into(), Value::from(row.segment_index));
    attrs.insert("author".into(), Value::String(row.author));
    if let Some(repo) = row.repo {
        attrs.insert("repo".into(), Value::String(repo));
    }
    if let Some(model) = row.model {
        attrs.insert("model".into(), Value::String(model));
    }
    attrs.insert("role".into(), Value::String(row.role));
    attrs.insert("timestamp".into(), Value::String(row.timestamp));
    attrs.insert("text".into(), Value::String(row.text));
    serde_json::json!({ "id": row.id, "attributes": Value::Object(attrs) })
}
