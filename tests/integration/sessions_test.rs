use cortexmem::db::Database;

fn test_db() -> Database {
    Database::open_in_memory().unwrap()
}

#[test]
fn should_create_session() {
    let db = test_db();
    let id = db
        .create_session("myproject", "/home/user/myproject")
        .unwrap();
    assert!(id > 0);
}

#[test]
fn should_end_session_with_summary() {
    let db = test_db();
    let id = db
        .create_session("myproject", "/home/user/myproject")
        .unwrap();
    db.end_session(id, Some("Implemented auth module")).unwrap();

    let session = db.get_session(id).unwrap().unwrap();
    assert!(session.ended_at.is_some());
    assert_eq!(session.summary, Some("Implemented auth module".into()));
}

#[test]
fn should_get_latest_session_for_project() {
    let db = test_db();
    db.create_session("myproject", "/home/user/myproject")
        .unwrap();
    let id2 = db
        .create_session("myproject", "/home/user/myproject")
        .unwrap();

    let latest = db.get_latest_session("myproject").unwrap().unwrap();
    assert_eq!(latest.id, id2);
}

#[test]
fn should_set_session_summary() {
    let db = test_db();
    let id = db
        .create_session("myproject", "/home/user/myproject")
        .unwrap();
    db.set_session_summary(id, "Working on auth").unwrap();

    let session = db.get_session(id).unwrap().unwrap();
    assert_eq!(session.summary, Some("Working on auth".into()));
    assert!(session.ended_at.is_none());
}
