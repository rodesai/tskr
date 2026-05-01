use anyhow::Context;
use serde::{Deserialize, Serialize};

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    embed_url: String,
}

impl Client {
    pub fn new(cfg: &crate::config::Config) -> Self {
        let base_url = cfg.embedding_url.trim_end_matches('/').to_string();
        let embed_url = format!("{base_url}/embed");
        Self {
            http: reqwest::Client::new(),
            base_url,
            embed_url,
        }
    }

    pub async fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let req = EmbedRequest { texts };
        let resp = self
            .http
            .post(&self.embed_url)
            .json(&req)
            .send()
            .await
            .with_context(|| format!("POST {} failed", self.embed_url))?
            .error_for_status()
            .with_context(|| format!("POST {} returned non-2xx", self.embed_url))?;
        let body: EmbedResponse = resp
            .json()
            .await
            .with_context(|| format!("failed to decode {} response", self.embed_url))?;
        if body.embeddings.len() != texts.len() {
            anyhow::bail!(
                "embedding count mismatch: requested {} texts, got {} embeddings",
                texts.len(),
                body.embeddings.len()
            );
        }
        Ok(body.embeddings)
    }

    pub async fn ready(&self) -> anyhow::Result<()> {
        let url = format!("{}/health", self.base_url);
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

#[derive(Serialize)]
struct EmbedRequest<'a> {
    texts: &'a [String],
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}
