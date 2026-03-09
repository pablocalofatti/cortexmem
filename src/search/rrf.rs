use std::collections::HashMap;

/// Reciprocal Rank Fusion: combines two ranked lists into a single score.
///
/// Formula: score(d) = 1/(k + rank_a) + 1/(k + rank_b)
/// Items appearing in only one list get a single-term score.
/// Returns (id, score) pairs sorted by score descending.
pub fn rrf_fuse(
    fts_ranks: &[(i64, usize)],
    vec_ranks: &[(i64, usize)],
    k: usize,
) -> Vec<(i64, f64)> {
    let mut scores: HashMap<i64, f64> = HashMap::new();

    for &(id, rank) in fts_ranks {
        *scores.entry(id).or_default() += 1.0 / (k + rank) as f64;
    }

    for &(id, rank) in vec_ranks {
        *scores.entry(id).or_default() += 1.0 / (k + rank) as f64;
    }

    let mut results: Vec<(i64, f64)> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_lists_produce_empty_result() {
        let result = rrf_fuse(&[], &[], 60);
        assert!(result.is_empty());
    }

    #[test]
    fn single_list_produces_scores() {
        let fts = vec![(1, 0), (2, 1)];
        let result = rrf_fuse(&fts, &[], 60);
        assert_eq!(result.len(), 2);
        assert!(result[0].1 > result[1].1);
    }
}
