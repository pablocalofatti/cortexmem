use cortexmem::db::Database;
use cortexmem::mcp::CortexMemServer;

fn test_server() -> CortexMemServer {
    let db = Database::open_in_memory().unwrap();
    CortexMemServer::new(db, None)
}

#[test]
fn suggest_topic_key_should_generate_family_prefix() {
    let server = test_server();
    server
        .call_save(
            "test-project",
            "JWT middleware design",
            "We chose JWT for auth",
            "architecture",
            None,
            None,
            None,
            Some("architecture/jwt-middleware".into()),
            None,
        )
        .unwrap();

    let result = server.call_suggest_topic("architecture", "Auth token validation");
    assert!(result.contains("architecture/auth-token-validation"));
    assert!(result.contains("architecture/jwt-middleware"));
}

#[test]
fn suggest_topic_key_should_handle_empty_db() {
    let server = test_server();
    let result = server.call_suggest_topic("decision", "Use SQLite for storage");
    assert!(result.contains("decision/use-sqlite-for"));
}

#[test]
fn suggest_topic_key_should_slugify_special_chars() {
    let server = test_server();
    let result = server.call_suggest_topic("bug_fix", "Fix #123: NULL pointer in auth");
    assert!(result.contains("bug/fix-123-null-pointer"));
}
