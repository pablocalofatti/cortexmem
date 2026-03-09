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
            Some(vec!["concept1".into()]),
            Some(vec!["fact1".into()]),
            None,
            None,
            None,
        )
        .unwrap()
        .id
}

#[test]
fn mem_search_should_return_compact_results() {
    let server = test_server();
    save_obs(&server, "Auth middleware design", "decision");
    save_obs(&server, "Database indexing strategy", "decision");
    save_obs(&server, "API rate limiting pattern", "pattern");

    let results = server.call_search("auth middleware", Some("testproject"), None, None, Some(10));
    assert!(!results.is_empty());
    assert!(results[0].title.contains("Auth"));
}

#[test]
fn mem_search_should_filter_by_type() {
    let server = test_server();
    save_obs(&server, "Auth decision made", "decision");
    save_obs(&server, "Auth pattern found", "pattern");

    let results = server.call_search("auth", Some("testproject"), Some("decision"), None, Some(10));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].obs_type, "decision");
}

#[test]
fn mem_get_should_return_full_observation() {
    let server = test_server();
    let id = save_obs(&server, "Full obs test", "decision");

    let obs = server.call_get(id).unwrap().unwrap();
    assert_eq!(obs.title, "Full obs test");
    assert_eq!(obs.content, "Content for Full obs test");
    assert_eq!(obs.obs_type, "decision");
}

#[test]
fn mem_get_multiple_should_return_all() {
    let server = test_server();
    let id1 = save_obs(&server, "Obs 1", "decision");
    let id2 = save_obs(&server, "Obs 2", "decision");
    save_obs(&server, "Obs 3", "decision");

    let results = server.call_get_multiple(&[id1, id2]).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn mem_get_should_increment_access_count() {
    let server = test_server();
    let id = save_obs(&server, "Access test", "decision");

    // Two gets should increment access_count
    server.call_get_and_track(id).unwrap();
    server.call_get_and_track(id).unwrap();

    let obs = server.call_get(id).unwrap().unwrap();
    assert_eq!(obs.access_count, 2);
}

#[test]
fn mem_timeline_should_show_surrounding_observations() {
    let server = test_server();
    let _id1 = save_obs(&server, "First obs", "decision");
    let _id2 = save_obs(&server, "Second obs", "decision");
    let id3 = save_obs(&server, "Third obs", "decision");
    let _id4 = save_obs(&server, "Fourth obs", "decision");
    let _id5 = save_obs(&server, "Fifth obs", "decision");

    let timeline = server.call_timeline(id3, Some(2), "testproject").unwrap();
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

    let suggestions = server.call_suggest_topic("testproject").unwrap();
    assert!(suggestions.contains(&"architecture/auth".to_string()));
    assert!(suggestions.contains(&"architecture/database".to_string()));
}
