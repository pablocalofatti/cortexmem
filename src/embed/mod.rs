mod model;
mod pipeline;

pub use model::{EmbeddingManager, ModelStatus, parse_model_name};
pub use pipeline::build_search_text;
