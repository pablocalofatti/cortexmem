use cortexmem::db::{Database, NewObservation};

fn make_observation(title: &str) -> NewObservation {
    NewObservation {
        project: "test".to_string(),
        title: title.to_string(),
        content: format!("content for {title}"),
        obs_type: "note".to_string(),
        concepts: None,
        facts: None,
        files: None,
        topic_key: None,
        scope: "project".to_string(),
        session_id: None,
    }
}

#[test]
fn should_record_search_feedback() {
    let db = Database::open_in_memory().unwrap();
    let session_id = db.create_session("test", "/tmp").unwrap();
    let obs_id = db
        .insert_observation(&make_observation("auth note"))
        .unwrap();

    db.record_search_feedback("authentication", obs_id, Some(session_id))
        .unwrap();
    db.record_search_feedback("authentication", obs_id, Some(session_id))
        .unwrap();
    db.record_search_feedback("auth tokens", obs_id, Some(session_id))
        .unwrap();

    let count = db.get_feedback_count(obs_id).unwrap();
    assert_eq!(count, 3);
}

#[test]
fn should_return_zero_for_no_feedback() {
    let db = Database::open_in_memory().unwrap();
    let obs_id = db
        .insert_observation(&make_observation("unused note"))
        .unwrap();

    let count = db.get_feedback_count(obs_id).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn should_count_feedback_per_observation() {
    let db = Database::open_in_memory().unwrap();
    let obs1 = db.insert_observation(&make_observation("note 1")).unwrap();
    let obs2 = db.insert_observation(&make_observation("note 2")).unwrap();

    db.record_search_feedback("query1", obs1, None).unwrap();
    db.record_search_feedback("query2", obs1, None).unwrap();
    db.record_search_feedback("query3", obs2, None).unwrap();

    assert_eq!(db.get_feedback_count(obs1).unwrap(), 2);
    assert_eq!(db.get_feedback_count(obs2).unwrap(), 1);
}
