use std::collections::HashMap;
use std::path::Path;

use super::git;
use anyhow::Result;
use git2::Repository;

#[derive(Debug)]
pub struct RelevanceScore {
    pub path: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

pub fn score_files_for_prompt(
    repo: &Repository,
    prompt: &str,
    candidates: &[String],
    budget: usize,
) -> Result<Vec<RelevanceScore>> {
    let prompt_lower = prompt.to_lowercase();
    let words: Vec<&str> = prompt_lower.split_whitespace().collect();

    let mut scores: Vec<RelevanceScore> = Vec::new();

    // Get recent file activity for recency scoring
    let recent_activity = git::get_recent_file_activity(repo, 50).unwrap_or_default();
    let activity_map: HashMap<_, _> = recent_activity
        .iter()
        .map(|a| (a.path.clone(), a.commit_count))
        .collect();

    for path in candidates {
        let mut score = 0.0;
        let mut reasons = Vec::new();

        // Check if file path is mentioned in prompt
        let path_lower = path.to_lowercase();
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        if prompt_lower.contains(&path_lower) {
            score += 10.0;
            reasons.push("path mentioned".to_string());
        } else if prompt_lower.contains(&file_name) {
            score += 5.0;
            reasons.push("filename mentioned".to_string());
        }

        // Check for keyword matches in path
        for word in &words {
            if word.len() >= 3 && path_lower.contains(word) {
                score += 1.0;
            }
        }

        // Boost recently active files
        if let Some(&commit_count) = activity_map.get(path) {
            let recency_boost = (commit_count as f64).min(5.0) * 0.5;
            score += recency_boost;
            if commit_count >= 3 {
                reasons.push(format!("{} recent commits", commit_count));
            }
        }

        // Boost based on file type relevance to common terms
        if is_relevant_file_type(path, &prompt_lower) {
            score += 2.0;
            reasons.push("relevant file type".to_string());
        }

        if score > 0.0 {
            scores.push(RelevanceScore {
                path: path.clone(),
                score,
                reasons,
            });
        }
    }

    // Sort by score descending
    scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    // Estimate tokens and truncate to budget
    let mut token_count = 0;
    let mut result = Vec::new();

    for scored in scores {
        // Rough estimate: 4 chars per token
        let estimated_tokens = scored.path.len() / 4 + 10;
        if token_count + estimated_tokens > budget {
            break;
        }
        token_count += estimated_tokens;
        result.push(scored);
    }

    Ok(result)
}

fn is_relevant_file_type(path: &str, prompt: &str) -> bool {
    let path_lower = path.to_lowercase();

    // Test-related keywords
    if prompt.contains("test") || prompt.contains("spec") {
        if path_lower.contains("test") || path_lower.contains("spec") {
            return true;
        }
    }

    // Config-related
    if prompt.contains("config") || prompt.contains("setting") {
        if path_lower.contains("config") || path_lower.ends_with(".toml") || path_lower.ends_with(".yaml") || path_lower.ends_with(".json") {
            return true;
        }
    }

    // Error-related
    if prompt.contains("error") || prompt.contains("bug") || prompt.contains("fix") {
        if path_lower.contains("error") || path_lower.contains("exception") {
            return true;
        }
    }

    false
}

pub fn extract_mentioned_files(prompt: &str) -> Vec<String> {
    let mut files = Vec::new();

    // Common file path patterns
    let words: Vec<&str> = prompt.split_whitespace().collect();

    for word in words {
        let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.' && c != '_' && c != '-');

        // Check if it looks like a file path
        if cleaned.contains('/') || cleaned.contains('.') {
            if let Some(ext) = Path::new(cleaned).extension() {
                let ext_str = ext.to_string_lossy();
                let known_extensions = ["rs", "py", "js", "ts", "jsx", "tsx", "go", "c", "cpp", "h", "java", "rb", "php", "toml", "yaml", "json", "md"];
                if known_extensions.contains(&ext_str.as_ref()) {
                    files.push(cleaned.to_string());
                }
            }
        }
    }

    files
}

pub fn extract_keywords(prompt: &str) -> Vec<String> {
    let stop_words = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "must", "can", "to", "of", "in", "for",
        "on", "with", "at", "by", "from", "as", "into", "through", "during",
        "before", "after", "above", "below", "between", "under", "again",
        "further", "then", "once", "here", "there", "when", "where", "why",
        "how", "all", "each", "few", "more", "most", "other", "some", "such",
        "no", "nor", "not", "only", "own", "same", "so", "than", "too", "very",
        "just", "and", "but", "if", "or", "because", "until", "while", "this",
        "that", "these", "those", "what", "which", "who", "whom", "it", "its",
        "i", "me", "my", "we", "our", "you", "your", "he", "him", "his", "she",
        "her", "they", "them", "their",
    ];

    let prompt_lower = prompt.to_lowercase();
    let words: Vec<String> = prompt_lower
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() >= 3)
        .filter(|w| !stop_words.contains(w))
        .map(|s| s.to_string())
        .collect();

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    words.into_iter().filter(|w| seen.insert(w.clone())).collect()
}
