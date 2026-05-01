use anyhow::Context;
use serde::Deserialize;
use serde_json::{json, Map, Value};

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("search response missing required attribute `{0}`")]
    MissingAttribute(&'static str),
}

pub struct Client {
    http: reqwest::Client,
    search_url: String,
}

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub vector: Vec<f32>,
    pub k: usize,
    pub eq_filters: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub id: String,
    pub score: f32,
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
        let reader_base_url = cfg.vector_reader_url.trim_end_matches('/').to_string();
        let search_url = format!("{reader_base_url}/api/v1/vector/search");
        Self {
            http: reqwest::Client::new(),
            search_url,
        }
    }

    pub async fn search(&self, req: SearchRequest) -> anyhow::Result<Vec<SearchHit>> {
        let mut body = Map::new();
        body.insert("vector".into(), json!(req.vector));
        body.insert("k".into(), json!(req.k as u32));
        if !req.eq_filters.is_empty() {
            let filter = build_filter(&req.eq_filters);
            body.insert("filter".into(), filter);
        }
        let payload = Value::Object(body);

        tracing::debug!(url = %self.search_url, k = req.k, "vector search request");

        let resp = self
            .http
            .post(&self.search_url)
            .header("content-type", "application/protobuf+json")
            .json(&payload)
            .send()
            .await
            .with_context(|| format!("POST {} failed", self.search_url))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("POST {} returned {}: {}", self.search_url, status, text);
        }
        let parsed: SearchResponse = resp
            .json()
            .await
            .with_context(|| format!("failed to decode {} response", self.search_url))?;

        let mut hits = Vec::with_capacity(parsed.results.len());
        for raw in parsed.results {
            hits.push(decode_hit(raw)?);
        }
        Ok(hits)
    }
}

fn build_filter(eq_filters: &[(String, String)]) -> Value {
    if eq_filters.len() == 1 {
        let (field, value) = &eq_filters[0];
        eq_clause(field, value)
    } else {
        let arr: Vec<Value> = eq_filters.iter().map(|(f, v)| eq_clause(f, v)).collect();
        json!({ "and": arr })
    }
}

fn eq_clause(field: &str, value: &str) -> Value {
    json!({ "eq": { "field": field, "value": value } })
}

fn decode_hit(raw: RawHit) -> anyhow::Result<SearchHit> {
    let attrs = raw.vector.attributes;
    let session_id = req_str(&attrs, "session_id")?;
    let event_index = req_i64(&attrs, "event_index")?;
    let segment_index = req_i64(&attrs, "segment_index")?;
    let author = req_str(&attrs, "author")?;
    let role = req_str(&attrs, "role")?;
    let timestamp = req_str(&attrs, "timestamp")?;
    let text = req_str(&attrs, "text")?;
    let repo = opt_str(&attrs, "repo");
    let model = opt_str(&attrs, "model");
    Ok(SearchHit {
        id: raw.vector.id,
        score: raw.score,
        session_id,
        event_index,
        segment_index,
        author,
        repo,
        model,
        role,
        timestamp,
        text,
    })
}

fn req_str(attrs: &Map<String, Value>, key: &'static str) -> anyhow::Result<String> {
    attrs
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| DecodeError::MissingAttribute(key).into())
}

fn req_i64(attrs: &Map<String, Value>, key: &'static str) -> anyhow::Result<i64> {
    attrs
        .get(key)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| DecodeError::MissingAttribute(key).into())
}

fn opt_str(attrs: &Map<String, Value>, key: &str) -> Option<String> {
    attrs
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[derive(Deserialize)]
struct SearchResponse {
    #[allow(dead_code)]
    status: Option<String>,
    #[serde(default)]
    results: Vec<RawHit>,
}

#[derive(Deserialize)]
struct RawHit {
    score: f32,
    vector: RawVector,
}

#[derive(Deserialize)]
struct RawVector {
    id: String,
    attributes: Map<String, Value>,
}
