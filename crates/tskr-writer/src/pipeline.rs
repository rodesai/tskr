pub struct UploadRequest {
    pub session_id: String,
    pub author: String,
    pub repo: String,
    pub host: String,
    pub events: Vec<serde_json::Value>,
}

pub struct UploadOutcome {
    pub accepted: usize,
    pub indexed: usize,
}

pub struct PipelineCtx {
    pub s3: crate::s3::Client,
    pub embed: crate::embed::Client,
    pub vector: crate::vector::Client,
}

pub async fn run(_req: UploadRequest, _ctx: &PipelineCtx) -> anyhow::Result<UploadOutcome> {
    anyhow::bail!("pipeline::run not yet implemented (iter 5)")
}
