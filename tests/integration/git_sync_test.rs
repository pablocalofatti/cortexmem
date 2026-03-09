use cortexmem::db::{Database, NewObservation};
use cortexmem::sync::git::{create_chunk, import_chunk};

#[test]
fn should_create_chunk_from_observations() {
    let db = Database::open_in_memory().unwrap();
    let obs = NewObservation {
        project: "proj".to_string(),
        title: "Test obs".to_string(),
        content: "Content".to_string(),
        obs_type: "decision".to_string(),
        concepts: None,
        facts: None,
        files: None,
        topic_key: None,
        scope: "project".to_string(),
        session_id: None,
    };
    db.insert_observation(&obs).unwrap();

    let chunk = create_chunk(&db, Some("proj")).unwrap();
    assert!(!chunk.chunk_id.is_empty());
    assert_eq!(chunk.project, "proj");
    assert_eq!(chunk.observations.len(), 1);
    assert_eq!(chunk.observations[0].title, "Test obs");
    assert_eq!(chunk.observations[0].content, "Content");
}

#[test]
fn should_create_chunk_with_sessions() {
    let db = Database::open_in_memory().unwrap();
    db.create_session("proj", "/tmp/dir").unwrap();

    let chunk = create_chunk(&db, Some("proj")).unwrap();
    assert_eq!(chunk.sessions.len(), 1);
    assert_eq!(chunk.sessions[0].project, "proj");
}

#[test]
fn should_create_empty_chunk_for_unknown_project() {
    let db = Database::open_in_memory().unwrap();
    let chunk = create_chunk(&db, Some("nonexistent")).unwrap();
    assert!(chunk.observations.is_empty());
    assert!(chunk.sessions.is_empty());
}

#[test]
fn should_import_chunk_with_dedup() {
    let db = Database::open_in_memory().unwrap();

    let chunk_json = serde_json::json!({
        "chunk_id": "test-chunk-001",
        "source": "test-host",
        "project": "proj",
        "exported_at": "2026-03-09T00:00:00+00:00",
        "observations": [{
            "id": 1,
            "session_id": null,
            "project": "proj",
            "topic_key": null,
            "type": "decision",
            "title": "Imported obs",
            "content": "Imported content",
            "concepts": null,
            "facts": null,
            "files": null,
            "scope": "project",
            "tier": "buffer",
            "access_count": 0,
            "revision_count": 1,
            "content_hash": "abc123unique",
            "created_at": "2026-03-09T00:00:00",
            "updated_at": "2026-03-09T00:00:00",
            "deleted_at": null
        }],
        "sessions": []
    })
    .to_string();

    let imported = import_chunk(&db, &chunk_json).unwrap();
    assert_eq!(imported, 1);

    // Duplicate import of same chunk should return 0
    let imported = import_chunk(&db, &chunk_json).unwrap();
    assert_eq!(imported, 0);
}

#[test]
fn should_dedup_observations_by_content_hash() {
    let db = Database::open_in_memory().unwrap();

    let chunk1_json = serde_json::json!({
        "chunk_id": "chunk-aaa",
        "source": "host-a",
        "project": "proj",
        "exported_at": "2026-03-09T00:00:00+00:00",
        "observations": [{
            "id": 1,
            "session_id": null,
            "project": "proj",
            "topic_key": null,
            "type": "decision",
            "title": "Obs A",
            "content": "Same content",
            "concepts": null,
            "facts": null,
            "files": null,
            "scope": "project",
            "tier": "buffer",
            "access_count": 0,
            "revision_count": 1,
            "content_hash": "shared-hash-999",
            "created_at": "2026-03-09T00:00:00",
            "updated_at": "2026-03-09T00:00:00",
            "deleted_at": null
        }],
        "sessions": []
    })
    .to_string();

    let chunk2_json = serde_json::json!({
        "chunk_id": "chunk-bbb",
        "source": "host-b",
        "project": "proj",
        "exported_at": "2026-03-09T01:00:00+00:00",
        "observations": [{
            "id": 2,
            "session_id": null,
            "project": "proj",
            "topic_key": null,
            "type": "decision",
            "title": "Obs B",
            "content": "Same content",
            "concepts": null,
            "facts": null,
            "files": null,
            "scope": "project",
            "tier": "buffer",
            "access_count": 0,
            "revision_count": 1,
            "content_hash": "shared-hash-999",
            "created_at": "2026-03-09T01:00:00",
            "updated_at": "2026-03-09T01:00:00",
            "deleted_at": null
        }],
        "sessions": []
    })
    .to_string();

    let imported1 = import_chunk(&db, &chunk1_json).unwrap();
    assert_eq!(imported1, 1);

    // Different chunk, same content_hash — observation should be deduped
    let imported2 = import_chunk(&db, &chunk2_json).unwrap();
    assert_eq!(imported2, 0);
}

#[test]
fn should_roundtrip_chunk_export_import() {
    let source_db = Database::open_in_memory().unwrap();
    let target_db = Database::open_in_memory().unwrap();

    let obs = NewObservation {
        project: "proj".to_string(),
        title: "Roundtrip test".to_string(),
        content: "Roundtrip content".to_string(),
        obs_type: "insight".to_string(),
        concepts: Some(vec!["rust".to_string(), "sync".to_string()]),
        facts: None,
        files: Some(vec!["src/main.rs".to_string()]),
        topic_key: Some("roundtrip-key".to_string()),
        scope: "project".to_string(),
        session_id: None,
    };
    source_db.insert_observation(&obs).unwrap();

    let chunk = create_chunk(&source_db, Some("proj")).unwrap();
    let json = serde_json::to_string(&chunk).unwrap();

    let imported = import_chunk(&target_db, &json).unwrap();
    assert_eq!(imported, 1);

    let target_obs = target_db
        .list_all_observations_for_export(Some("proj"))
        .unwrap();
    assert_eq!(target_obs.len(), 1);
    assert_eq!(target_obs[0].title, "Roundtrip test");
    assert_eq!(target_obs[0].content, "Roundtrip content");
    assert_eq!(
        target_obs[0].concepts,
        Some(vec!["rust".to_string(), "sync".to_string()])
    );
    assert_eq!(target_obs[0].files, Some(vec!["src/main.rs".to_string()]));
    assert_eq!(target_obs[0].topic_key.as_deref(), Some("roundtrip-key"));
}
