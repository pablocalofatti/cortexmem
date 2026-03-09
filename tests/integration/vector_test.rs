use cortexmem::db::Database;

fn test_db() -> Database {
    Database::open_in_memory().unwrap()
}

#[test]
fn should_insert_and_query_vector() {
    let db = test_db();
    let embedding: Vec<f32> = vec![0.1; 384];
    db.insert_vector(1, &embedding).unwrap();

    let query: Vec<f32> = vec![0.1; 384];
    let results = db.search_vector(&query, 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].rowid, 1);
}

#[test]
fn should_return_results_ordered_by_distance() {
    let db = test_db();

    // Insert 3 vectors at different distances from the query
    let mut v1: Vec<f32> = vec![0.1; 384]; // close to query
    let mut v2: Vec<f32> = vec![0.5; 384]; // medium distance
    let mut v3: Vec<f32> = vec![0.9; 384]; // far from query

    // Make them slightly different
    v1[0] = 0.11;
    v2[0] = 0.55;
    v3[0] = 0.99;

    db.insert_vector(1, &v1).unwrap();
    db.insert_vector(2, &v2).unwrap();
    db.insert_vector(3, &v3).unwrap();

    let query: Vec<f32> = vec![0.1; 384];
    let results = db.search_vector(&query, 10).unwrap();

    assert_eq!(results.len(), 3);
    // Closest should be first
    assert_eq!(results[0].rowid, 1);
    // Distances should be ascending
    assert!(results[0].distance <= results[1].distance);
    assert!(results[1].distance <= results[2].distance);
}

#[test]
fn should_handle_empty_vector_table() {
    let db = test_db();
    let query: Vec<f32> = vec![0.1; 384];
    let results = db.search_vector(&query, 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn should_delete_and_reinsert_vector() {
    let db = test_db();

    let v1: Vec<f32> = vec![0.1; 384];
    db.insert_vector(1, &v1).unwrap();

    // Delete
    db.delete_vector(1).unwrap();

    // Reinsert with different vector
    let v2: Vec<f32> = vec![0.9; 384];
    db.insert_vector(1, &v2).unwrap();

    let query: Vec<f32> = vec![0.9; 384];
    let results = db.search_vector(&query, 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].rowid, 1);
    // Should be very close since we queried with the same vector
    assert!(results[0].distance < 0.01);
}

#[test]
fn should_respect_limit() {
    let db = test_db();

    for i in 1..=20 {
        let mut v: Vec<f32> = vec![0.1; 384];
        v[0] = i as f32 * 0.05;
        db.insert_vector(i, &v).unwrap();
    }

    let query: Vec<f32> = vec![0.1; 384];
    let results = db.search_vector(&query, 5).unwrap();
    assert_eq!(results.len(), 5);
}
