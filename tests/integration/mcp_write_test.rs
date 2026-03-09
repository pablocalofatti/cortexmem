use cortexmem::db::{Database, NewObservation};
use cortexmem::mcp::CortexMemServer;
use cortexmem::memory::DedupResult;

fn test_server() -> CortexMemServer {
    let db = Database::open_in_memory().unwrap();
    CortexMemServer::new(db, None)
}

fn sample_obs(title: &str) -> NewObservation {
    NewObservation {
        project: "testproject".into(),
        title: title.into(),
        content: format!("Content for {title}"),
        obs_type: "decision".into(),
        concepts: Some(vec!["auth".into()]),
        facts: Some(vec!["JWT chosen".into()]),
        files: None,
        topic_key: None,
        scope: "project".into(),
        session_id: None,
    }
}

#[test]
fn mem_save_should_return_id_and_dedup_status() {
    let server = test_server();
    let result = server.call_save(
        "testproject",
        "Auth decision",
        "Chose JWT over sessions",
        "decision",
        Some(vec!["auth".into()]),
        None,
        None,
        None,
        None,
    );
    assert!(result.is_ok());
    let save_result = result.unwrap();
    assert!(save_result.id > 0);
    assert!(matches!(save_result.dedup_status, DedupResult::NewContent));
}

#[test]
fn mem_save_should_dedup_hash_match() {
    let server = test_server();
    let r1 = server
        .call_save(
            "testproject",
            "Auth decision",
            "Chose JWT over sessions",
            "decision",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let r2 = server
        .call_save(
            "testproject",
            "Auth decision",
            "Chose JWT over sessions",
            "decision",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    assert!(matches!(r2.dedup_status, DedupResult::HashMatch(_)));
    assert_eq!(r2.id, r1.id);
}

#[test]
fn mem_save_should_upsert_topic_key() {
    let server = test_server();
    let r1 = server
        .call_save(
            "testproject",
            "Auth v1",
            "Original auth content",
            "decision",
            None,
            None,
            None,
            Some("architecture/auth".into()),
            None,
        )
        .unwrap();

    let r2 = server
        .call_save(
            "testproject",
            "Auth v2",
            "Updated auth content",
            "decision",
            None,
            None,
            None,
            Some("architecture/auth".into()),
            None,
        )
        .unwrap();

    assert!(matches!(r2.dedup_status, DedupResult::TopicKeyUpsert(_)));
    assert_eq!(r2.id, r1.id);

    // Verify content was updated
    let obs = server.call_get(r1.id).unwrap().unwrap();
    assert_eq!(obs.title, "Auth v2");
    assert_eq!(obs.content, "Updated auth content");
}

#[test]
fn mem_update_should_modify_fields() {
    let server = test_server();
    let result = server
        .call_save(
            "testproject",
            "Original title",
            "Original content",
            "decision",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    server
        .call_update(result.id, Some("Updated title"), None, None, None, None)
        .unwrap();

    let obs = server.call_get(result.id).unwrap().unwrap();
    assert_eq!(obs.title, "Updated title");
    assert_eq!(obs.content, "Original content"); // unchanged
    assert_eq!(obs.revision_count, 2); // starts at 1 (insert) + 1 (update)
}

#[test]
fn mem_update_should_recompute_hash() {
    let server = test_server();
    let result = server
        .call_save(
            "testproject",
            "Title",
            "Original content",
            "decision",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let obs_before = server.call_get(result.id).unwrap().unwrap();

    server
        .call_update(
            result.id,
            None,
            Some("New content entirely"),
            None,
            None,
            None,
        )
        .unwrap();

    let obs_after = server.call_get(result.id).unwrap().unwrap();
    assert_ne!(obs_before.content_hash, obs_after.content_hash);
    assert_eq!(obs_after.content, "New content entirely");
}

#[test]
fn mem_session_summary_should_persist() {
    let server = test_server();

    // Start a session first
    let session_id = server.call_session_start("testproject", "/tmp/test").unwrap();

    // Save summary
    server
        .call_session_summary(session_id, "Implemented auth module with JWT")
        .unwrap();

    // Verify it's stored
    let session = server.call_get_session(session_id).unwrap().unwrap();
    assert_eq!(
        session.summary.as_deref(),
        Some("Implemented auth module with JWT")
    );
}
