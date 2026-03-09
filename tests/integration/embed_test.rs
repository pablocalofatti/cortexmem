use cortexmem::embed::{EmbeddingManager, build_search_text};

#[test]
fn should_report_model_not_downloaded() {
    let manager = EmbeddingManager::new("$TMPDIR/cortexmem-test-models-nonexistent");
    assert!(!manager.is_model_available());
}

#[test]
fn should_build_search_text_from_observation() {
    let text = build_search_text(
        "Auth decision",
        "Chose JWT",
        &["auth", "jwt"],
        &["stateless"],
    );
    assert!(text.contains("Auth decision"));
    assert!(text.contains("Chose JWT"));
    assert!(text.contains("auth"));
    assert!(text.contains("jwt"));
    assert!(text.contains("stateless"));
}

#[test]
fn should_build_search_text_with_empty_arrays() {
    let text = build_search_text("Title", "Content", &[], &[]);
    assert!(text.contains("Title"));
    assert!(text.contains("Content"));
}

#[test]
fn should_accept_valid_model_name() {
    let model = cortexmem::embed::parse_model_name("AllMiniLML6V2");
    assert!(model.is_some());
}

#[test]
fn should_accept_bge_model_name() {
    let model = cortexmem::embed::parse_model_name("BGESmallENV15");
    assert!(model.is_some());
}

#[test]
fn should_reject_unknown_model_name() {
    let model = cortexmem::embed::parse_model_name("NonExistentModel");
    assert!(model.is_none());
}

// This test requires model download — run manually:
// cargo test --test embed_test -- --ignored
#[test]
#[ignore]
fn should_generate_embedding_with_correct_dimensions() {
    let manager =
        EmbeddingManager::new_with_download("$TMPDIR/cortexmem-test-models", "AllMiniLML6V2")
            .unwrap();
    let embedding = manager.embed("authentication middleware").unwrap();
    assert_eq!(embedding.len(), 384);
}
