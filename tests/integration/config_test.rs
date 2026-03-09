use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn should_parse_default_config_when_no_file_exists() {
    let config = cortexmem::config::Config::load_from_path(None);
    assert_eq!(config.embedding.model, "AllMiniLML6V2");
}

#[test]
fn should_parse_config_from_toml_file() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "[embedding]").unwrap();
    writeln!(f, "model = \"BGESmallENV15\"").unwrap();
    let config = cortexmem::config::Config::load_from_path(Some(f.path()));
    assert_eq!(config.embedding.model, "BGESmallENV15");
}

#[test]
fn should_use_default_for_unknown_keys() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "[some_future_section]").unwrap();
    writeln!(f, "key = \"value\"").unwrap();
    let config = cortexmem::config::Config::load_from_path(Some(f.path()));
    assert_eq!(config.embedding.model, "AllMiniLML6V2");
}
