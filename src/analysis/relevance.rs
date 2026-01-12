use std::collections::HashMap;
use std::path::Path;

use super::git;
use anyhow::Result;
use git2::Repository;

/// Represents a file's relevance score relative to a given prompt or query.
///
/// This struct captures the result of scoring a file's relevance based on
/// various heuristics including path matching, keyword matching, file type
/// relevance, and recent git activity.
///
/// # Fields
/// * `path` - The file path (relative to repository root)
/// * `score` - Numeric relevance score (higher = more relevant)
/// * `reasons` - Human-readable explanations for why this file scored highly
#[derive(Debug)]
pub struct RelevanceScore {
    /// The file path relative to the repository root.
    pub path: String,
    /// The computed relevance score. Higher values indicate greater relevance.
    /// Scores are additive based on multiple factors (path match, keywords, etc.).
    pub score: f64,
    /// Human-readable reasons explaining why this file was scored as relevant.
    /// Examples: "path mentioned", "filename mentioned", "3 recent commits".
    pub reasons: Vec<String>,
}

/// Scores a list of candidate files for relevance to a given prompt.
///
/// This function analyzes each candidate file and assigns a relevance score
/// based on multiple heuristics. The results are sorted by score (descending)
/// and truncated to fit within a token budget.
///
/// # Arguments
/// * `repo` - Git repository for accessing commit history
/// * `prompt` - The user's query or prompt text
/// * `candidates` - List of file paths to evaluate
/// * `budget` - Maximum estimated token count for the returned results
///
/// # Returns
/// A vector of [`RelevanceScore`] instances, sorted by score in descending order,
/// truncated to fit within the token budget.
///
/// # Scoring Heuristics
/// - **+10.0**: Full path mentioned in prompt
/// - **+5.0**: Filename mentioned in prompt
/// - **+1.0**: Each keyword (3+ chars) found in path
/// - **+0.5-2.5**: Recent git activity (up to 5 commits)
/// - **+2.0**: File type matches prompt context (test, config, error files)
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

/// Extracts file paths mentioned in a prompt string.
///
/// This function scans the prompt text looking for words that appear to be
/// file paths based on common patterns (containing `/` or `.`) and having
/// recognized source code extensions.
///
/// # Arguments
/// * `prompt` - The text to scan for file path mentions
///
/// # Returns
/// A vector of file paths found in the prompt. Paths are cleaned of surrounding
/// punctuation but preserve internal structure.
///
/// # Recognized Extensions
/// Rust (.rs), Python (.py), JavaScript (.js, .jsx), TypeScript (.ts, .tsx),
/// Go (.go), C/C++ (.c, .cpp, .h), Java (.java), Ruby (.rb), PHP (.php),
/// Config (.toml, .yaml, .json), Documentation (.md)
///
/// # Example
/// ```ignore
/// let files = extract_mentioned_files("Fix bug in src/main.rs and config.toml");
/// // Returns: ["src/main.rs", "config.toml"]
/// ```
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

/// Extracts meaningful keywords from a prompt for relevance scoring.
///
/// This function tokenizes the prompt and filters out common English stop words
/// (articles, prepositions, pronouns, etc.) to identify the content-bearing
/// keywords that are most useful for matching against file paths and content.
///
/// # Arguments
/// * `prompt` - The text to extract keywords from
///
/// # Returns
/// A vector of lowercase keywords, deduplicated while preserving order of first
/// occurrence. Only words with 3+ characters are included.
///
/// # Filtering
/// - Words shorter than 3 characters are excluded
/// - Common stop words (the, a, is, are, to, from, etc.) are excluded
/// - Underscores within words are preserved (e.g., "user_profile")
///
/// # Example
/// ```ignore
/// let keywords = extract_keywords("Fix the authentication bug in user_service");
/// // Returns: ["fix", "authentication", "bug", "user_service"]
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        // Test basic keyword extraction
        let keywords = extract_keywords("implement authentication system");
        assert!(keywords.contains(&"implement".to_string()));
        assert!(keywords.contains(&"authentication".to_string()));
        assert!(keywords.contains(&"system".to_string()));

        // Test with mixed case
        let keywords = extract_keywords("Refactor DATABASE Connection");
        assert!(keywords.contains(&"refactor".to_string()));
        assert!(keywords.contains(&"database".to_string()));
        assert!(keywords.contains(&"connection".to_string()));

        // Test with underscores (should be kept in words)
        let keywords = extract_keywords("update user_profile handler");
        assert!(keywords.contains(&"update".to_string()));
        assert!(keywords.contains(&"user_profile".to_string()));
        assert!(keywords.contains(&"handler".to_string()));

        // Test deduplication
        let keywords = extract_keywords("test test test functionality");
        let test_count = keywords.iter().filter(|&k| k == "test").count();
        assert_eq!(test_count, 1, "Keywords should be deduplicated");

        // Test empty input
        let keywords = extract_keywords("");
        assert!(keywords.is_empty());

        // Test single word
        let keywords = extract_keywords("refactoring");
        assert_eq!(keywords.len(), 1);
        assert!(keywords.contains(&"refactoring".to_string()));
    }

    #[test]
    fn test_extract_keywords_filters_stop_words() {
        // Test that common stop words are filtered out
        let keywords = extract_keywords("the quick brown fox jumps with the lazy dog");

        // Stop words should not be present
        assert!(!keywords.contains(&"the".to_string()));
        assert!(!keywords.contains(&"with".to_string()));

        // Content words should be present
        assert!(keywords.contains(&"quick".to_string()));
        assert!(keywords.contains(&"brown".to_string()));
        assert!(keywords.contains(&"fox".to_string()));
        assert!(keywords.contains(&"jumps".to_string()));
        assert!(keywords.contains(&"lazy".to_string()));
        assert!(keywords.contains(&"dog".to_string()));

        // Test more stop words
        let keywords = extract_keywords("I want to create a new file for this project");
        assert!(keywords.contains(&"want".to_string())); // "want" has 4 chars and is NOT a stop word
        assert!(!keywords.contains(&"for".to_string()));
        assert!(!keywords.contains(&"this".to_string()));
        assert!(keywords.contains(&"create".to_string()));
        assert!(keywords.contains(&"new".to_string()));
        assert!(keywords.contains(&"file".to_string()));
        assert!(keywords.contains(&"project".to_string()));

        // Test short words (less than 3 chars) are filtered
        let keywords = extract_keywords("go to do it");
        assert!(keywords.is_empty(), "Short words should be filtered");

        // Test pronouns are filtered
        let keywords = extract_keywords("they should implement their own solution");
        assert!(!keywords.contains(&"they".to_string()));
        assert!(!keywords.contains(&"should".to_string()));
        assert!(!keywords.contains(&"their".to_string()));
        assert!(!keywords.contains(&"own".to_string()));
        assert!(keywords.contains(&"implement".to_string()));
        assert!(keywords.contains(&"solution".to_string()));
    }

    #[test]
    fn test_extract_mentioned_files() {
        // Test basic file path extraction
        let files = extract_mentioned_files("Please fix the bug in src/main.rs");
        assert!(files.contains(&"src/main.rs".to_string()));

        // Test multiple files
        let files = extract_mentioned_files("Update config.json and utils.py");
        assert!(files.contains(&"config.json".to_string()));
        assert!(files.contains(&"utils.py".to_string()));

        // Test file without path
        let files = extract_mentioned_files("Check the handler.js file");
        assert!(files.contains(&"handler.js".to_string()));

        // Test with punctuation around file path
        let files = extract_mentioned_files("Look at (src/lib.rs) for details");
        assert!(files.contains(&"src/lib.rs".to_string()));

        // Test nested paths
        let files = extract_mentioned_files("The error is in src/analysis/relevance.rs");
        assert!(files.contains(&"src/analysis/relevance.rs".to_string()));

        // Test no files mentioned
        let files = extract_mentioned_files("Implement a new feature");
        assert!(files.is_empty());
    }

    #[test]
    fn test_extract_mentioned_files_with_extensions() {
        // Test Rust files
        let files = extract_mentioned_files("main.rs and lib.rs");
        assert!(files.contains(&"main.rs".to_string()));
        assert!(files.contains(&"lib.rs".to_string()));

        // Test Python files
        let files = extract_mentioned_files("script.py and module.py");
        assert!(files.contains(&"script.py".to_string()));
        assert!(files.contains(&"module.py".to_string()));

        // Test JavaScript/TypeScript files
        let files = extract_mentioned_files("app.js, component.jsx, utils.ts, form.tsx");
        assert!(files.contains(&"app.js".to_string()));
        assert!(files.contains(&"component.jsx".to_string()));
        assert!(files.contains(&"utils.ts".to_string()));
        assert!(files.contains(&"form.tsx".to_string()));

        // Test Go files
        let files = extract_mentioned_files("main.go and server.go");
        assert!(files.contains(&"main.go".to_string()));
        assert!(files.contains(&"server.go".to_string()));

        // Test C/C++ files
        let files = extract_mentioned_files("main.c, helper.cpp, types.h");
        assert!(files.contains(&"main.c".to_string()));
        assert!(files.contains(&"helper.cpp".to_string()));
        assert!(files.contains(&"types.h".to_string()));

        // Test Java files
        let files = extract_mentioned_files("Main.java");
        assert!(files.contains(&"Main.java".to_string()));

        // Test Ruby files
        let files = extract_mentioned_files("app.rb");
        assert!(files.contains(&"app.rb".to_string()));

        // Test PHP files
        let files = extract_mentioned_files("index.php");
        assert!(files.contains(&"index.php".to_string()));

        // Test config files
        let files = extract_mentioned_files("Cargo.toml, config.yaml, settings.json");
        assert!(files.contains(&"Cargo.toml".to_string()));
        assert!(files.contains(&"config.yaml".to_string()));
        assert!(files.contains(&"settings.json".to_string()));

        // Test markdown files
        let files = extract_mentioned_files("README.md");
        assert!(files.contains(&"README.md".to_string()));

        // Test unknown extensions are not included
        let files = extract_mentioned_files("document.xyz and file.unknown");
        assert!(files.is_empty());
    }

    #[test]
    fn test_is_relevant_file_type() {
        // Test test-related files
        assert!(is_relevant_file_type("src/tests/mod.rs", "run the tests"));
        assert!(is_relevant_file_type("spec/helper.rb", "check spec files"));
        assert!(is_relevant_file_type("test_utils.py", "test utilities"));
        assert!(!is_relevant_file_type("src/main.rs", "run the tests"));

        // Test config-related files
        assert!(is_relevant_file_type("config/database.toml", "update config"));
        assert!(is_relevant_file_type("settings.yaml", "change settings"));
        assert!(is_relevant_file_type("app.json", "modify config"));
        assert!(is_relevant_file_type("config.rs", "update configuration"));
        assert!(!is_relevant_file_type("src/main.rs", "update config"));

        // Test error-related files
        assert!(is_relevant_file_type("src/error.rs", "fix the error handling"));
        assert!(is_relevant_file_type("exceptions.py", "handle bug in exception"));
        assert!(is_relevant_file_type("error_handler.js", "fix the bug"));
        assert!(!is_relevant_file_type("src/main.rs", "fix the error"));

        // Test case insensitivity
        assert!(is_relevant_file_type("src/TEST/mod.rs", "run the tests"));
        assert!(is_relevant_file_type("CONFIG.YAML", "update config"));

        // Test no match cases
        assert!(!is_relevant_file_type("src/main.rs", "implement new feature"));
        assert!(!is_relevant_file_type("lib.rs", "add functionality"));
    }
}
