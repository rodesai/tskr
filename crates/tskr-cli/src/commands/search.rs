use anyhow::Context;
use time::OffsetDateTime;

use crate::cli::SearchArgs;
use crate::config::Config;
use crate::embed::Client as EmbedClient;
use crate::vector::{Client as VectorClient, SearchHit, SearchRequest};

pub async fn run(cfg: Config, args: SearchArgs) -> anyhow::Result<()> {
    let limit = args.limit.unwrap_or(10);
    let cutoff = match args.since.as_deref() {
        Some(s) => Some(OffsetDateTime::now_utc() - parse_duration(s)?),
        None => None,
    };

    let embed = EmbedClient::new(&cfg);
    let vectors = embed.embed(std::slice::from_ref(&args.query)).await?;
    let vector = vectors
        .into_iter()
        .next()
        .context("embedding server returned no embeddings")?;

    let mut eq_filters: Vec<(String, String)> = Vec::new();
    if let Some(repo) = args.repo.as_ref() {
        eq_filters.push(("repo".into(), repo.clone()));
    }
    if let Some(author) = args.author.as_ref() {
        eq_filters.push(("author".into(), author.clone()));
    }

    let vector_client = VectorClient::new(&cfg);
    let req = SearchRequest {
        vector,
        k: limit,
        eq_filters,
    };
    let mut hits = vector_client.search(req).await?;

    if let Some(cutoff) = cutoff {
        hits.retain(|h| match parse_rfc3339(&h.timestamp) {
            Some(ts) => ts >= cutoff,
            None => true,
        });
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if hits.len() > limit {
        hits.truncate(limit);
    }

    if hits.is_empty() {
        println!("no results");
        return Ok(());
    }

    for hit in hits {
        println!("{}", format_hit(&hit));
    }
    Ok(())
}

fn format_hit(hit: &SearchHit) -> String {
    let snippet = snippet(&hit.text, 100);
    let repo = hit.repo.as_deref().unwrap_or("unknown");
    format!(
        "[{author}/{repo}] {ts} \"{snippet}...\" (score={score:.3}) — session={session} event={event}",
        author = hit.author,
        repo = repo,
        ts = hit.timestamp,
        snippet = snippet,
        score = hit.score,
        session = hit.session_id,
        event = hit.event_index,
    )
}

fn snippet(text: &str, max_chars: usize) -> String {
    let oneline: String = text
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    truncate_chars(&oneline, max_chars)
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => s[..idx].to_string(),
        None => s.to_string(),
    }
}

fn parse_rfc3339(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
}

pub(crate) fn parse_duration(s: &str) -> anyhow::Result<time::Duration> {
    if s.is_empty() {
        anyhow::bail!("--since: empty");
    }
    let (num, unit) = s.split_at(s.len() - 1);
    let n: i64 = num.parse().context("--since: leading number")?;
    match unit {
        "s" => Ok(time::Duration::seconds(n)),
        "m" => Ok(time::Duration::minutes(n)),
        "h" => Ok(time::Duration::hours(n)),
        "d" => Ok(time::Duration::days(n)),
        other => anyhow::bail!("--since: unknown unit {other:?} (expected s/m/h/d)"),
    }
}
