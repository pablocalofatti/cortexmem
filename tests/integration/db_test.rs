use cortexmem::db::Database;

#[test]
fn should_initialize_database_with_schema() {
    let db = Database::open_in_memory().unwrap();
    let version = db.schema_version().unwrap();
    assert_eq!(version, 3);
}

#[test]
fn should_use_wal_mode_for_file_backed_db() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let db = Database::open(&db_path).unwrap();
    let mode = db.journal_mode().unwrap();
    assert_eq!(mode, "wal");
}

#[test]
fn should_accept_memory_journal_mode_for_in_memory() {
    let db = Database::open_in_memory().unwrap();
    let mode = db.journal_mode().unwrap();
    assert_eq!(mode, "memory");
}

#[test]
fn should_register_sqlite_vec_extension() {
    let db = Database::open_in_memory().unwrap();
    let has_vec = db.has_vec_extension().unwrap();
    assert!(has_vec);
}
