use cortexmem::cli::doctor::{CheckStatus, run_checks};
use cortexmem::db::Database;
use cortexmem::mcp::CortexMemServer;

#[test]
fn doctor_should_return_check_results() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);
    let results = run_checks(&server);

    assert!(!results.is_empty());
    assert!(
        results
            .iter()
            .any(|r| r.name == "Binary version" && r.passed())
    );
    assert!(results.iter().any(|r| r.name == "Database" && r.passed()));
    assert!(
        results
            .iter()
            .any(|r| r.name == "Schema version" && r.passed())
    );
}

#[test]
fn doctor_should_check_fts_and_vector_indices() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);
    let results = run_checks(&server);

    let fts = results.iter().find(|r| r.name == "FTS5 index").unwrap();
    assert_eq!(fts.status, CheckStatus::Ok);
    assert!(fts.detail.contains("0 entries"));

    let vec = results.iter().find(|r| r.name == "Vector index").unwrap();
    assert_eq!(vec.status, CheckStatus::Ok);
    assert!(vec.detail.contains("0 entries"));
}

#[test]
fn doctor_should_warn_for_missing_embedding_model() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);
    let results = run_checks(&server);

    let model = results
        .iter()
        .find(|r| r.name == "Embedding model")
        .unwrap();
    assert_eq!(model.status, CheckStatus::Warn);
}

#[test]
fn doctor_should_have_ten_checks() {
    let db = Database::open_in_memory().unwrap();
    let server = CortexMemServer::new(db, None);
    let results = run_checks(&server);
    assert_eq!(results.len(), 10);
}
