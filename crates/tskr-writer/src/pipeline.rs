use std::collections::BTreeMap;

use tskr_core::event::{RawEvent, Role};
use tskr_core::Manifest;

pub struct UploadRequest {
    pub session_id: String,
    pub author: String,
    pub repo: String,
    pub host: String,
    pub start_event_index: usize,
    pub events: Vec<RawEvent>,
}

pub struct UploadOutcome {
    pub accepted: usize,
    pub indexed: usize,
}

pub async fn run(
    req: UploadRequest,
    state: &crate::routes::AppState,
) -> anyhow::Result<UploadOutcome> {
    let accepted = req.events.len();

    let existing = state.s3.get_manifest(&req.session_id).await?;
    let last_persisted: Option<usize> = existing.as_ref().map(|m| m.last_event_index);

    let survivors: Vec<(usize, &RawEvent)> = req
        .events
        .iter()
        .enumerate()
        .map(|(i, raw)| (req.start_event_index + i, raw))
        .filter(|(global_idx, _)| match last_persisted {
            Some(last) => *global_idx > last,
            None => true,
        })
        .collect();

    if survivors.is_empty() {
        return Ok(UploadOutcome {
            accepted,
            indexed: 0,
        });
    }

    let mut by_segment: BTreeMap<usize, Vec<(usize, &RawEvent)>> = BTreeMap::new();
    for (global_idx, raw) in &survivors {
        by_segment
            .entry(global_idx / 10)
            .or_default()
            .push((*global_idx, *raw));
    }

    let mut segment_bodies: Vec<(usize, Vec<u8>)> = Vec::with_capacity(by_segment.len());
    let lowest_seg = *by_segment.keys().next().expect("by_segment non-empty");
    for (seg_idx, items) in &by_segment {
        let mut lines: Vec<String> = Vec::new();
        if *seg_idx == lowest_seg {
            let first_global = items[0].0;
            if first_global > seg_idx * 10 {
                if let Some(existing_bytes) =
                    state.s3.get_segment(&req.session_id, *seg_idx).await?
                {
                    let text = String::from_utf8(existing_bytes)?;
                    for line in text.split('\n') {
                        if !line.is_empty() {
                            lines.push(line.to_string());
                        }
                    }
                }
            }
        }
        for (_, raw) in items {
            lines.push(serde_json::to_string(&raw.value)?);
        }
        let mut body = lines.join("\n");
        body.push('\n');
        segment_bodies.push((*seg_idx, body.into_bytes()));
    }

    let mut renderable: Vec<(usize, &RawEvent, tskr_core::RenderedChunk)> = Vec::new();
    for (global_idx, raw) in &survivors {
        let classification = tskr_core::classify(raw);
        if let Some(chunk) = tskr_core::render(raw, &classification) {
            renderable.push((*global_idx, *raw, chunk));
        }
    }

    let texts: Vec<String> = renderable.iter().map(|(_, _, c)| c.text.clone()).collect();
    let embeddings = state.embed.embed(&texts).await?;

    let mut rows: Vec<crate::vector::UpsertRow> = Vec::with_capacity(renderable.len());
    for ((global_idx, raw, chunk), vector) in renderable.iter().zip(embeddings) {
        let role = match chunk.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::ToolResult => "tool_result",
            Role::Summary => "summary",
        };
        rows.push(crate::vector::UpsertRow {
            id: format!("{}:{}", req.session_id, global_idx),
            vector,
            session_id: req.session_id.clone(),
            event_index: *global_idx as i64,
            segment_index: (*global_idx / 10) as i64,
            author: req.author.clone(),
            repo: Some(req.repo.clone()),
            model: tskr_core::model(raw).map(|s| s.to_string()),
            role: role.to_string(),
            timestamp: tskr_core::timestamp(raw).unwrap_or("").to_string(),
            text: chunk.text.clone(),
        });
    }

    let indexed = rows.len();
    state.vector.upsert(rows).await?;

    for (seg_idx, body) in segment_bodies {
        state.s3.put_segment(&req.session_id, seg_idx, body).await?;
    }

    let highest_global_index = survivors.iter().map(|(idx, _)| *idx).max().unwrap();
    let segment_count = highest_global_index / 10 + 1;
    let started_at = match existing.as_ref().and_then(|m| m.started_at.clone()) {
        Some(s) => Some(s),
        None => tskr_core::timestamp(&req.events[0]).map(|s| s.to_string()),
    };

    let manifest = Manifest {
        session_id: req.session_id.clone(),
        author: req.author.clone(),
        repo: Some(req.repo.clone()),
        host: Some(req.host.clone()),
        started_at,
        last_event_index: highest_global_index,
        segment_count,
    };
    state
        .s3
        .put_manifest(&req.session_id, serde_json::to_vec(&manifest)?)
        .await?;

    Ok(UploadOutcome { accepted, indexed })
}
