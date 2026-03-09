use crate::db::Observation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierAction {
    Promote(&'static str),
    Archive,
    Unchanged,
}

/// Evaluate what action to take for an observation based on decay rules.
///
/// Rules:
/// - Buffer: access_count >= 2 → promote to Working
/// - Buffer: no access for 30 days → archive
/// - Working: access_count >= 5 OR revision_count >= 3 → promote to Core
/// - Working: no access for 90 days → archive
/// - Core: never archive, never demote
pub fn evaluate_tier(obs: &Observation) -> TierAction {
    let days_since_update = days_since(&obs.updated_at);

    match obs.tier.as_str() {
        "buffer" => {
            if obs.access_count >= 2 {
                TierAction::Promote("working")
            } else if days_since_update >= 30 {
                TierAction::Archive
            } else {
                TierAction::Unchanged
            }
        }
        "working" => {
            if obs.access_count >= 5 || obs.revision_count >= 3 {
                TierAction::Promote("core")
            } else if days_since_update >= 90 {
                TierAction::Archive
            } else {
                TierAction::Unchanged
            }
        }
        "core" => TierAction::Unchanged,
        _ => TierAction::Unchanged,
    }
}

fn days_since(datetime_str: &str) -> i64 {
    let Ok(dt) = chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") else {
        return 0;
    };

    let now = chrono::Utc::now().naive_utc();
    (now - dt).num_days().max(0)
}
