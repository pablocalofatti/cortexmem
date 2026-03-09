use cortexmem::db::{Database, NewObservation};

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

#[test]
fn should_index_observation_in_fts5() {
    let db = test_db();
    let id = db
        .insert_observation(&make_obs(
            "Authentication middleware",
            "JWT tokens for API auth",
            "decision",
        ))
        .unwrap();
    db.sync_observation_to_fts(id).unwrap();

    let results = db
        .search_fts("authentication", Some("myproject"), 10)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].rowid, id);
}

#[test]
fn should_find_by_partial_match() {
    let db = test_db();
    let id = db
        .insert_observation(&make_obs(
            "Authentication design",
            "Using OAuth2 for authentication flow",
            "decision",
        ))
        .unwrap();
    db.sync_observation_to_fts(id).unwrap();

    // FTS5 prefix query: "auth*" matches "authentication"
    let results = db.search_fts("auth*", Some("myproject"), 10).unwrap();
    assert!(!results.is_empty());
}

#[test]
fn should_rank_by_bm25() {
    let db = test_db();

    let id1 = db
        .insert_observation(&make_obs(
            "Database config",
            "PostgreSQL connection pooling setup",
            "discovery",
        ))
        .unwrap();
    db.sync_observation_to_fts(id1).unwrap();

    let id2 = db
        .insert_observation(&make_obs(
            "Auth middleware",
            "JWT authentication with refresh tokens for secure authentication",
            "decision",
        ))
        .unwrap();
    db.sync_observation_to_fts(id2).unwrap();

    let id3 = db
        .insert_observation(&make_obs(
            "API routes",
            "REST endpoint design patterns",
            "pattern",
        ))
        .unwrap();
    db.sync_observation_to_fts(id3).unwrap();

    let results = db
        .search_fts("authentication", Some("myproject"), 10)
        .unwrap();
    assert!(!results.is_empty());
    // The auth observation should rank first (most relevant)
    assert_eq!(results[0].rowid, id2);
}

#[test]
fn should_exclude_soft_deleted_from_fts() {
    let db = test_db();
    let id = db
        .insert_observation(&make_obs(
            "Deleted feature",
            "This feature was removed",
            "decision",
        ))
        .unwrap();
    db.sync_observation_to_fts(id).unwrap();

    // Verify it's found before deletion
    let results = db.search_fts("removed", Some("myproject"), 10).unwrap();
    assert_eq!(results.len(), 1);

    // Soft delete and remove from FTS
    db.soft_delete(id).unwrap();
    db.remove_from_fts(id).unwrap();

    let results = db.search_fts("removed", Some("myproject"), 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn should_filter_by_project() {
    let db = test_db();

    let mut obs1 = make_obs("Shared concept", "Authentication patterns", "pattern");
    obs1.project = "project-a".into();
    let id1 = db.insert_observation(&obs1).unwrap();
    db.sync_observation_to_fts(id1).unwrap();

    let mut obs2 = make_obs(
        "Other concept",
        "Authentication in other project",
        "pattern",
    );
    obs2.project = "project-b".into();
    let id2 = db.insert_observation(&obs2).unwrap();
    db.sync_observation_to_fts(id2).unwrap();

    let results = db
        .search_fts("authentication", Some("project-a"), 10)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].rowid, id1);
}
