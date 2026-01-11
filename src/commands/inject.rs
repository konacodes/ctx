use anyhow::Result;
use std::io::{self, Read};

use crate::analysis::git;
use crate::analysis::relevance;
use crate::analysis::treesitter;
use crate::analysis::walker;

#[derive(Debug, Clone, Copy)]
pub enum InjectFormat {
    Prepend,
    Append,
    Wrap,
}

impl std::str::FromStr for InjectFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "prepend" => Ok(InjectFormat::Prepend),
            "append" => Ok(InjectFormat::Append),
            "wrap" => Ok(InjectFormat::Wrap),
            _ => anyhow::bail!("Invalid format: {}. Use prepend, append, or wrap", s),
        }
    }
}

pub fn run(budget: usize, format: InjectFormat) -> Result<()> {
    // Read prompt from stdin
    let mut prompt = String::new();
    io::stdin().read_to_string(&mut prompt)?;

    let context = build_context(&prompt, budget)?;

    match format {
        InjectFormat::Prepend => {
            println!("{}", context);
            println!("---");
            print!("{}", prompt);
        }
        InjectFormat::Append => {
            print!("{}", prompt);
            println!("---");
            println!("{}", context);
        }
        InjectFormat::Wrap => {
            println!("[CTX-START]");
            println!("{}", context);
            println!("[CTX-END]");
            print!("{}", prompt);
        }
    }

    Ok(())
}

fn build_context(prompt: &str, budget: usize) -> Result<String> {
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

fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: ~4 characters per token
    (text.len() + 3) / 4
}
