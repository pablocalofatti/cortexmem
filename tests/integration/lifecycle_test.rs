use cortexmem::db::{Database, NewObservation};
use cortexmem::memory::{run_compaction, MemoryManager};

fn test_db() -> Database {
    Database::open_in_memory().unwrap()
}

fn sample_obs(title: &str) -> NewObservation {
    NewObservation {
        project: "myproject".into(),
        title: title.into(),
        content: format!("Content for {title}"),
        obs_type: "decision".into(),
        concepts: Some(vec![]),
        facts: Some(vec![]),
        files: None,
        topic_key: None,
        scope: "project".into(),
        session_id: None,
    }
}

#[test]
fn should_promote_buffer_to_working_on_access() {
    let db = test_db();
    let mgr = MemoryManager::new(db, None);

    let result = mgr.save_observation(&sample_obs("Test obs")).unwrap();
    let obs = mgr.db().get_observation(result.id).unwrap().unwrap();
    assert_eq!(obs.tier, "buffer");

    // Access twice
    mgr.db().increment_access_count(result.id).unwrap();
    mgr.db().increment_access_count(result.id).unwrap();

    let stats = run_compaction(mgr.db(), Some("myproject")).unwrap();
    assert!(stats.promoted > 0);

    let obs = mgr.db().get_observation(result.id).unwrap().unwrap();
    assert_eq!(obs.tier, "working");
}

#[test]
fn should_promote_working_to_core_on_5_accesses() {
    let db = test_db();
    let mgr = MemoryManager::new(db, None);

    let result = mgr.save_observation(&sample_obs("Core obs")).unwrap();

    for _ in 0..5 {
        mgr.db().increment_access_count(result.id).unwrap();
    }

    // First compaction: buffer -> working
    run_compaction(mgr.db(), Some("myproject")).unwrap();
    // Second compaction: working -> core
    run_compaction(mgr.db(), Some("myproject")).unwrap();

    let obs = mgr.db().get_observation(result.id).unwrap().unwrap();
    assert_eq!(obs.tier, "core");
}

#[test]
fn should_archive_stale_buffer() {
    let db = test_db();
    let mgr = MemoryManager::new(db, None);

    let result = mgr.save_observation(&sample_obs("Old obs")).unwrap();
    mgr.db().backdate_observation(result.id, 31).unwrap();

    let stats = run_compaction(mgr.db(), Some("myproject")).unwrap();
    assert!(stats.archived > 0);

    let obs = mgr.db().get_observation(result.id).unwrap().unwrap();
    assert!(obs.deleted_at.is_some());
}

#[test]
fn should_never_archive_core() {
    let db = test_db();
    let mgr = MemoryManager::new(db, None);

    let result = mgr.save_observation(&sample_obs("Core obs")).unwrap();
    mgr.db().update_tier(result.id, "core").unwrap();
    mgr.db().backdate_observation(result.id, 365).unwrap();

    let stats = run_compaction(mgr.db(), Some("myproject")).unwrap();
    assert_eq!(stats.archived, 0);

    let obs = mgr.db().get_observation(result.id).unwrap().unwrap();
    assert!(obs.deleted_at.is_none());
}

#[test]
fn should_return_compaction_stats() {
    let db = test_db();
    let mgr = MemoryManager::new(db, None);

    let _r1 = mgr.save_observation(&sample_obs("Fresh")).unwrap();
    let r2 = mgr.save_observation(&sample_obs("Accessed")).unwrap();
    let r3 = mgr.save_observation(&sample_obs("Old")).unwrap();

    // Make r2 accessed (will promote)
    mgr.db().increment_access_count(r2.id).unwrap();
    mgr.db().increment_access_count(r2.id).unwrap();

    // Make r3 old (will archive)
    mgr.db().backdate_observation(r3.id, 31).unwrap();

    let stats = run_compaction(mgr.db(), Some("myproject")).unwrap();
    assert_eq!(stats.promoted, 1);
    assert_eq!(stats.archived, 1);
    assert_eq!(stats.unchanged, 1);
}
