use cortexmem::db::Database;
use cortexmem::mcp::CortexMemServer;

#[test]
fn should_list_14_tools() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);
    let tools = server.list_tools();
    assert_eq!(tools.len(), 14, "Expected 14 MCP tools, got {}", tools.len());
}

#[test]
fn should_have_mem_save_tool() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);
    let tools = server.list_tools();
    let names: Vec<&str> = tools.iter().map(|t| &*t.name).collect();
    assert!(names.contains(&"mem_save"), "Missing mem_save tool");
}

#[test]
fn should_have_mem_search_tool() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);
    let tools = server.list_tools();
    let names: Vec<&str> = tools.iter().map(|t| &*t.name).collect();
    assert!(names.contains(&"mem_search"), "Missing mem_search tool");
}

#[test]
fn should_have_all_expected_tool_names() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);
    let tools = server.list_tools();
    let names: Vec<&str> = tools.iter().map(|t| &*t.name).collect();

    let expected = vec![
        "mem_save",
        "mem_update",
        "mem_session_summary",
        "mem_search",
        "mem_get",
        "mem_timeline",
        "mem_context",
        "mem_suggest_topic",
        "mem_session_start",
        "mem_session_end",
        "mem_delete",
        "mem_stats",
        "mem_compact",
        "mem_model",
    ];

    for name in &expected {
        assert!(names.contains(name), "Missing tool: {name}");
    }
}
