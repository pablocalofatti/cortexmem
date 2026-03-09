use cortexmem::db::Database;

#[test]
fn should_insert_and_list_sync_mutations() {
    let db = Database::open_in_memory().unwrap();

    let seq = db
        .insert_sync_mutation(
            "observation",
            "obs-1",
            "insert",
            r#"{"title":"test"}"#,
            "myproject",
        )
        .unwrap();
    assert!(seq > 0);

    let unacked = db.list_unacked_mutations(100).unwrap();
    assert_eq!(unacked.len(), 1);
    assert_eq!(unacked[0].seq, seq);
    assert_eq!(unacked[0].entity, "observation");
    assert_eq!(unacked[0].entity_key, "obs-1");
    assert_eq!(unacked[0].op, "insert");
    assert_eq!(unacked[0].project, "myproject");
    assert!(unacked[0].acked_at.is_none());
}

#[test]
fn should_ack_mutations() {
    let db = Database::open_in_memory().unwrap();

    let seq1 = db
        .insert_sync_mutation("observation", "obs-1", "insert", "{}", "proj")
        .unwrap();
    let _seq2 = db
        .insert_sync_mutation("observation", "obs-2", "insert", "{}", "proj")
        .unwrap();

    db.ack_mutations(seq1).unwrap();

    let unacked = db.list_unacked_mutations(100).unwrap();
    assert_eq!(unacked.len(), 1);
    assert_eq!(unacked[0].entity_key, "obs-2");
}

#[test]
fn should_track_sync_state() {
    let db = Database::open_in_memory().unwrap();

    // Initially no state
    let state = db.get_sync_state("cloud").unwrap();
    assert!(state.is_none());

    // Insert state
    db.update_sync_state("cloud", 10, 5, None).unwrap();
    let state = db.get_sync_state("cloud").unwrap().unwrap();
    assert_eq!(state.target_key, "cloud");
    assert_eq!(state.last_pushed_seq, 10);
    assert_eq!(state.last_pulled_seq, 5);
    assert!(state.last_error.is_none());

    // Update state (upsert)
    db.update_sync_state("cloud", 20, 15, Some("timeout"))
        .unwrap();
    let state = db.get_sync_state("cloud").unwrap().unwrap();
    assert_eq!(state.last_pushed_seq, 20);
    assert_eq!(state.last_pulled_seq, 15);
    assert_eq!(state.last_error.as_deref(), Some("timeout"));
}

#[test]
fn should_track_sync_chunks() {
    let db = Database::open_in_memory().unwrap();

    let is_new = db.record_sync_chunk("chunk-abc-123").unwrap();
    assert!(is_new, "first insert should be new");

    let is_new = db.record_sync_chunk("chunk-abc-123").unwrap();
    assert!(!is_new, "second insert should be duplicate");
}
