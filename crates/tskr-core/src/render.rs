use serde_json::Value;

use crate::event::{Classification, RawEvent, Role};

const TOOL_USE_INPUT_LIMIT: usize = 200;
const TOOL_RESULT_BYTE_LIMIT: usize = 4096;

pub struct RenderedChunk {
    pub role: Role,
    pub text: String,
}

pub fn render(raw: &RawEvent, classification: &Classification) -> Option<RenderedChunk> {
    let role = match classification {
        Classification::Skip => return None,
        Classification::Index { role } => *role,
    };

    let text = match role {
        Role::Assistant => render_assistant(&raw.value),
        Role::User => render_user_string(&raw.value),
        Role::ToolResult => render_user_tool_result(&raw.value),
        Role::Summary => render_away_summary(&raw.value),
    };

    if text.trim().is_empty() {
        None
    } else {
        Some(RenderedChunk { role, text })
    }
}

fn render_assistant(value: &Value) -> String {
    let blocks = match value.get("message").and_then(|m| m.get("content")) {
        Some(Value::Array(b)) => b,
        _ => return String::new(),
    };

    let mut pieces: Vec<String> = Vec::new();
    for block in blocks {
        let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match block_type {
            "text" => {
                if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                    if !t.is_empty() {
                        pieces.push(t.to_string());
                    }
                }
            }
            "tool_use" => {
                let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let input_str = match block.get("input") {
                    Some(v) => serde_json::to_string(v).unwrap_or_default(),
                    None => String::new(),
                };
                let truncated = truncate_chars(&input_str, TOOL_USE_INPUT_LIMIT);
                pieces.push(format!("tool_use: {name}({truncated})"));
            }
            _ => {}
        }
    }
    pieces.join("\n")
}

fn render_user_string(value: &Value) -> String {
    match value.get("message").and_then(|m| m.get("content")) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn render_user_tool_result(value: &Value) -> String {
    let blocks = match value.get("message").and_then(|m| m.get("content")) {
        Some(Value::Array(b)) => b,
        _ => return String::new(),
    };

    let mut pieces: Vec<String> = Vec::new();
    for block in blocks {
        let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if block_type != "tool_result" {
            continue;
        }
        let inner = match block.get("content") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Array(parts)) => {
                let mut acc = String::new();
                for part in parts {
                    let pt = part.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if pt == "text" {
                        if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                            if !acc.is_empty() {
                                acc.push('\n');
                            }
                            acc.push_str(t);
                        }
                    }
                }
                acc
            }
            _ => String::new(),
        };
        let truncated = truncate_bytes(&inner, TOOL_RESULT_BYTE_LIMIT);
        pieces.push(truncated);
    }
    pieces.join("\n")
}

fn render_away_summary(value: &Value) -> String {
    for key in ["summary", "content"] {
        if let Some(s) = value.get(key).and_then(|v| v.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => s[..idx].to_string(),
        None => s.to_string(),
    }
}

fn truncate_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}
