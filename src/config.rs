use std::path::{Path, PathBuf};

use serde::Deserialize;

const DEFAULT_MODEL: &str = "AllMiniLML6V2";

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub embedding: EmbeddingConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EmbeddingConfig {
    pub model: String,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
        }
    }
}

impl Config {
    /// Load config from a specific path, or return defaults if path is None / file doesn't exist.
    pub fn load_from_path(path: Option<&Path>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };
        match std::fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse config at {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Load config from the standard platform-specific location.
    pub fn load() -> Self {
        let path = Self::default_path();
        Self::load_from_path(path.as_deref())
    }

    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("cortexmem").join("config.toml"))
    }
}
