use anyhow::Result;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

use crate::analysis::git;
use crate::analysis::symbols;
use crate::analysis::treesitter::{self, SupportedLanguage};
use crate::analysis::walker;
use crate::output::OutputFormat;

#[derive(Debug, Serialize)]
pub struct RelatedFiles {
    pub source: String,
    pub imports: Vec<RelatedFile>,
    pub imported_by: Vec<RelatedFile>,
    pub co_changed: Vec<RelatedFile>,
    pub test_files: Vec<RelatedFile>,
}

#[derive(Debug, Serialize)]
pub struct RelatedFile {
    pub path: String,
    pub reason: String,
}

impl std::fmt::Display for RelatedFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Related to: {}", self.source)?;

        if !self.imports.is_empty() {
            writeln!(f, "\nImports:")?;
            for file in &self.imports {
                writeln!(f, "  {} ({})", file.path, file.reason)?;
            }
        }

        if !self.imported_by.is_empty() {
            writeln!(f, "\nImported by:")?;
            for file in &self.imported_by {
                writeln!(f, "  {} ({})", file.path, file.reason)?;
            }
        }

        if !self.co_changed.is_empty() {
            writeln!(f, "\nCommonly edited together:")?;
            for file in &self.co_changed {
                writeln!(f, "  {} ({})", file.path, file.reason)?;
            }
        }

        if !self.test_files.is_empty() {
            writeln!(f, "\nTest files:")?;
            for file in &self.test_files {
                writeln!(f, "  {}", file.path)?;
            }
        }

        Ok(())
    }
}

pub fn run(file_path: &str, format: OutputFormat) -> Result<()> {
    let path = Path::new(file_path);

    if !path.exists() {
        anyhow::bail!("File does not exist: {}", file_path);
    }

    let imports = find_imports(path)?;
    let imported_by = find_imported_by(path)?;
    let co_changed = find_co_changed(path)?;
    let test_files = find_test_files(path)?;

    let related = RelatedFiles {
        source: file_path.to_string(),
        imports,
        imported_by,
        co_changed,
        test_files,
    };

    match format {
        OutputFormat::Human => println!("{}", related),
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&related)?);
        }
        OutputFormat::Compact => {
            println!("{}", serde_json::to_string(&related)?);
        }
    }

    Ok(())
}

fn find_imports(path: &Path) -> Result<Vec<RelatedFile>> {
    let mut related = Vec::new();

    let lang = match SupportedLanguage::from_path(path) {
        Some(l) => l,
        None => return Ok(related),
    };

    let content = std::fs::read_to_string(path)?;
    let tree = match treesitter::parse_file(path, &content)? {
        Some(t) => t,
        None => return Ok(related),
    };

    let imports = symbols::find_imports(&tree, &content, &lang);

    for import in imports {
        // Try to resolve import to a file path
        if let Some(resolved) = resolve_import(&import, path, &lang) {
            related.push(RelatedFile {
                path: resolved,
                reason: import,
            });
        }
    }

    Ok(related)
}

fn find_imported_by(path: &Path) -> Result<Vec<RelatedFile>> {
    let mut related = Vec::new();
    let target_name = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    if target_name.is_empty() {
        return Ok(related);
    }

    let file_walker = walker::create_walker(Path::new(".")).build();

    for entry in file_walker.flatten() {
        let entry_path = entry.path();

        if !entry_path.is_file() || entry_path == path {
            continue;
        }

        let lang = match SupportedLanguage::from_path(entry_path) {
            Some(l) => l,
            None => continue,
        };

        if let Ok(content) = std::fs::read_to_string(entry_path) {
            if let Ok(Some(tree)) = treesitter::parse_file(entry_path, &content) {
                let imports = symbols::find_imports(&tree, &content, &lang);

                for import in imports {
                    if import.contains(&target_name) {
                        related.push(RelatedFile {
                            path: entry_path.to_string_lossy().to_string(),
                            reason: import,
                        });
                        break;
                    }
                }
            }
        }
    }

    Ok(related)
}

fn find_co_changed(path: &Path) -> Result<Vec<RelatedFile>> {
    let cwd = std::env::current_dir()?;
    let repo = git::find_repo(&cwd)?;

    let file_path = path
        .strip_prefix(&cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let co_changes = git::get_files_changed_with(&repo, &file_path, 10)?;

    Ok(co_changes
        .into_iter()
        .map(|(path, count)| RelatedFile {
            path,
            reason: format!("{} commits together", count),
        })
        .collect())
}

fn find_test_files(path: &Path) -> Result<Vec<RelatedFile>> {
    let mut related = Vec::new();

    let file_stem = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    if file_stem.is_empty() {
        return Ok(related);
    }

    let test_patterns = [
        format!("{}_test", file_stem),
        format!("test_{}", file_stem),
        format!("{}.test", file_stem),
        format!("{}.spec", file_stem),
        format!("{}_spec", file_stem),
    ];

    let file_walker = walker::create_walker(Path::new(".")).build();

    let mut seen = HashSet::new();

    for entry in file_walker.flatten() {
        let entry_path = entry.path();

        if !entry_path.is_file() {
            continue;
        }

        let entry_stem = entry_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        for pattern in &test_patterns {
            if entry_stem == pattern {
                let path_str = entry_path.to_string_lossy().to_string();
                if seen.insert(path_str.clone()) {
                    related.push(RelatedFile {
                        path: path_str,
                        reason: "test file".to_string(),
                    });
                }
            }
        }

        // Also check for files in a tests directory
        let path_str = entry_path.to_string_lossy();
        if (path_str.contains("/tests/") || path_str.contains("/test/"))
            && path_str.contains(&file_stem)
        {
            let path_string = path_str.to_string();
            if seen.insert(path_string.clone()) {
                related.push(RelatedFile {
                    path: path_string,
                    reason: "test file".to_string(),
                });
            }
        }
    }

    Ok(related)
}

fn resolve_import(import: &str, source: &Path, lang: &SupportedLanguage) -> Option<String> {
    // This is a simplified resolver - production would need more sophisticated path resolution
    match lang {
        SupportedLanguage::Rust => {
            // Extract crate/module name from use statement
            let parts: Vec<&str> = import
                .trim_start_matches("use ")
                .trim_end_matches(';')
                .split("::")
                .collect();

            if parts.is_empty() {
                return None;
            }

            // Check for local modules
            let parent = source.parent()?;
            let module_name = parts.last()?;

            let candidates = [
                parent.join(format!("{}.rs", module_name)),
                parent.join(module_name).join("mod.rs"),
            ];

            for candidate in candidates {
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
        }
        SupportedLanguage::Python => {
            // Extract module path from import statement
            let path_part = if import.starts_with("from ") {
                import
                    .trim_start_matches("from ")
                    .split(' ')
                    .next()
                    .unwrap_or("")
            } else {
                import
                    .trim_start_matches("import ")
                    .split(' ')
                    .next()
                    .unwrap_or("")
            };

            let file_path = path_part.replace('.', "/") + ".py";
            let candidate = Path::new(&file_path);

            if candidate.exists() {
                return Some(file_path);
            }
        }
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
            // Extract path from import statement
            if let Some(start) = import.find(['\'', '"']) {
                let rest = &import[start + 1..];
                if let Some(end) = rest.find(['\'', '"']) {
                    let path_str = &rest[..end];

                    if path_str.starts_with('.') {
                        let parent = source.parent()?;
                        let extensions = ["", ".js", ".ts", ".jsx", ".tsx", "/index.js", "/index.ts"];

                        for ext in extensions {
                            let candidate = parent.join(format!("{}{}", path_str, ext));
                            if candidate.exists() {
                                return Some(candidate.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
