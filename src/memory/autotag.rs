use std::collections::{HashMap, HashSet};

const MAX_CONCEPTS: usize = 8;
const MIN_WORD_LENGTH: usize = 3;

/// Common English stop words to exclude from keyword extraction.
fn stop_words() -> HashSet<&'static str> {
    [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        "from", "as", "is", "was", "are", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "can", "shall",
        "must", "not", "no", "nor", "so", "if", "then", "than", "that", "this", "these", "those",
        "it", "its", "my", "your", "his", "her", "our", "their", "we", "they", "them", "us", "he",
        "she", "who", "which", "what", "when", "where", "how", "all", "each", "every", "both",
        "few", "more", "most", "other", "some", "such", "only", "own", "same", "into", "over",
        "after", "before", "between", "under", "again", "further", "once", "also", "just", "about",
        "very", "there", "here", "out", "up", "down", "off", "any", "because", "through", "during",
        "above", "below", "while", "using", "used",
    ]
    .into_iter()
    .collect()
}

/// Simple suffix stemmer — reduces common English suffixes.
fn stem(word: &str) -> String {
    let w = word.to_lowercase();
    if w.len() < 5 {
        return w;
    }
    for suffix in &[
        "tion", "sion", "ment", "ness", "ance", "ence", "ible", "able", "ious", "eous",
    ] {
        if w.ends_with(suffix) {
            return w[..w.len() - suffix.len()].to_string();
        }
    }
    for suffix in &["ing", "ted", "ied", "ies", "ers", "est"] {
        if w.ends_with(suffix) {
            return w[..w.len() - suffix.len()].to_string();
        }
    }
    if w.ends_with("ed") && w.len() > 4 {
        return w[..w.len() - 2].to_string();
    }
    if w.ends_with('s') && !w.ends_with("ss") && w.len() > 4 {
        return w[..w.len() - 1].to_string();
    }
    w
}

/// Tokenize text into lowercase words, filtering out non-alpha and short tokens.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() >= MIN_WORD_LENGTH)
        .collect()
}

/// Extract top-N keywords from text using TF scoring with stop word removal.
pub fn extract_keywords(text: &str, limit: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }

    let stops = stop_words();
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return vec![];
    }

    // Count term frequency using stemmed forms
    let mut tf: HashMap<String, (usize, String)> = HashMap::new(); // stem -> (count, original)
    for token in &tokens {
        if stops.contains(token.as_str()) {
            continue;
        }
        let stemmed = stem(token);
        if stops.contains(stemmed.as_str()) {
            continue;
        }
        let entry = tf.entry(stemmed).or_insert((0, token.clone()));
        entry.0 += 1;
    }

    // Sort by frequency descending, take top N
    let mut scored: Vec<(String, usize)> = tf
        .into_iter()
        .map(|(_, (count, original))| (original, count))
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    scored
        .into_iter()
        .take(limit.min(MAX_CONCEPTS))
        .map(|(word, _)| word)
        .collect()
}

/// Extract declarative sentences as candidate facts.
/// Picks sentences that look like statements (not questions, not too short).
pub fn extract_facts(text: &str, limit: usize) -> Vec<String> {
    let min_fact_length = 20;
    text.split('.')
        .map(|s| s.trim().to_string())
        .filter(|s| {
            s.len() >= min_fact_length
                && !s.ends_with('?')
                && !s.starts_with("TODO")
                && !s.starts_with("FIXME")
        })
        .take(limit)
        .map(|s| if s.ends_with('.') { s } else { format!("{s}.") })
        .collect()
}
