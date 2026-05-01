use anyhow::Context;

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
            "tskr-cli",
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

    pub async fn get_manifest(&self, session_id: &str) -> anyhow::Result<tskr_core::Manifest> {
        let key = tskr_core::manifest_path(session_id);
        let output = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .with_context(|| format!("get_manifest failed for key {key}"))?;
        let bytes = output
            .body
            .collect()
            .await
            .with_context(|| format!("get_manifest read body failed for key {key}"))?
            .into_bytes();
        let manifest: tskr_core::Manifest = serde_json::from_slice(&bytes)
            .with_context(|| format!("get_manifest parse failed for key {key}"))?;
        Ok(manifest)
    }

    pub async fn get_segment(
        &self,
        session_id: &str,
        segment_index: usize,
    ) -> anyhow::Result<Vec<u8>> {
        let key = tskr_core::segment_path(session_id, segment_index);
        let output = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .with_context(|| format!("get_segment failed for key {key}"))?;
        let bytes = output
            .body
            .collect()
            .await
            .with_context(|| format!("get_segment read body failed for key {key}"))?
            .into_bytes();
        Ok(bytes.to_vec())
    }
}
