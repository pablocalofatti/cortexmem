use cortexmem::db::{Database, NewObservation};
use cortexmem::memory::{DedupResult, MemoryManager};

fn test_db() -> Database {
    Database::open_in_memory().unwrap()
}

fn sample_obs() -> NewObservation {
    NewObservation {
        project: "myproject".into(),
        title: "Auth decision".into(),
        content: "Chose JWT over sessions for stateless auth".into(),
        obs_type: "decision".into(),
        concepts: Some(vec!["auth".into(), "jwt".into()]),
        facts: Some(vec!["JWT chosen for stateless auth".into()]),
        files: Some(vec!["src/auth.ts".into()]),
        topic_key: None,
        scope: "project".into(),
        session_id: None,
    }
}

#[test]
fn should_detect_hash_duplicate_within_window() {
    let db = test_db();
    let mgr = MemoryManager::new(db, None);

    let obs = sample_obs();
    let result1 = mgr.save_observation(&obs).unwrap();
    assert!(matches!(result1.dedup_status, DedupResult::NewContent));

    let result2 = mgr.save_observation(&obs).unwrap();
    assert!(matches!(result2.dedup_status, DedupResult::HashMatch(_)));
}

#[test]
fn should_upsert_on_topic_key_match() {
    let db = test_db();
    let mgr = MemoryManager::new(db, None);

    let mut obs = sample_obs();
    obs.topic_key = Some("architecture/auth".into());
    let result1 = mgr.save_observation(&obs).unwrap();
    assert!(matches!(result1.dedup_status, DedupResult::NewContent));

    obs.content = "Updated: Using OAuth2 + JWT".into();
    let result2 = mgr.save_observation(&obs).unwrap();
    assert!(matches!(
        result2.dedup_status,
        DedupResult::TopicKeyUpsert(_)
    ));
    assert_eq!(result1.id, result2.id);
}

#[test]
fn should_save_new_content_with_different_hash() {
    let db = test_db();
    let mgr = MemoryManager::new(db, None);

    let obs1 = sample_obs();
    let result1 = mgr.save_observation(&obs1).unwrap();

    let mut obs2 = sample_obs();
    obs2.content = "Completely different content about databases".into();
    obs2.title = "DB decision".into();
    let result2 = mgr.save_observation(&obs2).unwrap();

    assert!(matches!(result1.dedup_status, DedupResult::NewContent));
    assert!(matches!(result2.dedup_status, DedupResult::NewContent));
    assert_ne!(result1.id, result2.id);
}
