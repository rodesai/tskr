use anyhow::Context;
use serde::{Deserialize, Serialize};

pub struct Client {
    http: reqwest::Client,
    url: String,
}

impl Client {
    pub fn new(cfg: &crate::config::Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            url: format!("{}/embed", cfg.embedding_url.trim_end_matches('/')),
        }
    }

    pub async fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let req = EmbedRequest { texts };
        let resp = self
            .http
            .post(&self.url)
            .json(&req)
            .send()
            .await
            .with_context(|| format!("POST {} failed", self.url))?
            .error_for_status()
            .with_context(|| format!("POST {} returned non-2xx", self.url))?;
        let body: EmbedResponse = resp
            .json()
            .await
            .with_context(|| format!("failed to decode {} response", self.url))?;
        if body.embeddings.len() != texts.len() {
            anyhow::bail!(
                "embedding count mismatch: requested {} texts, got {} embeddings",
                texts.len(),
                body.embeddings.len()
            );
        }
        Ok(body.embeddings)
    }
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    texts: &'a [String],
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}
