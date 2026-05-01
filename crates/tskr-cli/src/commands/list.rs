use std::collections::HashMap;

use time::OffsetDateTime;

use crate::cli::ListArgs;
use crate::commands::search::parse_duration;
use crate::config::Config;
use crate::vector::{Client as VectorClient, SearchHit, SearchRequest};

pub async fn run(cfg: Config, args: ListArgs) -> anyhow::Result<()> {
    let limit = args.limit.unwrap_or(20);
    let k = std::cmp::max(limit.saturating_mul(50), 200);
    let cutoff = match args.since.as_deref() {
        Some(s) => Some(OffsetDateTime::now_utc() - parse_duration(s)?),
        None => None,
    };

    let mut eq_filters: Vec<(String, String)> = Vec::new();
    if let Some(repo) = args.repo.as_ref() {
        eq_filters.push(("repo".into(), repo.clone()));
    }
    if let Some(author) = args.author.as_ref() {
        eq_filters.push(("author".into(), author.clone()));
    }

    let vector_client = VectorClient::new(&cfg);
    let req = SearchRequest {
        vector: vec![0.0_f32; 384],
        k,
        eq_filters,
    };
    let mut hits = vector_client.search(req).await?;

    if let Some(cutoff) = cutoff {
        hits.retain(|h| match parse_rfc3339(&h.timestamp) {
            Some(ts) => ts >= cutoff,
            None => true,
        });
    }

    let mut groups: HashMap<String, SessionGroup> = HashMap::new();
    for hit in hits {
        let entry = groups
            .entry(hit.session_id.clone())
            .or_insert_with(|| SessionGroup::seed(&hit));
        entry.absorb(&hit);
    }

    let mut sessions: Vec<SessionGroup> = groups.into_values().collect();
    sessions.sort_by(|a, b| b.most_recent_ts.cmp(&a.most_recent_ts));
    if sessions.len() > limit {
        sessions.truncate(limit);
    }

    if sessions.is_empty() {
        println!("no sessions");
        return Ok(());
    }

    for s in sessions {
        let repo = s.repo.as_deref().unwrap_or("unknown");
        let head = truncate_chars(&one_line(&s.head_text), 80);
        println!(
            "{ts} {sid} [{author}/{repo}] events≥{count} — {head}",
            ts = s.earliest_ts,
            sid = s.session_id,
            author = s.author,
            repo = repo,
            count = s.max_event_index + 1,
            head = head,
        );
    }
    Ok(())
}

struct SessionGroup {
    session_id: String,
    author: String,
    repo: Option<String>,
    earliest_event_index: i64,
    max_event_index: i64,
    earliest_ts: String,
    most_recent_ts: String,
    head_text: String,
}

impl SessionGroup {
    fn seed(hit: &SearchHit) -> Self {
        Self {
            session_id: hit.session_id.clone(),
            author: hit.author.clone(),
            repo: hit.repo.clone(),
            earliest_event_index: hit.event_index,
            max_event_index: hit.event_index,
            earliest_ts: hit.timestamp.clone(),
            most_recent_ts: hit.timestamp.clone(),
            head_text: hit.text.clone(),
        }
    }

    fn absorb(&mut self, hit: &SearchHit) {
        if hit.event_index < self.earliest_event_index {
            self.earliest_event_index = hit.event_index;
            self.head_text = hit.text.clone();
        }
        if hit.event_index > self.max_event_index {
            self.max_event_index = hit.event_index;
        }
        if hit.timestamp < self.earliest_ts {
            self.earliest_ts = hit.timestamp.clone();
        }
        if hit.timestamp > self.most_recent_ts {
            self.most_recent_ts = hit.timestamp.clone();
        }
    }
}

fn parse_rfc3339(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
}

fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect()
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => s[..idx].to_string(),
        None => s.to_string(),
    }
}
