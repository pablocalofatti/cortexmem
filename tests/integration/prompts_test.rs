use cortexmem::db::Database;

#[test]
fn should_insert_and_retrieve_prompt() {
    let db = Database::open_in_memory().unwrap();
    let id = db
        .insert_prompt(None, "Fix the login bug", Some("myproject"))
        .unwrap();
    assert!(id > 0);
    let prompts = db.get_recent_prompts(Some("myproject"), 10).unwrap();
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].content, "Fix the login bug");
    assert_eq!(prompts[0].project, Some("myproject".to_string()));
}

#[test]
fn should_search_prompts_via_fts() {
    let db = Database::open_in_memory().unwrap();
    db.insert_prompt(None, "Add authentication to API", Some("proj"))
        .unwrap();
    db.insert_prompt(None, "Fix database connection pool", Some("proj"))
        .unwrap();
    db.insert_prompt(None, "Unrelated task", Some("other"))
        .unwrap();
    let results = db
        .search_prompts("authentication", Some("proj"), 10)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].content.contains("authentication"));
}

#[test]
fn should_get_recent_prompts_ordered_by_date() {
    let db = Database::open_in_memory().unwrap();
    db.insert_prompt(None, "First prompt", Some("proj"))
        .unwrap();
    db.insert_prompt(None, "Second prompt", Some("proj"))
        .unwrap();
    db.insert_prompt(None, "Third prompt", Some("proj"))
        .unwrap();
    let prompts = db.get_recent_prompts(Some("proj"), 2).unwrap();
    assert_eq!(prompts.len(), 2);
    assert_eq!(prompts[0].content, "Third prompt");
}
