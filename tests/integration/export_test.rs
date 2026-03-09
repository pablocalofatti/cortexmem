use cortexmem::db::Database;
use cortexmem::mcp::CortexMemServer;

fn test_server() -> CortexMemServer {
    let db = Database::open_in_memory().unwrap();
    CortexMemServer::new(db, None)
}

#[test]
fn export_import_round_trip_should_preserve_observations() {
    let server = test_server();

    server
        .call_save(
            "proj",
            "Decision A",
            "Content A",
            "decision",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    server
        .call_save(
            "proj",
            "Bug B",
            "Content B",
            "bug_fix",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    // Export
    let mgr = server.memory_lock();
    let db = mgr.db();
    let observations = db.list_all_observations_for_export(None).unwrap();
    assert_eq!(observations.len(), 2);
    drop(mgr);

    // Create a new server (fresh DB) and import
    let server2 = test_server();
    let mgr2 = server2.memory_lock();
    let db2 = mgr2.db();

    for obs in &observations {
        db2.import_observation(obs).unwrap();
    }

    let imported = db2.list_all_observations_for_export(None).unwrap();
    assert_eq!(imported.len(), 2);
    assert_eq!(imported[0].title, "Decision A");
    assert_eq!(imported[1].title, "Bug B");
}

#[test]
fn import_merge_should_skip_duplicates_by_hash() {
    let server = test_server();

    server
        .call_save(
            "proj",
            "Decision A",
            "Same content",
            "decision",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let mgr = server.memory_lock();
    let db = mgr.db();
    let observations = db.list_all_observations_for_export(None).unwrap();

    // Import same data again — should skip (duplicate hash)
    for obs in &observations {
        db.import_observation(obs).unwrap();
    }

    let all = db.list_all_observations_for_export(None).unwrap();
    assert_eq!(all.len(), 1);
}
