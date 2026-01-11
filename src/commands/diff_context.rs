use anyhow::Result;
use colored::Colorize;
use git2::{DiffOptions, Repository};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

use crate::analysis::git;
use crate::analysis::symbols::{self, SymbolKind};
use crate::analysis::treesitter::{self, SupportedLanguage};
use crate::output::OutputFormat;

#[derive(Debug, Serialize)]
pub struct DiffContext {
    pub ref_name: String,
    pub files_changed: Vec<FileContext>,
    pub callers_affected: Vec<CallerInfo>,
}

#[derive(Debug, Serialize)]
pub struct FileContext {
    pub path: String,
    pub insertions: usize,
    pub deletions: usize,
    pub functions_modified: Vec<FunctionContext>,
}

#[derive(Debug, Serialize)]
pub struct FunctionContext {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub signature: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CallerInfo {
    pub function_modified: String,
    pub called_from: Vec<String>,
}

impl std::fmt::Display for DiffContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Diff context for: {}", self.ref_name.cyan())?;

        for file in &self.files_changed {
            writeln!(
                f,
                "\n{} ({} {}/{})",
                file.path.bold(),
                "+".green(),
                file.insertions,
                format!("-{}", file.deletions).red()
            )?;

            if !file.functions_modified.is_empty() {
                writeln!(f, "  Functions modified:")?;
                for func in &file.functions_modified {
                    if let Some(sig) = &func.signature {
                        writeln!(f, "    {}:{} {}", func.start_line, func.kind, sig)?;
                    } else {
                        writeln!(f, "    {}:{} {}", func.start_line, func.kind, func.name)?;
                    }
                }
            }
        }

        if !self.callers_affected.is_empty() {
            writeln!(f, "\n{}", "Callers of modified functions:".dimmed())?;
            for caller in &self.callers_affected {
                writeln!(f, "  {}:", caller.function_modified)?;
                for location in &caller.called_from {
                    writeln!(f, "    - {}", location)?;
                }
            }
        }

        Ok(())
    }
}

pub fn run(git_ref: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = git::find_repo(&cwd)?;

    let ref_name = git_ref.unwrap_or("HEAD");
    let diff_context = analyze_diff(&repo, ref_name)?;

    match format {
        OutputFormat::Human => println!("{}", diff_context),
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&diff_context)?);
        }
        OutputFormat::Compact => {
            println!("{}", serde_json::to_string(&diff_context)?);
        }
    }

    Ok(())
}

fn analyze_diff(repo: &Repository, ref_name: &str) -> Result<DiffContext> {
    let head = repo.head()?.peel_to_tree()?;

    let mut diff_opts = DiffOptions::new();

    // Get diff between HEAD and working directory (or between refs)
    let diff = if ref_name == "HEAD" {
        repo.diff_tree_to_workdir_with_index(Some(&head), Some(&mut diff_opts))?
    } else {
        let obj = repo.revparse_single(ref_name)?;
        let tree = obj.peel_to_tree()?;
        repo.diff_tree_to_tree(Some(&tree), Some(&head), Some(&mut diff_opts))?
    };

    let mut files_changed = Vec::new();
    let mut modified_functions: HashSet<String> = HashSet::new();

    // Collect changed files first
    let mut changed_file_paths: Vec<String> = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                changed_file_paths.push(path.to_string_lossy().to_string());
            }
            true
        },
        None,
        None,
        None,
    )?;

    // Now collect line changes using print callback
    use std::collections::HashMap;
    let mut file_lines: HashMap<String, Vec<usize>> = HashMap::new();
    for path in &changed_file_paths {
        file_lines.insert(path.clone(), Vec::new());
    }

    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        if line.origin() == '+' || line.origin() == '-' {
            if let Some(path) = delta.new_file().path() {
                let path_str = path.to_string_lossy().to_string();
                if let Some(new_lineno) = line.new_lineno() {
                    if let Some(lines) = file_lines.get_mut(&path_str) {
                        lines.push(new_lineno as usize);
                    }
                }
            }
        }
        true
    })?;

    for path_str in changed_file_paths {
        let changed_lines: Vec<(usize, bool)> = file_lines
            .get(&path_str)
            .map(|lines| lines.iter().map(|&l| (l, true)).collect())
            .unwrap_or_default();
        let path = Path::new(&path_str);

        if !path.exists() {
            continue;
        }

        let stats = diff.stats()?;

        let functions_modified = find_modified_functions(path, &changed_lines)?;

        for func in &functions_modified {
            modified_functions.insert(format!("{}:{}", path_str, func.name));
        }

        files_changed.push(FileContext {
            path: path_str,
            insertions: stats.insertions(),
            deletions: stats.deletions(),
            functions_modified,
        });
    }

    // Find callers of modified functions
    let callers_affected = find_callers(&modified_functions)?;

    Ok(DiffContext {
        ref_name: ref_name.to_string(),
        files_changed,
        callers_affected,
    })
}

fn find_modified_functions(
    path: &Path,
    changed_lines: &[(usize, bool)],
) -> Result<Vec<FunctionContext>> {
    let lang = match SupportedLanguage::from_path(path) {
        Some(l) => l,
        None => return Ok(Vec::new()),
    };

    let content = std::fs::read_to_string(path)?;
    let tree = match treesitter::parse_file(path, &content)? {
        Some(t) => t,
        None => return Ok(Vec::new()),
    };

    let all_symbols = symbols::extract_symbols(&tree, &content, &lang);
    let changed_line_nums: HashSet<usize> = changed_lines.iter().map(|(l, _)| *l).collect();

    let mut modified = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for sym in all_symbols {
        if sym.kind != SymbolKind::Function && sym.kind != SymbolKind::Method {
            continue;
        }

        // Find function end (rough heuristic - look for closing brace at same indent level)
        let func_end = find_function_end(&lines, sym.line - 1);

        // Check if any changed line is within this function
        let in_function = (sym.line..=func_end).any(|l| changed_line_nums.contains(&l));

        if in_function {
            modified.push(FunctionContext {
                name: sym.name,
                kind: sym.kind.to_string(),
                start_line: sym.line,
                signature: sym.signature,
            });
        }
    }

    Ok(modified)
}

fn find_function_end(lines: &[&str], start: usize) -> usize {
    if start >= lines.len() {
        return start + 1;
    }

    let start_line = lines[start];
    let base_indent = start_line.len() - start_line.trim_start().len();

    let mut brace_count = 0;
    let mut found_opening = false;

    for (i, line) in lines.iter().enumerate().skip(start) {
        for c in line.chars() {
            if c == '{' {
                brace_count += 1;
                found_opening = true;
            } else if c == '}' {
                brace_count -= 1;
            }
        }

        if found_opening && brace_count == 0 {
            return i + 1;
        }

        // For Python-style (no braces)
        if !found_opening && i > start {
            let current_indent = line.len() - line.trim_start().len();
            if !line.trim().is_empty() && current_indent <= base_indent {
                return i;
            }
        }
    }

    lines.len()
}

fn find_callers(modified_functions: &HashSet<String>) -> Result<Vec<CallerInfo>> {
    let mut callers = Vec::new();

    for func_ref in modified_functions {
        let parts: Vec<&str> = func_ref.split(':').collect();
        if parts.len() != 2 {
            continue;
        }

        let func_name = parts[1];
        let mut called_from = Vec::new();

        // Search for calls to this function
        let walker = ignore::WalkBuilder::new(".")
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if SupportedLanguage::from_path(path).is_none() {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(path) {
                for (idx, line) in content.lines().enumerate() {
                    // Simple pattern matching for function calls
                    let patterns = [
                        format!("{}(", func_name),
                        format!("{} (", func_name),
                        format!(".{}(", func_name),
                    ];

                    let is_call = patterns.iter().any(|p| line.contains(p));
                    let is_definition = line.contains("fn ")
                        || line.contains("def ")
                        || line.contains("function ");

                    if is_call && !is_definition {
                        called_from.push(format!("{}:{}", path.display(), idx + 1));
                    }
                }
            }
        }

        if !called_from.is_empty() {
            callers.push(CallerInfo {
                function_modified: func_ref.clone(),
                called_from,
            });
        }
    }

    Ok(callers)
}
