use crate::db::{Observation, Prompt};
use crate::memory::CompactionStats;

pub fn format_compact(observations: &[Observation]) -> String {
    if observations.is_empty() {
        return "No results found.".to_string();
    }

    let mut out = String::new();
    for obs in observations {
        out.push_str(&format!(
            "[{}] {} ({}){}\n",
            obs.id,
            obs.title,
            obs.obs_type,
            obs.concepts
                .as_ref()
                .map(|c| format!(" — {}", c.join(", ")))
                .unwrap_or_default(),
        ));
    }
    out
}

pub fn format_full(obs: &Observation) -> String {
    let mut out = format!(
        "# {} (id: {})\n\nType: {} | Tier: {} | Scope: {}\n",
        obs.title, obs.id, obs.obs_type, obs.tier, obs.scope,
    );

    if let Some(ref concepts) = obs.concepts
        && !concepts.is_empty()
    {
        out.push_str(&format!("Concepts: {}\n", concepts.join(", ")));
    }
    if let Some(ref facts) = obs.facts
        && !facts.is_empty()
    {
        out.push_str(&format!("Facts: {}\n", facts.join("; ")));
    }
    if let Some(ref files) = obs.files
        && !files.is_empty()
    {
        out.push_str(&format!("Files: {}\n", files.join(", ")));
    }
    if let Some(ref topic_key) = obs.topic_key {
        out.push_str(&format!("Topic: {topic_key}\n"));
    }

    out.push_str(&format!(
        "Accesses: {} | Revisions: {}\nCreated: {} | Updated: {}\n\n{}\n",
        obs.access_count, obs.revision_count, obs.created_at, obs.updated_at, obs.content,
    ));

    out
}

pub fn format_prompts(prompts: &[Prompt]) -> String {
    if prompts.is_empty() {
        return String::new();
    }
    let mut out = String::from("## Recent Prompts\n");
    for p in prompts {
        out.push_str(&format!("- [{}] {}\n", p.created_at, p.content));
    }
    out
}

pub fn format_stats(
    project: &str,
    total: usize,
    by_tier: &[(String, i64)],
    by_type: &[(String, i64)],
    model_status: &str,
) -> String {
    let mut out = format!("# Stats for {project}\n\nTotal observations: {total}\n\n");

    out.push_str("By tier:\n");
    for (tier, count) in by_tier {
        out.push_str(&format!("  {tier}: {count}\n"));
    }

    out.push_str("\nBy type:\n");
    for (obs_type, count) in by_type {
        out.push_str(&format!("  {obs_type}: {count}\n"));
    }

    out.push_str(&format!("\nModel: {model_status}\n"));
    out
}

pub fn format_compaction(stats: &CompactionStats) -> String {
    format!(
        "Compaction complete: {} promoted, {} archived, {} unchanged",
        stats.promoted, stats.archived, stats.unchanged,
    )
}
