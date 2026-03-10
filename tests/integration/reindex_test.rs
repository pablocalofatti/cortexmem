use cortexmem::db::{Database, NewObservation};
use cortexmem::memory::MemoryManager;

fn test_observation(title: &str) -> NewObservation {
    NewObservation {
        project: "test".into(),
        title: title.into(),
        content: format!("Content about {title}"),
        obs_type: "discovery".into(),
        concepts: None,
        facts: None,
        files: None,
        topic_key: None,
        scope: "project".into(),
        session_id: None,
    }
}

#[test]
fn should_delete_all_vectors() {
    let db = Database::open_in_memory().unwrap();
    db.insert_vector(1, &vec![0.0f32; 384]).unwrap();
    db.insert_vector(2, &vec![0.0f32; 384]).unwrap();
    assert_eq!(db.count_vector_entries().unwrap(), 2);

    db.delete_all_vectors().unwrap();
    assert_eq!(db.count_vector_entries().unwrap(), 0);
}

#[test]
fn should_list_all_active_observation_ids() {
    let db = Database::open_in_memory().unwrap();
    let mgr = MemoryManager::new(db, None);
    mgr.save_observation(&test_observation("first")).unwrap();
    mgr.save_observation(&test_observation("second")).unwrap();

    let ids = mgr.db().list_all_observation_ids().unwrap();
    assert_eq!(ids.len(), 2);
}
