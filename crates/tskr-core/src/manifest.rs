use serde::{Deserialize, Serialize};

pub const MANIFEST_PATH_TAIL: &str = "manifest.json";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Manifest {
    pub session_id: String,
    pub author: String,
    pub repo: Option<String>,
    pub host: Option<String>,
    pub started_at: Option<String>,
    pub last_event_index: usize,
    pub segment_count: usize,
}

pub fn manifest_path(session_id: &str) -> String {
    format!("sessions/{session_id}/manifest.json")
}
