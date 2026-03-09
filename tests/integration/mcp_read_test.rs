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
fn mem_search_should_return_results() {
    let server = test_server();
    save_obs(&server, "Auth decision made", "decision");
    save_obs(&server, "Pattern observed", "pattern");

    let results = server.call_search("auth", Some("testproject"), None, None, Some(10));
    assert!(!results.is_empty());
}

#[test]
fn mem_search_should_filter_by_type() {
    let server = test_server();
    save_obs(&server, "Auth decision made", "decision");
    save_obs(&server, "Auth pattern found", "pattern");

    let results = server.call_search(
        "auth",
        Some("testproject"),
        Some("decision"),
        None,
        Some(10),
    );
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].obs_type, "decision");
}

#[test]
fn mem_get_should_return_full_observation() {
    let server = test_server();
    let id = save_obs(&server, "Full obs test", "decision");

    let result = server.call_get(id).unwrap();
    assert!(result.is_some());
    let obs = result.unwrap();
    assert_eq!(obs.title, "Full obs test");
}

#[test]
fn mem_get_multiple_should_return_all_requested() {
    let server = test_server();
    let id1 = save_obs(&server, "Obs 1", "decision");
    let id2 = save_obs(&server, "Obs 2", "pattern");

    let results = server.call_get_multiple(&[id1, id2]).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn mem_timeline_should_return_nearby_observations() {
    let server = test_server();
    save_obs(&server, "Before 1", "decision");
    save_obs(&server, "Before 2", "decision");
    let target_id = save_obs(&server, "Target", "decision");
    save_obs(&server, "After 1", "decision");
    save_obs(&server, "After 2", "decision");

    let timeline = server
        .call_timeline(target_id, Some(2), "testproject")
        .unwrap();
    assert!(timeline.len() >= 3); // at least the target + some neighbors
}

#[test]
fn mem_context_should_return_recent() {
    let server = test_server();
    save_obs(&server, "Context obs 1", "decision");
    save_obs(&server, "Context obs 2", "pattern");

    let results = server.call_context(Some("testproject"), 20).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn mem_context_should_include_prompts_when_present() {
    let server = test_server();
    save_obs(&server, "Context obs 1", "decision");
    server
        .call_save_prompt(None, "Fix the login bug", Some("testproject"))
        .unwrap();

    // call_context returns observations only — the enriched output is in the MCP handler
    // Test the protocol layer directly
    let prompts = server.call_recent_prompts(Some("testproject"), 10).unwrap();
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].content, "Fix the login bug");

    let formatted = cortexmem::mcp::protocol::format_prompts(&prompts);
    assert!(formatted.contains("Fix the login bug"));
    assert!(formatted.contains("Recent Prompts"));
}

#[test]
fn format_prompts_should_return_empty_string_for_no_prompts() {
    let formatted = cortexmem::mcp::protocol::format_prompts(&[]);
    assert!(formatted.is_empty());
}

#[test]
fn mem_suggest_topic_should_find_similar_keys() {
    let server = test_server();

    // Save with known topic_keys
    server
        .call_save(
            "testproject",
            "Auth decision",
            "JWT auth design",
            "decision",
            None,
            None,
            None,
            Some("architecture/auth".into()),
            None,
        )
        .unwrap();

    server
        .call_save(
            "testproject",
            "Database schema",
            "PostgreSQL schema design",
            "decision",
            None,
            None,
            None,
            Some("architecture/database".into()),
            None,
        )
        .unwrap();

    let result = server.call_suggest_topic("architecture", "Auth token service");
    assert!(result.contains("architecture/auth-token-service"));
    assert!(result.contains("architecture/auth"));
    assert!(result.contains("architecture/database"));
}
