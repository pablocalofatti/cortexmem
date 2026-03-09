mod rrf;

pub use rrf::rrf_fuse;

use anyhow::Result;
use serde::Serialize;

use crate::db::Database;
use crate::embed::EmbeddingManager;

const RRF_K: usize = 60;
const FTS_FETCH_LIMIT: i64 = 50;
const VEC_FETCH_LIMIT: i64 = 50;

pub struct SearchParams {
    pub query: String,
    pub project: Option<String>,
    pub obs_type: Option<String>,
    pub scope: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub id: i64,
    pub title: String,
    pub obs_type: String,
    pub concepts: Option<Vec<String>>,
    pub created_at: String,
    pub score: f64,
}

pub struct HybridSearcher<'a> {
    db: &'a Database,
    embed_mgr: Option<&'a EmbeddingManager>,
}

impl<'a> HybridSearcher<'a> {
    pub fn new(db: &'a Database, embed_mgr: Option<&'a EmbeddingManager>) -> Self {
        Self { db, embed_mgr }
    }

    pub fn search(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        // Step 1: FTS5 search
        let fts_results =
            self.db
                .search_fts(&params.query, params.project.as_deref(), FTS_FETCH_LIMIT)?;

        let fts_ranks: Vec<(i64, usize)> = fts_results
            .iter()
            .enumerate()
            .map(|(rank, r)| (r.rowid, rank))
            .collect();

        // Step 2: Vector KNN search (auto-download model on first use)
        let vec_ranks: Vec<(i64, usize)> = if let Some(mgr) = self.embed_mgr {
            if !mgr.is_model_available() {
                tracing::info!("Embedding model not loaded — downloading on first use...");
                let _ = mgr.download_model();
            }
            if mgr.is_model_available() {
                if let Ok(query_embedding) = mgr.embed(&params.query) {
                    let vec_results = self.db.search_vector(&query_embedding, VEC_FETCH_LIMIT)?;
                    vec_results
                        .iter()
                        .enumerate()
                        .map(|(rank, r)| (r.rowid, rank))
                        .collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        // Step 3: RRF fusion
        let fused = rrf_fuse(&fts_ranks, &vec_ranks, RRF_K);

        // Step 4: Fetch observation details, apply filters, boost, and limit
        let mut results = Vec::new();
        for (id, rrf_score) in &fused {
            if results.len() >= params.limit {
                break;
            }

            let Some(obs) = self.db.get_observation(*id)? else {
                continue;
            };

            // Skip soft-deleted
            if obs.deleted_at.is_some() {
                continue;
            }

            // Filter by type
            if let Some(ref filter_type) = params.obs_type
                && obs.obs_type != *filter_type
            {
                continue;
            }

            // Filter by scope
            if let Some(ref filter_scope) = params.scope
                && obs.scope != *filter_scope
            {
                continue;
            }

            // Filter by project (double-check since FTS may not perfectly filter)
            if let Some(ref filter_project) = params.project
                && obs.project != *filter_project
            {
                continue;
            }

            // Boost by recency, access count, and search feedback
            let feedback_count = self.db.get_feedback_count(obs.id).unwrap_or_else(|e| {
                tracing::warn!("Failed to get feedback count for obs {}: {e}", obs.id);
                0
            });
            let final_score = apply_boosts(
                *rrf_score,
                &obs.updated_at,
                obs.access_count,
                feedback_count,
            );

            results.push(SearchResult {
                id: obs.id,
                title: obs.title,
                obs_type: obs.obs_type,
                concepts: obs.concepts,
                created_at: obs.created_at,
                score: final_score,
            });
        }

        // Re-sort by final score after boosting
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }
}

const ACCESS_BOOST_PER_HIT: f64 = 0.1;
const ACCESS_BOOST_CAP: f64 = 2.0;
const FEEDBACK_BOOST_PER_HIT: f64 = 0.1;
const FEEDBACK_BOOST_CAP: f64 = 2.0;
const RECENCY_DECAY_RATE: f64 = 0.01;

fn apply_boosts(rrf_score: f64, updated_at: &str, access_count: i64, feedback_count: i64) -> f64 {
    let recency_factor = compute_recency_factor(updated_at);
    let access_factor = (1.0 + ACCESS_BOOST_PER_HIT * access_count as f64).min(ACCESS_BOOST_CAP);
    let feedback_factor =
        (1.0 + FEEDBACK_BOOST_PER_HIT * feedback_count as f64).min(FEEDBACK_BOOST_CAP);
    rrf_score * recency_factor * access_factor * feedback_factor
}

fn compute_recency_factor(updated_at: &str) -> f64 {
    let Ok(updated) = chrono::NaiveDateTime::parse_from_str(updated_at, "%Y-%m-%d %H:%M:%S") else {
        return 1.0;
    };

    let now = chrono::Utc::now().naive_utc();
    let days_since = (now - updated).num_days().max(0) as f64;
    1.0 / (1.0 + days_since * RECENCY_DECAY_RATE)
}
