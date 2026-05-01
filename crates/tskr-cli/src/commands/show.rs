use crate::cli::ShowArgs;
use crate::config::Config;
use crate::s3::Client as S3Client;
use tskr_core::{classify, render, Classification, RawEvent, SEGMENT_SIZE};

pub async fn run(cfg: Config, args: ShowArgs) -> anyhow::Result<()> {
    let s3 = S3Client::new(&cfg).await?;
    let manifest = s3.get_manifest(&args.session_id).await?;
    let at_event = args.at_event.unwrap_or(0);

    if at_event > manifest.last_event_index {
        eprintln!(
            "--at-event {at_event} exceeds last_event_index {} for session {}",
            manifest.last_event_index, args.session_id
        );
        std::process::exit(1);
    }

    let segment_index = at_event / SEGMENT_SIZE;
    let bytes = s3.get_segment(&args.session_id, segment_index).await?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|e| anyhow::anyhow!("segment {segment_index} is not utf-8: {e}"))?;
    let mut events: Vec<RawEvent> = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let raw = RawEvent::parse(line).map_err(|e| {
            anyhow::anyhow!("failed to parse segment {segment_index} line {line_idx}: {e}")
        })?;
        events.push(raw);
    }

    if events.is_empty() {
        eprintln!(
            "segment {segment_index} for session {} has no events",
            args.session_id
        );
        std::process::exit(1);
    }

    let local_at = at_event - segment_index * SEGMENT_SIZE;
    let last_local = events.len() - 1;
    let start_local = local_at.saturating_sub(2);
    let end_local = std::cmp::min(local_at + 2, last_local);

    let repo = manifest.repo.as_deref().unwrap_or("unknown");
    println!(
        "session={sid} author={author} repo={repo} last_event_index={last}",
        sid = manifest.session_id,
        author = manifest.author,
        repo = repo,
        last = manifest.last_event_index,
    );

    for (i, raw) in events
        .iter()
        .enumerate()
        .take(end_local + 1)
        .skip(start_local)
    {
        let global_idx = segment_index * SEGMENT_SIZE + i;
        let classification = classify(raw);
        let role_str = match classification {
            Classification::Index { role } => role_to_str(role),
            Classification::Skip => "N/A".to_string(),
        };
        println!(
            "--- event={global_idx} type={t} role={role} ---",
            t = raw.event_type,
            role = role_str,
        );
        match render(raw, &classification) {
            Some(rendered) => println!("{}", rendered.text),
            None => println!("(non-indexed event)"),
        }
    }
    Ok(())
}

fn role_to_str(role: tskr_core::Role) -> String {
    match role {
        tskr_core::Role::User => "user".to_string(),
        tskr_core::Role::Assistant => "assistant".to_string(),
        tskr_core::Role::ToolResult => "tool_result".to_string(),
        tskr_core::Role::Summary => "summary".to_string(),
    }
}
