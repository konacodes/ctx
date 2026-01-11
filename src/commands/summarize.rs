use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

use crate::analysis::symbols::{self, Symbol};
use crate::analysis::treesitter::{self, SupportedLanguage};
use crate::analysis::walker;
use crate::output::OutputFormat;

#[derive(Debug, Serialize)]
pub struct FileSummary {
    pub path: String,
    pub language: Option<String>,
    pub lines: usize,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DirectorySummary {
    pub path: String,
    pub file_count: usize,
    pub files: Vec<FileSummary>,
}

impl std::fmt::Display for FileSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let lang_str = self
            .language
            .as_ref()
            .map(|l| format!(" [{}]", l))
            .unwrap_or_default();

        writeln!(f, "{}{} ({} lines)", self.path, lang_str, self.lines)?;

        if !self.imports.is_empty() {
            writeln!(f, "\nImports:")?;
            for import in &self.imports {
                writeln!(f, "  {}", import)?;
            }
        }

        if !self.symbols.is_empty() {
            writeln!(f, "\nSymbols:")?;
            for sym in &self.symbols {
                if let Some(sig) = &sym.signature {
                    writeln!(f, "  {}:{} {}", sym.kind, sym.line, sig)?;
                } else {
                    writeln!(f, "  {}:{} {}", sym.kind, sym.line, sym.name)?;
                }
            }
        }

        Ok(())
    }
}

impl std::fmt::Display for DirectorySummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} ({} files)", self.path, self.file_count)?;

        for file in &self.files {
            writeln!(f)?;
            write!(f, "{}", file)?;
        }

        Ok(())
    }
}

pub fn run(
    path: &str,
    depth: Option<usize>,
    skeleton: bool,
    format: OutputFormat,
) -> Result<()> {
    let target = Path::new(path);

    if !target.exists() {
        anyhow::bail!("Path does not exist: {}", path);
    }

    if target.is_file() {
        let summary = summarize_file(target)?;

        if skeleton {
            print_skeleton(target, format)?;
        } else {
            match format {
                OutputFormat::Human => println!("{}", summary),
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
                OutputFormat::Compact => {
                    println!("{}", serde_json::to_string(&summary)?);
                }
            }
        }
    } else {
        let summary = summarize_directory(target, depth.unwrap_or(1))?;

        match format {
            OutputFormat::Human => println!("{}", summary),
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
            OutputFormat::Compact => {
                println!("{}", serde_json::to_string(&summary)?);
            }
        }
    }

    Ok(())
}

fn summarize_file(path: &Path) -> Result<FileSummary> {
    let source = std::fs::read_to_string(path).context("Failed to read file")?;
    let lines = source.lines().count();

    let lang = SupportedLanguage::from_path(path);
    let (symbols_list, imports) = if let Some(ref l) = lang {
        if let Some(tree) = treesitter::parse_file(path, &source)? {
            let syms = symbols::extract_symbols(&tree, &source, l);
            let imps = symbols::find_imports(&tree, &source, l);
            (syms, imps)
        } else {
            (Vec::new(), Vec::new())
        }
    } else {
        (Vec::new(), Vec::new())
    };

    Ok(FileSummary {
        path: path.to_string_lossy().to_string(),
        language: lang.map(|l| l.name().to_string()),
        lines,
        symbols: symbols_list,
        imports,
    })
}

fn summarize_directory(path: &Path, depth: usize) -> Result<DirectorySummary> {
    let mut files = Vec::new();

    let file_walker = walker::create_walker(path)
        .max_depth(Some(depth + 1))
        .build();

    for entry in file_walker.flatten() {
        let entry_path = entry.path();

        if entry_path.is_file() {
            // Only summarize supported languages
            if SupportedLanguage::from_path(entry_path).is_some() {
                if let Ok(summary) = summarize_file(entry_path) {
                    files.push(summary);
                }
            }
        }
    }

    Ok(DirectorySummary {
        path: path.to_string_lossy().to_string(),
        file_count: files.len(),
        files,
    })
}

fn print_skeleton(path: &Path, format: OutputFormat) -> Result<()> {
    let source = std::fs::read_to_string(path).context("Failed to read file")?;
    let lang =
        SupportedLanguage::from_path(path).ok_or_else(|| anyhow::anyhow!("Unsupported language"))?;

    let tree =
        treesitter::parse_file(path, &source)?.ok_or_else(|| anyhow::anyhow!("Failed to parse"))?;

    let skeleton = symbols::get_skeleton(&tree, &source, &lang);

    match format {
        OutputFormat::Human => println!("{}", skeleton),
        OutputFormat::Json => {
            let output = serde_json::json!({
                "path": path.to_string_lossy(),
                "skeleton": skeleton
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Compact => {
            let output = serde_json::json!({
                "path": path.to_string_lossy(),
                "skeleton": skeleton
            });
            println!("{}", serde_json::to_string(&output)?);
        }
    }

    Ok(())
}
