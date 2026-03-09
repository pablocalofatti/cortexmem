use anyhow::Result;

use crate::db::Database;

use super::decay::{TierAction, evaluate_tier};

#[derive(Debug, Clone, Default)]
pub struct CompactionStats {
    pub promoted: usize,
    pub archived: usize,
    pub unchanged: usize,
}

pub fn run_compaction(db: &Database, project: Option<&str>) -> Result<CompactionStats> {
    let observations = if let Some(proj) = project {
        db.list_observations(proj, 10_000)?
    } else {
        db.list_all_active_observations()?
    };

    let mut stats = CompactionStats::default();

    for obs in &observations {
        match evaluate_tier(obs) {
            TierAction::Promote(new_tier) => {
                db.update_tier(obs.id, new_tier)?;
                stats.promoted += 1;
            }
            TierAction::Archive => {
                db.soft_delete(obs.id)?;
                stats.archived += 1;
            }
            TierAction::Unchanged => {
                stats.unchanged += 1;
            }
        }
    }

    Ok(stats)
}
