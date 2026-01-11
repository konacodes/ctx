use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

use crate::analysis::symbols;
use crate::analysis::treesitter::{self, SupportedLanguage};
use crate::analysis::walker;
use crate::output::OutputFormat;

#[derive(Debug, Serialize)]
pub struct ProjectMap {
    pub directories: BTreeMap<String, DirectoryInfo>,
}

#[derive(Debug, Serialize)]
pub struct DirectoryInfo {
    pub path: String,
    pub description: Option<String>,
    pub files: Vec<FileInfo>,
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub name: String,
    pub language: Option<String>,
    pub symbols: usize,
}

impl std::fmt::Display for ProjectMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (_, dir) in &self.directories {
            let desc = dir
                .description
                .as_ref()
                .map(|d| format!("  # {}", d))
                .unwrap_or_default();
            writeln!(f, "{}/{}", dir.path, desc)?;

            for file in &dir.files {
                let lang_info = file
                    .language
                    .as_ref()
                    .map(|l| format!(" [{}]", l))
                    .unwrap_or_default();
                writeln!(f, "  {}{}", file.name, lang_info)?;
            }
        }
        Ok(())
    }
}

pub fn run(path: Option<&str>, depth: Option<usize>, format: OutputFormat) -> Result<()> {
    let root = path.map(Path::new).unwrap_or(Path::new("."));
    let max_depth = depth.unwrap_or(3);

    let mut directories: BTreeMap<String, DirectoryInfo> = BTreeMap::new();

    let file_walker = walker::create_walker(root)
        .max_depth(Some(max_depth))
        .build();

    for entry in file_walker.flatten() {
        let entry_path = entry.path();

        if entry_path.is_dir() {
            let rel_path = entry_path
                .strip_prefix(root)
                .unwrap_or(entry_path)
                .to_string_lossy()
                .to_string();

            if rel_path.is_empty() {
                continue;
            }

            // Skip hidden directories
            if rel_path.starts_with('.') || rel_path.contains("/.") {
                continue;
            }

            let description = get_directory_description(entry_path);

            directories.insert(
                rel_path.clone(),
                DirectoryInfo {
                    path: rel_path,
                    description,
                    files: Vec::new(),
                },
            );
        } else if entry_path.is_file() {
            let rel_path = entry_path
                .strip_prefix(root)
                .unwrap_or(entry_path)
                .to_string_lossy()
                .to_string();

            // Skip hidden files
            if rel_path.starts_with('.') || rel_path.contains("/.") {
                continue;
            }

            let parent = entry_path
                .parent()
                .and_then(|p| p.strip_prefix(root).ok())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());

            let file_name = entry_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let lang = SupportedLanguage::from_path(entry_path);
            let symbol_count = if lang.is_some() {
                count_symbols(entry_path).unwrap_or(0)
            } else {
                0
            };

            let file_info = FileInfo {
                name: file_name,
                language: lang.map(|l| l.name().to_string()),
                symbols: symbol_count,
            };

            if let Some(dir) = directories.get_mut(&parent) {
                dir.files.push(file_info);
            } else if parent == "." || parent.is_empty() {
                // Root level files
                let root_dir = directories
                    .entry(".".to_string())
                    .or_insert_with(|| DirectoryInfo {
                        path: ".".to_string(),
                        description: None,
                        files: Vec::new(),
                    });
                root_dir.files.push(file_info);
            }
        }
    }

    let map = ProjectMap { directories };

    match format {
        OutputFormat::Human => println!("{}", map),
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&map)?);
        }
        OutputFormat::Compact => {
            println!("{}", serde_json::to_string(&map)?);
        }
    }

    Ok(())
}

fn get_directory_description(path: &Path) -> Option<String> {
    // Try to find a README or module-level doc comment
    let readme_names = ["README.md", "README", "readme.md"];
    for name in readme_names {
        let readme_path = path.join(name);
        if readme_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&readme_path) {
                // Get first non-empty line
                for line in content.lines() {
                    let trimmed = line.trim().trim_start_matches('#').trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.chars().take(60).collect());
                    }
                }
            }
        }
    }

    // Try to get description from mod.rs or __init__.py
    let mod_files = ["mod.rs", "lib.rs", "__init__.py", "index.ts", "index.js"];
    for name in mod_files {
        let mod_path = path.join(name);
        if mod_path.exists() {
            if let Some(desc) = get_file_doc_comment(&mod_path) {
                return Some(desc);
            }
        }
    }

    None
}

fn get_file_doc_comment(path: &Path) -> Option<String> {
    let source = std::fs::read_to_string(path).ok()?;

    // Look for doc comments at the start of the file
    for line in source.lines().take(10) {
        let trimmed = line.trim();

        // Rust-style doc comments
        if let Some(stripped) = trimmed.strip_prefix("//!") {
            let desc = stripped.trim();
            if !desc.is_empty() {
                return Some(desc.chars().take(60).collect());
            }
        }

        // Python-style docstrings
        if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
            let desc = trimmed.trim_start_matches("\"\"\"").trim_start_matches("'''");
            let desc = desc.trim_end_matches("\"\"\"").trim_end_matches("'''").trim();
            if !desc.is_empty() {
                return Some(desc.chars().take(60).collect());
            }
        }

        // JS/TS-style JSDoc
        if let Some(stripped) = trimmed.strip_prefix("/**") {
            let desc = stripped.trim_start_matches('*').trim();
            if !desc.is_empty() {
                return Some(desc.chars().take(60).collect());
            }
        }
    }

    None
}

fn count_symbols(path: &Path) -> Result<usize> {
    let lang = SupportedLanguage::from_path(path).ok_or_else(|| anyhow::anyhow!("Unsupported language"))?;
    let source = std::fs::read_to_string(path)?;
    let tree = treesitter::parse_file(path, &source)?
        .ok_or_else(|| anyhow::anyhow!("Failed to parse"))?;
    let syms = symbols::extract_symbols(&tree, &source, &lang);
    Ok(syms.len())
}
