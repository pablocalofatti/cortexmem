#[test]
fn should_extract_keywords_from_text() {
    let text = "Implemented JWT authentication with refresh tokens for the REST API. \
                The authentication middleware validates Bearer tokens on every request. \
                Tokens expire after 24 hours and can be refreshed using the refresh endpoint.";
    let keywords = cortexmem::memory::autotag::extract_keywords(text, 5);

    assert!(!keywords.is_empty());
    assert!(keywords.len() <= 5);
    // Should find domain-relevant terms, not stop words
    assert!(
        keywords
            .iter()
            .any(|k| k.contains("token") || k.contains("authent") || k.contains("jwt"))
    );
    // Should NOT include stop words
    assert!(
        !keywords
            .iter()
            .any(|k| k == "the" || k == "and" || k == "for" || k == "with")
    );
}

#[test]
fn should_handle_empty_text() {
    let keywords = cortexmem::memory::autotag::extract_keywords("", 5);
    assert!(keywords.is_empty());
}

#[test]
fn should_handle_short_text() {
    let keywords = cortexmem::memory::autotag::extract_keywords("hello world", 5);
    assert!(keywords.len() <= 2);
}

#[test]
fn should_extract_facts_from_declarative_sentences() {
    let text = "The database uses WAL mode for concurrent reads. \
                Authentication tokens expire after 24 hours. \
                We chose PostgreSQL over MySQL for JSON support.";
    let facts = cortexmem::memory::autotag::extract_facts(text, 3);

    assert!(!facts.is_empty());
    assert!(facts.len() <= 3);
    // Each fact should be a complete sentence
    for fact in &facts {
        assert!(fact.contains('.') || fact.len() > 10);
    }
}

use cortexmem::db::{Database, NewObservation};
use cortexmem::memory::MemoryManager;

#[test]
fn should_auto_tag_concepts_when_not_provided() {
    let db = Database::open_in_memory().unwrap();
    let mgr = MemoryManager::new(db, None);

    let obs = NewObservation {
        project: "test".into(),
        title: "JWT Authentication Decision".into(),
        content: "Chose JWT over session cookies for the REST API authentication. \
                  JWT tokens are stateless and work well with microservices architecture. \
                  Refresh tokens stored in httpOnly cookies for security."
            .into(),
        obs_type: "decision".into(),
        concepts: None,
        facts: None,
        files: None,
        topic_key: None,
        scope: "project".into(),
        session_id: None,
    };

    let result = mgr.save_observation(&obs).unwrap();
    let saved = mgr.db().get_observation(result.id).unwrap().unwrap();

    // Should have auto-generated concepts
    assert!(saved.concepts.is_some());
    let concepts = saved.concepts.unwrap();
    assert!(!concepts.is_empty());
}

#[test]
fn should_not_overwrite_provided_concepts() {
    let db = Database::open_in_memory().unwrap();
    let mgr = MemoryManager::new(db, None);

    let obs = NewObservation {
        project: "test".into(),
        title: "Manual Concepts".into(),
        content: "Some content about databases and queries".into(),
        obs_type: "discovery".into(),
        concepts: Some(vec!["my-concept".into()]),
        facts: None,
        files: None,
        topic_key: None,
        scope: "project".into(),
        session_id: None,
    };

    let result = mgr.save_observation(&obs).unwrap();
    let saved = mgr.db().get_observation(result.id).unwrap().unwrap();

    let concepts = saved.concepts.unwrap();
    assert_eq!(concepts, vec!["my-concept"]);
}
