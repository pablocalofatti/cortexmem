use cortexmem::db::Database;
use cortexmem::mcp::CortexMemServer;

fn make_server() -> CortexMemServer {
    let db = Database::open_in_memory().unwrap();
    CortexMemServer::new(db, None)
}

#[test]
fn should_save_and_retrieve_prompt_via_server() {
    let server = make_server();
    let id = server
        .call_save_prompt(None, "Implement the login feature", Some("myproject"))
        .unwrap();
    assert!(id > 0);

    let prompts = server.call_recent_prompts(Some("myproject"), 10).unwrap();
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].content, "Implement the login feature");
    assert_eq!(prompts[0].project, Some("myproject".to_string()));
}

#[test]
fn should_return_empty_when_no_prompts() {
    let server = make_server();
    let prompts = server
        .call_recent_prompts(Some("empty-project"), 10)
        .unwrap();
    assert!(prompts.is_empty());
}

#[test]
fn should_limit_recent_prompts_results() {
    let server = make_server();
    for i in 0..5 {
        server
            .call_save_prompt(None, &format!("Prompt {i}"), Some("proj"))
            .unwrap();
    }
    let prompts = server.call_recent_prompts(Some("proj"), 3).unwrap();
    assert_eq!(prompts.len(), 3);
}

#[test]
fn should_save_prompt_with_session_id() {
    let server = make_server();
    let session_id = server
        .call_session_start("proj", "/home/user/proj")
        .unwrap();
    let id = server
        .call_save_prompt(Some(session_id), "Fix the bug", Some("proj"))
        .unwrap();
    assert!(id > 0);

    let prompts = server.call_recent_prompts(Some("proj"), 10).unwrap();
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].session_id, Some(session_id));
}
