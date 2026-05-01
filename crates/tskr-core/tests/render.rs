use serde_json::json;
use tskr_core::{
    classify, manifest_path, render, segment_index, segment_path, Classification, Manifest,
    RawEvent, Role, MANIFEST_PATH_TAIL,
};

fn raw(value: serde_json::Value) -> RawEvent {
    let line = serde_json::to_string(&value).unwrap();
    RawEvent::parse(&line).unwrap()
}

#[test]
fn classifier_user_string() {
    let r = raw(json!({
        "type": "user",
        "message": { "content": "hello world" }
    }));
    assert_eq!(classify(&r), Classification::Index { role: Role::User });
}

#[test]
fn classifier_user_local_command_caveat_skipped() {
    let r = raw(json!({
        "type": "user",
        "message": { "content": "<local-command-caveat>foo</local-command-caveat>" }
    }));
    assert_eq!(classify(&r), Classification::Skip);
}

#[test]
fn classifier_user_tool_result() {
    let r = raw(json!({
        "type": "user",
        "message": {
            "content": [
                { "type": "tool_result", "content": "ok" }
            ]
        }
    }));
    assert_eq!(
        classify(&r),
        Classification::Index {
            role: Role::ToolResult
        }
    );
}

#[test]
fn classifier_assistant() {
    let r = raw(json!({
        "type": "assistant",
        "message": {
            "content": [
                { "type": "text", "text": "hi" }
            ]
        }
    }));
    assert_eq!(
        classify(&r),
        Classification::Index {
            role: Role::Assistant
        }
    );
}

#[test]
fn classifier_system_away_summary() {
    let r = raw(json!({
        "type": "system",
        "subtype": "away_summary",
        "summary": "stuff happened"
    }));
    assert_eq!(
        classify(&r),
        Classification::Index {
            role: Role::Summary
        }
    );
}

#[test]
fn classifier_misc_skipped() {
    for v in [
        json!({ "type": "permission-mode" }),
        json!({ "type": "file-history-snapshot" }),
        json!({ "type": "queue-operation" }),
        json!({ "type": "attachment" }),
        json!({ "type": "system", "subtype": "info" }),
        json!({ "type": "last-prompt" }),
        json!({ "type": "ai-title" }),
    ] {
        let r = raw(v.clone());
        assert_eq!(classify(&r), Classification::Skip, "should skip {v:?}");
    }
}

#[test]
fn renderer_assistant_text_and_tool_use_skips_thinking() {
    let r = raw(json!({
        "type": "assistant",
        "message": {
            "content": [
                { "type": "thinking", "thinking": "SECRET_REASONING_CONTENT", "signature": "sig" },
                { "type": "text", "text": "hello" },
                { "type": "tool_use", "name": "bash", "input": { "cmd": "ls" } }
            ]
        }
    }));
    let c = classify(&r);
    let chunk = render(&r, &c).expect("should render");
    assert_eq!(chunk.role, Role::Assistant);
    assert!(chunk.text.contains("hello"), "text was: {}", chunk.text);
    assert!(
        chunk.text.contains("tool_use: bash("),
        "text was: {}",
        chunk.text
    );
    assert!(
        !chunk.text.contains("SECRET_REASONING_CONTENT"),
        "thinking leaked: {}",
        chunk.text
    );
    assert!(!chunk.text.contains("\"signature\""));
}

#[test]
fn renderer_tool_result_truncated_to_4096_bytes() {
    let big = "a".repeat(10_000);
    let r = raw(json!({
        "type": "user",
        "message": {
            "content": [
                { "type": "tool_result", "content": big }
            ]
        }
    }));
    let c = classify(&r);
    let chunk = render(&r, &c).expect("should render");
    assert!(
        chunk.text.len() <= 4096,
        "expected ≤4096 bytes, got {}",
        chunk.text.len()
    );
}

#[test]
fn renderer_tool_result_array_content() {
    let r = raw(json!({
        "type": "user",
        "message": {
            "content": [
                {
                    "type": "tool_result",
                    "content": [
                        { "type": "text", "text": "line1" },
                        { "type": "text", "text": "line2" }
                    ]
                }
            ]
        }
    }));
    let c = classify(&r);
    let chunk = render(&r, &c).expect("should render");
    assert!(chunk.text.contains("line1"));
    assert!(chunk.text.contains("line2"));
}

#[test]
fn renderer_returns_none_for_whitespace() {
    let r = raw(json!({
        "type": "user",
        "message": { "content": "   \n  " }
    }));
    let c = classify(&r);
    assert!(render(&r, &c).is_none());
}

#[test]
fn renderer_returns_none_for_empty_assistant() {
    let r = raw(json!({
        "type": "assistant",
        "message": {
            "content": [
                { "type": "thinking", "thinking": "x", "signature": "s" }
            ]
        }
    }));
    let c = classify(&r);
    assert!(render(&r, &c).is_none());
}

#[test]
fn renderer_returns_none_for_skip() {
    let r = raw(json!({ "type": "permission-mode" }));
    let c = classify(&r);
    assert!(render(&r, &c).is_none());
}

#[test]
fn segment_index_buckets() {
    for i in 0..10 {
        assert_eq!(segment_index(i), 0, "i={i}");
    }
    for i in 10..20 {
        assert_eq!(segment_index(i), 1, "i={i}");
    }
}

#[test]
fn segment_path_zero_padded() {
    assert_eq!(segment_path("abc", 0), "sessions/abc/seg-00000.jsonl");
    assert_eq!(segment_path("abc", 12345), "sessions/abc/seg-12345.jsonl");
}

#[test]
fn manifest_round_trips() {
    let m = Manifest {
        session_id: "s1".into(),
        author: "rohan@example.com".into(),
        repo: Some("tskr".into()),
        host: Some("laptop".into()),
        started_at: Some("2026-05-01T00:00:00Z".into()),
        last_event_index: 42,
        segment_count: 5,
    };
    let s = serde_json::to_string(&m).unwrap();
    let back: Manifest = serde_json::from_str(&s).unwrap();
    assert_eq!(m, back);
}

#[test]
fn manifest_path_format() {
    assert_eq!(manifest_path("abc"), "sessions/abc/manifest.json");
    assert_eq!(MANIFEST_PATH_TAIL, "manifest.json");
}
