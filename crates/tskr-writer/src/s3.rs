use anyhow::Context;
use aws_sdk_s3::primitives::ByteStream;

pub struct Client {
    inner: aws_sdk_s3::Client,
    bucket: String,
}

impl Client {
    pub async fn new(cfg: &crate::config::Config) -> anyhow::Result<Self> {
        let credentials = aws_sdk_s3::config::Credentials::new(
            &cfg.s3_access_key,
            &cfg.s3_secret_key,
            None,
            None,
            "tskr-writer",
        );
        let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(cfg.s3_region.clone()))
            .credentials_provider(credentials)
            .load()
            .await;
        let conf = aws_sdk_s3::config::Builder::from(&shared_config)
            .endpoint_url(&cfg.s3_endpoint)
            .force_path_style(true)
            .build();
        let inner = aws_sdk_s3::Client::from_conf(conf);
        Ok(Self {
            inner,
            bucket: cfg.s3_bucket.clone(),
        })
    }

    pub async fn ensure_bucket(&self) -> anyhow::Result<()> {
        match self.inner.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => Ok(()),
            Err(err) => {
                let svc = err.into_service_error();
                if svc.is_not_found() {
                    self.inner
                        .create_bucket()
                        .bucket(&self.bucket)
                        .send()
                        .await
                        .with_context(|| format!("create_bucket({}) failed", self.bucket))?;
                    tracing::info!(bucket = %self.bucket, "ensured bucket exists");
                    Ok(())
                } else {
                    Err(anyhow::Error::new(svc)
                        .context(format!("head_bucket({}) failed", self.bucket)))
                }
            }
        }
    }

    pub async fn ready(&self) -> anyhow::Result<()> {
        self.inner
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .with_context(|| format!("head_bucket failed for {}", self.bucket))?;
        Ok(())
    }

    pub async fn get_segment(
        &self,
        session_id: &str,
        segment_index: usize,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let key = tskr_core::segment_path(session_id, segment_index);
        let result = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await;
        match result {
            Ok(output) => {
                let bytes = output
                    .body
                    .collect()
                    .await
                    .with_context(|| format!("get_segment read body failed for key {key}"))?
                    .into_bytes();
                Ok(Some(bytes.to_vec()))
            }
            Err(err) => {
                let service_err = err.into_service_error();
                if service_err.is_no_such_key() {
                    return Ok(None);
                }
                Err(anyhow::Error::new(service_err)
                    .context(format!("get_segment failed for key {key}")))
            }
        }
    }

    pub async fn put_segment(
        &self,
        session_id: &str,
        segment_index: usize,
        body: Vec<u8>,
    ) -> anyhow::Result<()> {
        let key = tskr_core::segment_path(session_id, segment_index);
        self.inner
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(body))
            .content_type("application/x-ndjson")
            .send()
            .await
            .with_context(|| format!("put_segment failed for key {key}"))?;
        Ok(())
    }

    pub async fn put_manifest(&self, session_id: &str, body: Vec<u8>) -> anyhow::Result<()> {
        let key = tskr_core::manifest_path(session_id);
        self.inner
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(body))
            .content_type("application/json")
            .send()
            .await
            .with_context(|| format!("put_manifest failed for key {key}"))?;
        Ok(())
    }

    pub async fn get_manifest(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Option<tskr_core::Manifest>> {
        let key = tskr_core::manifest_path(session_id);
        let result = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await;
        let output = match result {
            Ok(output) => output,
            Err(err) => {
                let service_err = err.into_service_error();
                if service_err.is_no_such_key() {
                    return Ok(None);
                }
                return Err(anyhow::Error::new(service_err)
                    .context(format!("get_manifest failed for key {key}")));
            }
        };
        let bytes = output
            .body
            .collect()
            .await
            .with_context(|| format!("get_manifest read body failed for key {key}"))?
            .into_bytes();
        let manifest: tskr_core::Manifest = serde_json::from_slice(&bytes)
            .with_context(|| format!("get_manifest parse failed for key {key}"))?;
        Ok(Some(manifest))
    }

    pub async fn list_segment_indices(&self, session_id: &str) -> anyhow::Result<Vec<usize>> {
        let prefix = format!("sessions/{session_id}/seg-");
        let mut indices: Vec<usize> = Vec::new();
        let mut continuation: Option<String> = None;
        loop {
            let mut req = self
                .inner
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&prefix);
            if let Some(token) = continuation.as_ref() {
                req = req.continuation_token(token);
            }
            let output = req
                .send()
                .await
                .with_context(|| format!("list_segment_indices failed for prefix {prefix}"))?;
            for obj in output.contents() {
                let Some(key) = obj.key() else { continue };
                let Some(rest) = key.strip_prefix(&prefix) else {
                    continue;
                };
                let Some(num) = rest.strip_suffix(".jsonl") else {
                    continue;
                };
                if let Ok(idx) = num.parse::<usize>() {
                    indices.push(idx);
                }
            }
            if output.is_truncated().unwrap_or(false) {
                continuation = output.next_continuation_token().map(|s| s.to_string());
                if continuation.is_none() {
                    break;
                }
            } else {
                break;
            }
        }
        indices.sort_unstable();
        Ok(indices)
    }
}
