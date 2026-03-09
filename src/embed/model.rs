use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Result;
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};

const MODEL_DIR_NAME: &str = "models";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelStatus {
    NotDownloaded,
    Ready,
}

/// Parse a config model name string to a fastembed EmbeddingModel variant.
pub fn parse_model_name(name: &str) -> Option<EmbeddingModel> {
    match name {
        "AllMiniLML6V2" => Some(EmbeddingModel::AllMiniLML6V2),
        "BGESmallENV15" => Some(EmbeddingModel::BGESmallENV15),
        "AllMiniLML12V2" => Some(EmbeddingModel::AllMiniLML12V2),
        _ => None,
    }
}

pub struct EmbeddingManager {
    cache_dir: PathBuf,
    model: Mutex<Option<TextEmbedding>>,
    model_name: String,
}

impl EmbeddingManager {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            model: Mutex::new(None),
            model_name: "AllMiniLML6V2".to_string(),
        }
    }

    pub fn new_with_model(cache_dir: impl Into<PathBuf>, model_name: &str) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            model: Mutex::new(None),
            model_name: model_name.to_string(),
        }
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    pub fn new_with_download(cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let cache_dir = cache_dir.into();
        let model = Self::load_model_for(&cache_dir, "AllMiniLML6V2")?;
        Ok(Self {
            cache_dir,
            model: Mutex::new(Some(model)),
            model_name: "AllMiniLML6V2".to_string(),
        })
    }

    pub fn default_cache_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cortexmem")
    }

    pub fn is_model_available(&self) -> bool {
        let guard = self.model.lock().unwrap_or_else(|e| e.into_inner());
        guard.is_some()
    }

    pub fn model_status(&self) -> ModelStatus {
        if self.is_model_available() {
            ModelStatus::Ready
        } else {
            ModelStatus::NotDownloaded
        }
    }

    pub fn download_model(&self) -> Result<()> {
        let model = Self::load_model_for(&self.cache_dir, &self.model_name)?;
        let mut guard = self.model.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(model);
        Ok(())
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut guard = self.model.lock().unwrap_or_else(|e| e.into_inner());
        let model = guard.as_mut().ok_or_else(|| {
            anyhow::anyhow!("Embedding model not downloaded. Run `cortexmem model download` first.")
        })?;

        let embeddings = model.embed(vec![text], None)?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))
    }

    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut guard = self.model.lock().unwrap_or_else(|e| e.into_inner());
        let model = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Embedding model not downloaded"))?;

        let embeddings = model.embed(texts, None)?;
        Ok(embeddings)
    }

    fn load_model_for(cache_dir: &Path, model_name: &str) -> Result<TextEmbedding> {
        let embedding_model = parse_model_name(model_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown embedding model: {model_name}"))?;
        let options = TextInitOptions::new(embedding_model)
            .with_cache_dir(cache_dir.join(MODEL_DIR_NAME))
            .with_show_download_progress(true);
        TextEmbedding::try_new(options)
    }
}
