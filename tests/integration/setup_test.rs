use cortexmem::cli::setup::{ALL_AGENTS, detect_installed_agents};

#[test]
fn detect_installed_agents_should_return_list() {
    let detected = detect_installed_agents();
    assert_eq!(detected.len(), ALL_AGENTS.len());
    for (agent, _detected) in &detected {
        assert!(!format!("{agent}").is_empty());
    }
}

#[test]
fn all_agents_should_include_zed_and_cline() {
    let names: Vec<String> = ALL_AGENTS.iter().map(|a| a.to_string()).collect();
    assert!(names.iter().any(|a| a == "Zed"), "Missing Zed agent");
    assert!(names.iter().any(|a| a == "Cline"), "Missing Cline agent");
}

#[test]
fn all_agents_should_have_config_paths() {
    for agent in ALL_AGENTS {
        assert!(agent.config_path().is_some(), "{agent} has no config path");
    }
}
