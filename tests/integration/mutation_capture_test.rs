use cortexmem::db::Database;
use cortexmem::mcp::CortexMemServer;

#[test]
fn should_capture_mutation_on_save() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);

    server
        .call_save(
            "testproj",
            "Test obs",
            "Content here",
            "decision",
            None,
            None,
            None,
            None,
            Some("project".to_string()),
        )
        .unwrap();

    let mgr = server.memory_lock();
    let mutations = mgr.db().list_unacked_mutations(100).unwrap();
    assert_eq!(mutations.len(), 1);
    assert_eq!(mutations[0].entity, "observation");
    assert_eq!(mutations[0].op, "insert");
    assert_eq!(mutations[0].project, "testproj");
}

#[test]
fn should_capture_mutation_on_delete() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);

    let result = server
        .call_save(
            "proj",
            "To delete",
            "Content",
            "discovery",
            None,
            None,
            None,
            None,
            Some("project".to_string()),
        )
        .unwrap();

    server.call_delete(result.id).unwrap();

    let mgr = server.memory_lock();
    let mutations = mgr.db().list_unacked_mutations(100).unwrap();
    assert_eq!(mutations.len(), 2);
    assert_eq!(mutations[1].op, "soft_delete");
}

#[test]
fn should_capture_mutation_on_update() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);

    let result = server
        .call_save(
            "proj",
            "Original",
            "Content",
            "pattern",
            None,
            None,
            None,
            None,
            Some("project".to_string()),
        )
        .unwrap();

    server
        .call_update(result.id, Some("Updated title"), None, None, None, None)
        .unwrap();

    let mgr = server.memory_lock();
    let mutations = mgr.db().list_unacked_mutations(100).unwrap();
    assert_eq!(mutations.len(), 2);
    assert_eq!(mutations[1].op, "update");
}

#[test]
fn should_not_capture_mutation_on_hash_duplicate() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);

    // Save same content twice
    server
        .call_save(
            "proj",
            "Same",
            "Same content",
            "decision",
            None,
            None,
            None,
            None,
            Some("project".to_string()),
        )
        .unwrap();
    server
        .call_save(
            "proj",
            "Same",
            "Same content",
            "decision",
            None,
            None,
            None,
            None,
            Some("project".to_string()),
        )
        .unwrap();

    let mgr = server.memory_lock();
    let mutations = mgr.db().list_unacked_mutations(100).unwrap();
    // Only 1 mutation, not 2 — hash match skips
    assert_eq!(mutations.len(), 1);
}
