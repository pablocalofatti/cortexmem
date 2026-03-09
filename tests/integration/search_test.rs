use cortexmem::db::{Database, NewObservation};
use cortexmem::search::{HybridSearcher, SearchParams, rrf_fuse};

fn test_db() -> Database {
    Database::open_in_memory().unwrap()
}

fn make_obs(title: &str, content: &str, obs_type: &str) -> NewObservation {
    NewObservation {
        project: "myproject".into(),
        title: title.into(),
        content: content.into(),
        obs_type: obs_type.into(),
        concepts: Some(vec![]),
        facts: Some(vec![]),
        files: None,
        topic_key: None,
        scope: "project".into(),
        session_id: None,
    }
}

fn insert_and_index(db: &Database, obs: &NewObservation) -> i64 {
    let id = db.insert_observation(obs).unwrap();
    db.sync_observation_to_fts(id).unwrap();
    id
}

#[test]
fn should_search_fts_only_when_no_model() {
    let db = test_db();
    insert_and_index(&db, &make_obs("Auth middleware", "JWT authentication tokens", "decision"));
    insert_and_index(&db, &make_obs("DB config", "PostgreSQL connection pooling", "discovery"));

    let searcher = HybridSearcher::new(&db, None);
    let params = SearchParams {
        query: "authentication".into(),
        project: Some("myproject".into()),
        obs_type: None,
        scope: None,
        limit: 10,
    };

    let results = searcher.search(&params).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Auth middleware");
}

#[test]
fn should_fuse_results_with_rrf() {
    // Unit test for RRF fusion
    let fts_ranks = vec![(1_i64, 0_usize), (2, 1), (3, 2)];
    let vec_ranks = vec![(3_i64, 0_usize), (1, 1), (4, 2)];
    let fused = rrf_fuse(&fts_ranks, &vec_ranks, 60);

    // id=1: 1/(60+0) + 1/(60+1) = 0.01667 + 0.01639 = 0.03306
    // id=3: 1/(60+2) + 1/(60+0) = 0.01613 + 0.01667 = 0.03279
    // id=1 should rank first (appears high in both)
    assert_eq!(fused[0].0, 1);
    assert_eq!(fused[1].0, 3);

    // id=2: only in fts at rank 1 → 1/(60+1) = 0.01639
    // id=4: only in vec at rank 2 → 1/(60+2) = 0.01613
    assert_eq!(fused[2].0, 2);
    assert_eq!(fused[3].0, 4);
}

#[test]
fn should_filter_by_project() {
    let db = test_db();

    let mut obs_a = make_obs("Auth in A", "JWT auth for project A", "decision");
    obs_a.project = "project-a".into();
    insert_and_index(&db, &obs_a);

    let mut obs_b = make_obs("Auth in B", "JWT auth for project B", "decision");
    obs_b.project = "project-b".into();
    insert_and_index(&db, &obs_b);

    let searcher = HybridSearcher::new(&db, None);
    let params = SearchParams {
        query: "JWT".into(),
        project: Some("project-a".into()),
        obs_type: None,
        scope: None,
        limit: 10,
    };

    let results = searcher.search(&params).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Auth in A");
}

#[test]
fn should_filter_by_type() {
    let db = test_db();
    insert_and_index(&db, &make_obs("Auth decision", "JWT tokens chosen", "decision"));
    insert_and_index(&db, &make_obs("Auth discovery", "Found OAuth library", "discovery"));

    let searcher = HybridSearcher::new(&db, None);
    let params = SearchParams {
        query: "auth*".into(),
        project: Some("myproject".into()),
        obs_type: Some("decision".into()),
        scope: None,
        limit: 10,
    };

    let results = searcher.search(&params).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].obs_type, "decision");
}

#[test]
fn should_respect_limit() {
    let db = test_db();
    for i in 0..15 {
        insert_and_index(
            &db,
            &make_obs(
                &format!("Auth pattern {i}"),
                &format!("Authentication pattern number {i}"),
                "pattern",
            ),
        );
    }

    let searcher = HybridSearcher::new(&db, None);
    let params = SearchParams {
        query: "authentication".into(),
        project: Some("myproject".into()),
        obs_type: None,
        scope: None,
        limit: 5,
    };

    let results = searcher.search(&params).unwrap();
    assert_eq!(results.len(), 5);
}
