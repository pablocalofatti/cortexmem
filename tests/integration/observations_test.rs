use cortexmem::db::{Database, NewObservation};

fn test_db() -> Database {
    Database::open_in_memory().unwrap()
}

fn sample_observation() -> NewObservation {
    NewObservation {
        project: "myproject".into(),
        title: "Auth decision".into(),
        content: "Chose JWT over sessions for stateless auth".into(),
        obs_type: "decision".into(),
        concepts: Some(vec!["auth".into(), "jwt".into()]),
        facts: Some(vec!["JWT chosen for stateless auth".into()]),
        files: Some(vec!["src/auth.ts".into()]),
        topic_key: Some("architecture/auth".into()),
        scope: "project".into(),
        session_id: None,
    }
}

#[test]
fn should_insert_observation() {
    let db = test_db();
    let id = db.insert_observation(&sample_observation()).unwrap();
    assert!(id > 0);
}

#[test]
fn should_get_observation_by_id() {
    let db = test_db();
    let obs = sample_observation();
    let id = db.insert_observation(&obs).unwrap();

    let fetched = db.get_observation(id).unwrap().unwrap();
    assert_eq!(fetched.title, "Auth decision");
    assert_eq!(
        fetched.content,
        "Chose JWT over sessions for stateless auth"
    );
    assert_eq!(fetched.obs_type, "decision");
    assert_eq!(fetched.project, "myproject");
    assert_eq!(fetched.concepts, Some(vec!["auth".into(), "jwt".into()]));
    assert_eq!(
        fetched.facts,
        Some(vec!["JWT chosen for stateless auth".into()])
    );
    assert_eq!(fetched.files, Some(vec!["src/auth.ts".into()]));
    assert_eq!(fetched.topic_key, Some("architecture/auth".into()));
    assert_eq!(fetched.scope, "project");
    assert_eq!(fetched.tier, "buffer");
    assert_eq!(fetched.access_count, 0);
    assert_eq!(fetched.revision_count, 1);
}

#[test]
fn should_find_by_topic_key() {
    let db = test_db();
    let obs = sample_observation();
    let id = db.insert_observation(&obs).unwrap();

    let found = db
        .find_by_topic_key("myproject", "architecture/auth")
        .unwrap()
        .unwrap();
    assert_eq!(found.id, id);
}

#[test]
fn should_upsert_on_topic_key_match() {
    let db = test_db();
    let obs = sample_observation();
    let id1 = db.upsert_observation(&obs).unwrap();

    let mut obs2 = sample_observation();
    obs2.content = "Updated: Using OAuth2 + JWT".into();
    let id2 = db.upsert_observation(&obs2).unwrap();

    assert_eq!(id1, id2);
    let fetched = db.get_observation(id1).unwrap().unwrap();
    assert_eq!(fetched.content, "Updated: Using OAuth2 + JWT");
    assert_eq!(fetched.revision_count, 2);
}

#[test]
fn should_soft_delete() {
    let db = test_db();
    let id = db.insert_observation(&sample_observation()).unwrap();
    db.soft_delete(id).unwrap();

    let all = db.list_observations("myproject", 100).unwrap();
    assert!(all.is_empty());

    // But get_observation still finds it (soft delete)
    let obs = db.get_observation(id).unwrap().unwrap();
    assert!(obs.deleted_at.is_some());
}

#[test]
fn should_increment_access_count() {
    let db = test_db();
    let id = db.insert_observation(&sample_observation()).unwrap();

    db.increment_access_count(id).unwrap();
    db.increment_access_count(id).unwrap();

    let obs = db.get_observation(id).unwrap().unwrap();
    assert_eq!(obs.access_count, 2);
}

#[test]
fn should_find_by_content_hash_within_window() {
    let db = test_db();
    let obs = sample_observation();
    let id = db.insert_observation(&obs).unwrap();

    let original = db.get_observation(id).unwrap().unwrap();
    let found = db.find_by_content_hash(&original.content_hash, 15).unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, id);
}

#[test]
fn should_list_observations_excluding_deleted() {
    let db = test_db();
    db.insert_observation(&sample_observation()).unwrap();

    let mut obs2 = sample_observation();
    obs2.topic_key = Some("architecture/db".into());
    obs2.title = "DB decision".into();
    let id2 = db.insert_observation(&obs2).unwrap();
    db.soft_delete(id2).unwrap();

    let all = db.list_observations("myproject", 100).unwrap();
    assert_eq!(all.len(), 1);
}
