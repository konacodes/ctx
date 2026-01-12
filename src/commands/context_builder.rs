use anyhow::Result;

use crate::analysis::git;
use crate::analysis::relevance;
use crate::analysis::treesitter;
use crate::analysis::walker;

/// Build context string for a prompt within a token budget.
///
/// # Arguments
/// * `prompt` - The user's prompt to analyze for context
/// * `budget` - Maximum token budget for the context
/// * `include_uncommitted` - Whether to include uncommitted diff stats
///
/// # Returns
/// A formatted context string with project info, recent files, mentioned files,
/// relevant files, and keywords.
pub fn build_context(prompt: &str, budget: usize, include_uncommitted: bool) -> Result<String> {
    let mut context_parts = Vec::new();
    let mut tokens_used = 0;

    let cwd = std::env::current_dir()?;

    // Project info
    let project_name = treesitter::detect_project_name(&cwd).unwrap_or_else(|| "unknown".to_string());
    let project_type = treesitter::detect_project_type(&cwd).unwrap_or("unknown");

    // Git info
    let git_info = if let Ok(repo) = git::find_repo(&cwd) {
        let status = git::get_status(&repo).ok();
        let branch = status.as_ref().map(|s| s.branch.clone()).unwrap_or_else(|| "unknown".to_string());

        let dirty_marker = if status.as_ref().map(|s| s.is_dirty).unwrap_or(false) {
            "*"
        } else {
            ""
        };

        format!("branch={}{}", branch, dirty_marker)
    } else {
        "no-git".to_string()
    };

    let header = format!("[CTX: project={} lang={} {}]", project_name, project_type, git_info);
    tokens_used += estimate_tokens(&header);
    context_parts.push(header);

    // Recent file activity
    if let Ok(repo) = git::find_repo(&cwd) {
        if let Ok(activity) = git::get_recent_file_activity(&repo, 5) {
            for file in activity.iter().take(3) {
                let line = format!("[RECENT: {} modified {}]", file.path, file.last_modified);
                let line_tokens = estimate_tokens(&line);
                if tokens_used + line_tokens > budget {
                    break;
                }
                tokens_used += line_tokens;
                context_parts.push(line);
            }
        }

        // Get diff stats if there are uncommitted changes (only when requested)
        if include_uncommitted {
            if let Ok((ins, del)) = git::get_diff_summary(&repo) {
                if ins > 0 || del > 0 {
                    let line = format!("[UNCOMMITTED: +{} -{}]", ins, del);
                    let line_tokens = estimate_tokens(&line);
                    if tokens_used + line_tokens <= budget {
                        tokens_used += line_tokens;
                        context_parts.push(line);
                    }
                }
            }
        }
    }

    // Find files mentioned in prompt
    let mentioned_files = relevance::extract_mentioned_files(prompt);
    for file in mentioned_files.iter().take(5) {
        let line = format!("[MENTIONED: {}]", file);
        let line_tokens = estimate_tokens(&line);
        if tokens_used + line_tokens > budget {
            break;
        }
        tokens_used += line_tokens;
        context_parts.push(line);
    }

    // Extract keywords and find relevant files
    let keywords = relevance::extract_keywords(prompt);
    if !keywords.is_empty() {
        // Collect all source files (respecting .gitignore and common ignores)
        let mut all_files = Vec::new();
        let file_walker = walker::create_walker(&cwd).build();

        for entry in file_walker.flatten() {
            if entry.path().is_file() {
                if let Some(path) = entry.path().strip_prefix(&cwd).ok() {
                    all_files.push(path.to_string_lossy().to_string());
                }
            }
        }

        // Score files for relevance
        if let Ok(repo) = git::find_repo(&cwd) {
            if let Ok(scored) = relevance::score_files_for_prompt(&repo, prompt, &all_files, budget - tokens_used) {
                for scored_file in scored.iter().take(5) {
                    let reasons = scored_file.reasons.join(", ");
                    let line = format!("[RELEVANT: {} ({})]", scored_file.path, reasons);
                    let line_tokens = estimate_tokens(&line);
                    if tokens_used + line_tokens > budget {
                        break;
                    }
                    tokens_used += line_tokens;
                    context_parts.push(line);
                }
            }
        }
    }

    // Keywords summary
    if !keywords.is_empty() && tokens_used < budget {
        let keywords_str = keywords.iter().take(10).cloned().collect::<Vec<_>>().join(", ");
        let line = format!("[KEYWORDS: {}]", keywords_str);
        let line_tokens = estimate_tokens(&line);
        if tokens_used + line_tokens <= budget {
            context_parts.push(line);
        }
    }

    Ok(context_parts.join("\n"))
}

/// Estimate token count for text using a hybrid word/character approach.
///
/// This provides a more accurate estimate than pure character count,
/// especially for code which tends to have shorter tokens due to
/// punctuation and operators.
///
/// # Algorithm
/// 1. Count words (split on whitespace)
/// 2. Count punctuation/operators (often individual tokens in code)
/// 3. Character-based estimate (non-whitespace / 4)
/// 4. Weighted word estimate (words * 1.3 + punctuation / 2)
/// 5. Average the character and word estimates
///
/// # Examples
/// ```
/// use ctx::commands::context_builder::estimate_tokens;
///
/// assert_eq!(estimate_tokens(""), 0);
/// assert!(estimate_tokens("hello world") >= 2);
/// ```
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    // Count words
    let word_count = text.split_whitespace().count();

    // Count punctuation/operators (these are often individual tokens)
    let punct_count = text
        .chars()
        .filter(|c| {
            matches!(
                c,
                '(' | ')'
                    | '{'
                    | '}'
                    | '['
                    | ']'
                    | ';'
                    | ','
                    | '.'
                    | ':'
                    | '<'
                    | '>'
                    | '='
                    | '+'
                    | '-'
                    | '*'
                    | '/'
                    | '&'
                    | '|'
                    | '!'
                    | '@'
                    | '#'
                    | '$'
                    | '%'
                    | '^'
            )
        })
        .count();

    // Character-based estimate (for non-whitespace)
    let char_count = text.chars().filter(|c| !c.is_whitespace()).count();
    let char_estimate = (char_count + 3) / 4;

    // Weighted average: code typically has ~1.3 tokens per word due to operators
    // and shorter identifiers
    let word_estimate = (word_count as f64 * 1.3) as usize + punct_count / 2;

    // Take the average of both approaches for robustness
    (char_estimate + word_estimate) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_simple_text() {
        // "hello world" - 2 words, ~11 chars
        let tokens = estimate_tokens("hello world");
        assert!(tokens >= 2 && tokens <= 5, "Expected 2-5 tokens, got {}", tokens);
    }

    #[test]
    fn test_estimate_tokens_code() {
        // Code with punctuation
        let code = "fn main() { println!(\"hello\"); }";
        let tokens = estimate_tokens(code);
        // Should account for punctuation
        assert!(tokens >= 6, "Expected at least 6 tokens for code, got {}", tokens);
    }

    #[test]
    fn test_estimate_tokens_punctuation_heavy() {
        // Dense punctuation like JSON or code
        let punct = "[(1, 2), (3, 4)]";
        let tokens = estimate_tokens(punct);
        assert!(tokens >= 4, "Expected at least 4 tokens for punctuation-heavy text, got {}", tokens);
    }

    #[test]
    fn test_estimate_tokens_long_identifiers() {
        // Long camelCase identifiers
        let code = "calculateTotalAmountWithTaxAndDiscount";
        let tokens = estimate_tokens(code);
        // Long identifier should estimate reasonable tokens
        assert!(tokens >= 5, "Expected at least 5 tokens for long identifier, got {}", tokens);
    }

    #[test]
    fn test_estimate_tokens_mixed_content() {
        // Prose mixed with code
        let mixed = "The function `getUserById(id)` returns a User object.";
        let tokens = estimate_tokens(mixed);
        assert!(tokens >= 8, "Expected at least 8 tokens for mixed content, got {}", tokens);
    }
}
