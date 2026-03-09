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

pub struct EmbeddingManager {
    cache_dir: PathBuf,
    model: Mutex<Option<TextEmbedding>>,
}

impl EmbeddingManager {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            model: Mutex::new(None),
        }
    }

    pub fn new_with_download(cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let cache_dir = cache_dir.into();
        let model = Self::load_model(&cache_dir)?;
        Ok(Self {
            cache_dir,
            model: Mutex::new(Some(model)),
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
        let model = Self::load_model(&self.cache_dir)?;
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

    fn load_model(cache_dir: &Path) -> Result<TextEmbedding> {
        let options = TextInitOptions::new(EmbeddingModel::AllMiniLML6V2)
            .with_cache_dir(cache_dir.join(MODEL_DIR_NAME))
            .with_show_download_progress(true);

        let model = TextEmbedding::try_new(options)?;
        Ok(model)
    }
}
