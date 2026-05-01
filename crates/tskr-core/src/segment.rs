use serde_json::Value;

pub const SEGMENT_SIZE: usize = 10;

pub fn segment_index(event_index: usize) -> usize {
    event_index / SEGMENT_SIZE
}

/// Builds the canonical S3 key tail for a segment.
///
/// Uses a five-digit zero-padded prefix; this caps a session at 100,000
/// segments (one million events). Milestone 1 will not approach that bound.
pub fn segment_path(session_id: &str, segment_index: usize) -> String {
    assert!(segment_index < 100_000, "segment_index exceeds 5-digit cap");
    format!("sessions/{session_id}/seg-{segment_index:05}.jsonl")
}

pub struct Segment {
    pub session_id: String,
    pub segment_index: usize,
    pub events: Vec<Value>,
}

pub fn split_events(events: Vec<Value>) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    for (idx, ev) in events.into_iter().enumerate() {
        let seg_idx = segment_index(idx);
        if out.last().map(|s| s.segment_index) != Some(seg_idx) {
            out.push(Segment {
                session_id: String::new(),
                segment_index: seg_idx,
                events: Vec::new(),
            });
        }
        if let Some(last) = out.last_mut() {
            last.events.push(ev);
        }
    }
    out
}
