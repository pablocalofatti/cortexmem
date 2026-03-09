use anyhow::Result;

use super::EmbeddingManager;

pub fn build_search_text(
    title: &str,
    content: &str,
    concepts: &[&str],
    facts: &[&str],
) -> String {
    let mut parts = vec![title.to_string(), content.to_string()];

    if !concepts.is_empty() {
        parts.push(format!("Concepts: {}", concepts.join(", ")));
    }

    if !facts.is_empty() {
        parts.push(format!("Facts: {}", facts.join(", ")));
    }

    parts.join("\n")
}

pub fn embed_text(manager: &EmbeddingManager, text: &str) -> Result<Option<Vec<f32>>> {
    if !manager.is_model_available() {
        return Ok(None);
    }

    let embedding = manager.embed(text)?;
    Ok(Some(embedding))
}
