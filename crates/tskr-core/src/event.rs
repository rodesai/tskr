use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RawEvent {
    pub event_type: String,
    pub value: Value,
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("invalid json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("event missing `type` discriminator")]
    MissingType,
    #[error("event `type` is not a string")]
    TypeNotString,
    #[error("event is not a json object")]
    NotObject,
}

impl RawEvent {
    pub fn parse(line: &str) -> Result<Self, ParseError> {
        let value: Value = serde_json::from_str(line)?;
        let obj = value.as_object().ok_or(ParseError::NotObject)?;
        let type_value = obj.get("type").ok_or(ParseError::MissingType)?;
        let event_type = type_value
            .as_str()
            .ok_or(ParseError::TypeNotString)?
            .to_string();
        Ok(RawEvent { event_type, value })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    ToolResult,
    Summary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    Skip,
    Index { role: Role },
}

const LOCAL_COMMAND_CAVEAT: &str = "<local-command-caveat>";

pub fn classify(raw: &RawEvent) -> Classification {
    match raw.event_type.as_str() {
        "assistant" => Classification::Index {
            role: Role::Assistant,
        },
        "user" => classify_user(&raw.value),
        "system" => {
            let subtype = raw
                .value
                .get("subtype")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if subtype == "away_summary" {
                Classification::Index {
                    role: Role::Summary,
                }
            } else {
                Classification::Skip
            }
        }
        _ => Classification::Skip,
    }
}

fn classify_user(value: &Value) -> Classification {
    let content = value.get("message").and_then(|m| m.get("content"));
    match content {
        Some(Value::String(s)) => {
            if s.starts_with(LOCAL_COMMAND_CAVEAT) {
                Classification::Skip
            } else {
                Classification::Index { role: Role::User }
            }
        }
        Some(Value::Array(blocks)) => {
            let has_tool_result = blocks.iter().any(|b| {
                b.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "tool_result")
                    .unwrap_or(false)
            });
            if has_tool_result {
                Classification::Index {
                    role: Role::ToolResult,
                }
            } else {
                Classification::Skip
            }
        }
        _ => Classification::Skip,
    }
}

pub fn session_id(raw: &RawEvent) -> Option<&str> {
    raw.value.get("sessionId").and_then(|v| v.as_str())
}

pub fn timestamp(raw: &RawEvent) -> Option<&str> {
    raw.value.get("timestamp").and_then(|v| v.as_str())
}

pub fn model(raw: &RawEvent) -> Option<&str> {
    raw.value
        .get("message")
        .and_then(|m| m.get("model"))
        .and_then(|m| m.as_str())
}
