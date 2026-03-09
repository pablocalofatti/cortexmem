use cortexmem::db::Database;
use cortexmem::mcp::CortexMemServer;

fn test_server() -> CortexMemServer {
    let db = Database::open_in_memory().unwrap();
    CortexMemServer::new(db, None)
}

fn save_obs(server: &CortexMemServer, title: &str, obs_type: &str) -> i64 {
    server
        .call_save(
            "testproject",
            title,
            &format!("Content for {title}"),
            obs_type,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap()
        .id
}

#[test]
fn mem_session_start_should_create_session_and_return_id() {
    let server = test_server();
    let session_id = server.call_session_start("testproject", "/tmp/test").unwrap();
    assert!(session_id > 0);

    let session = server.call_get_session(session_id).unwrap().unwrap();
    assert_eq!(session.project, "testproject");
    assert_eq!(session.directory, "/tmp/test");
    assert!(session.ended_at.is_none());
}

#[test]
fn mem_session_end_should_close_session() {
    let server = test_server();
    let session_id = server.call_session_start("testproject", "/tmp/test").unwrap();
    server.call_session_end(session_id, Some("Great session")).unwrap();

    let session = server.call_get_session(session_id).unwrap().unwrap();
    assert!(session.ended_at.is_some());
    assert_eq!(session.summary.as_deref(), Some("Great session"));
}

#[test]
fn mem_delete_should_soft_delete() {
    let server = test_server();
    let id = save_obs(&server, "To delete", "decision");

    server.call_delete(id).unwrap();

    let obs = server.call_get(id).unwrap().unwrap();
    assert!(obs.deleted_at.is_some());

    // Should not appear in search
    let results = server.call_search("delete", Some("testproject"), None, None, Some(10));
    assert!(results.is_empty());
}

#[test]
fn mem_stats_should_return_counts() {
    let server = test_server();
    save_obs(&server, "Decision 1", "decision");
    save_obs(&server, "Decision 2", "decision");
    save_obs(&server, "Pattern 1", "pattern");

    let stats = server.call_stats(Some("testproject")).unwrap();
    assert_eq!(stats.total, 3);
    assert!(stats.by_type.iter().any(|(t, c)| t == "decision" && *c == 2));
    assert!(stats.by_type.iter().any(|(t, c)| t == "pattern" && *c == 1));
}

#[test]
fn mem_compact_should_return_stats() {
    let server = test_server();
    let id = save_obs(&server, "Old obs", "decision");

    // Backdate to trigger archive
    {
        let mgr = server.memory_lock();
        mgr.db().backdate_observation(id, 31).unwrap();
    }

    let stats = server.call_compact(Some("testproject")).unwrap();
    assert!(stats.archived > 0);
}
