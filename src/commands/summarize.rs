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

/// Result of summarizing a single path (either file or directory)
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SummaryResult {
    File(FileSummary),
    Directory(DirectorySummary),
    Skeleton { path: String, skeleton: String },
}

pub fn run(
    paths: &[String],
    depth: Option<usize>,
    skeleton: bool,
    format: OutputFormat,
) -> Result<()> {
    let mut results: Vec<SummaryResult> = Vec::new();
    let mut first = true;

    for path in paths {
        let target = Path::new(path);

        if !target.exists() {
            anyhow::bail!("Path does not exist: {}", path);
        }

        if target.is_file() {
            if skeleton {
                let skeleton_result = get_skeleton_result(target)?;
                results.push(skeleton_result);
            } else {
                let summary = summarize_file(target)?;
                results.push(SummaryResult::File(summary));
            }
        } else {
            let summary = summarize_directory(target, depth.unwrap_or(1))?;
            results.push(SummaryResult::Directory(summary));
        }
    }

    // Output based on format
    match format {
        OutputFormat::Human => {
            for result in &results {
                if !first {
                    println!("\n{}", "=".repeat(60));
                    println!();
                }
                first = false;
                match result {
                    SummaryResult::File(summary) => println!("{}", summary),
                    SummaryResult::Directory(summary) => println!("{}", summary),
                    SummaryResult::Skeleton { path, skeleton } => {
                        println!("{}:", path);
                        println!("{}", skeleton);
                    }
                }
            }
        }
        OutputFormat::Json => {
            if results.len() == 1 {
                println!("{}", serde_json::to_string_pretty(&results[0])?);
            } else {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
        }
        OutputFormat::Compact => {
            if results.len() == 1 {
                println!("{}", serde_json::to_string(&results[0])?);
            } else {
                println!("{}", serde_json::to_string(&results)?);
            }
        }
    }

    Ok(())
}

fn get_skeleton_result(path: &Path) -> Result<SummaryResult> {
    let source = std::fs::read_to_string(path).context("Failed to read file")?;
    let lang =
        SupportedLanguage::from_path(path).ok_or_else(|| anyhow::anyhow!("Unsupported language"))?;

    let tree =
        treesitter::parse_file(path, &source)?.ok_or_else(|| anyhow::anyhow!("Failed to parse"))?;

    let skeleton = symbols::get_skeleton(&tree, &source, &lang);

    Ok(SummaryResult::Skeleton {
        path: path.to_string_lossy().to_string(),
        skeleton,
    })
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

