use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use crate::analysis::symbols::{self, SymbolKind};
use crate::analysis::treesitter::{self, SupportedLanguage};
use crate::analysis::walker;
use crate::output::OutputFormat;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub text: String,
    pub context: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResults {
    pub query: String,
    pub results: Vec<SearchResult>,
}

impl std::fmt::Display for SearchResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for result in &self.results {
            writeln!(f, "{}:{}:{}", result.path, result.line, result.text)?;

            for ctx_line in &result.context {
                writeln!(f, "  {}", ctx_line)?;
            }
        }

        if self.results.is_empty() {
            writeln!(f, "No results found for '{}'", self.query)?;
        }

        Ok(())
    }
}

pub fn run(
    query: &str,
    symbol: bool,
    caller: bool,
    context_lines: usize,
    format: OutputFormat,
) -> Result<()> {
    let results = if symbol {
        search_symbols(query, context_lines)?
    } else if caller {
        search_callers(query, context_lines)?
    } else {
        search_text(query, context_lines)?
    };

    let search_results = SearchResults {
        query: query.to_string(),
        results,
    };

    match format {
        OutputFormat::Human => println!("{}", search_results),
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&search_results)?);
        }
        OutputFormat::Compact => {
            println!("{}", serde_json::to_string(&search_results)?);
        }
    }

    Ok(())
}

fn search_text(query: &str, context_lines: usize) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    let file_walker = walker::create_walker(Path::new(".")).build();

    for entry in file_walker.flatten() {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        // Skip binary files
        if is_binary_file(path) {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(path) {
            let lines: Vec<&str> = content.lines().collect();

            for (idx, line) in lines.iter().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    let start = idx.saturating_sub(context_lines);
                    let end = (idx + context_lines + 1).min(lines.len());

                    let context: Vec<String> = lines[start..end]
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i + start != idx)
                        .map(|(i, l)| format!("{}: {}", start + i + 1, l))
                        .collect();

                    results.push(SearchResult {
                        path: path.to_string_lossy().to_string(),
                        line: idx + 1,
                        column: line.to_lowercase().find(&query_lower).unwrap_or(0) + 1,
                        text: line.to_string(),
                        context,
                    });
                }
            }
        }
    }

    Ok(results)
}

fn search_symbols(query: &str, _context_lines: usize) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    let file_walker = walker::create_walker(Path::new(".")).build();

    for entry in file_walker.flatten() {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let lang = match SupportedLanguage::from_path(path) {
            Some(l) => l,
            None => continue,
        };

        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(Some(tree)) = treesitter::parse_file(path, &content) {
                let syms = symbols::extract_symbols(&tree, &content, &lang);

                for sym in syms {
                    if sym.name.to_lowercase().contains(&query_lower) {
                        let text = sym
                            .signature
                            .as_ref()
                            .unwrap_or(&sym.name)
                            .to_string();

                        results.push(SearchResult {
                            path: path.to_string_lossy().to_string(),
                            line: sym.line,
                            column: 1,
                            text: format!("[{}] {}", sym.kind, text),
                            context: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    Ok(results)
}

fn search_callers(function_name: &str, context_lines: usize) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    // Simple heuristic: search for function calls
    // This is a basic implementation - could be enhanced with proper call graph analysis
    let patterns = [
        format!("{}(", function_name),
        format!("{} (", function_name),
        format!(".{}(", function_name),
        format!("self.{}(", function_name),
    ];

    let file_walker = walker::create_walker(Path::new(".")).build();

    for entry in file_walker.flatten() {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let lang = match SupportedLanguage::from_path(path) {
            Some(l) => l,
            None => continue,
        };

        if let Ok(content) = std::fs::read_to_string(path) {
            let lines: Vec<&str> = content.lines().collect();

            // Check if this file defines the function (for context, not currently used)
            let _is_definition_file =
                if let Ok(Some(tree)) = treesitter::parse_file(path, &content) {
                    let syms = symbols::extract_symbols(&tree, &content, &lang);
                    syms.iter().any(|s| {
                        s.name == function_name
                            && (s.kind == SymbolKind::Function || s.kind == SymbolKind::Method)
                    })
                } else {
                    false
                };

            for (idx, line) in lines.iter().enumerate() {
                let is_call = patterns.iter().any(|p| line.contains(p));

                // Skip definition lines
                let is_definition = line.contains("fn ")
                    || line.contains("def ")
                    || line.contains("function ")
                    || line.contains("func ");

                if is_call && !is_definition {
                    let start = idx.saturating_sub(context_lines);
                    let end = (idx + context_lines + 1).min(lines.len());

                    let context: Vec<String> = lines[start..end]
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i + start != idx)
                        .map(|(i, l)| format!("{}: {}", start + i + 1, l))
                        .collect();

                    results.push(SearchResult {
                        path: path.to_string_lossy().to_string(),
                        line: idx + 1,
                        column: 1,
                        text: line.to_string(),
                        context,
                    });
                }
            }
        }
    }

    Ok(results)
}

fn is_binary_file(path: &Path) -> bool {
    let binary_extensions = [
        "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "pdf", "doc", "docx", "xls", "xlsx",
        "ppt", "pptx", "zip", "tar", "gz", "bz2", "7z", "rar", "exe", "dll", "so", "dylib", "o",
        "a", "lib", "bin", "dat", "db", "sqlite", "wasm", "class", "pyc", "pyo",
    ];

    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| binary_extensions.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}
