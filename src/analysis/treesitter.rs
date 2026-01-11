use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::{Language, Parser, Tree};

#[derive(Debug, Clone)]
pub enum SupportedLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
}

impl SupportedLanguage {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "py" => Some(Self::Python),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Self::TypeScript),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_extension)
    }

    pub fn language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::language(),
            Self::Python => tree_sitter_python::language(),
            Self::JavaScript => tree_sitter_javascript::language(),
            Self::TypeScript => tree_sitter_typescript::language_typescript(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
        }
    }
}

pub fn parse_file(path: &Path, source: &str) -> Result<Option<Tree>> {
    let lang = match SupportedLanguage::from_path(path) {
        Some(l) => l,
        None => return Ok(None),
    };

    let mut parser = Parser::new();
    parser
        .set_language(&lang.language())
        .context("Failed to set language")?;

    parser.parse(source, None).context("Failed to parse").map(Some)
}

#[allow(dead_code)]
pub fn create_parser(lang: &SupportedLanguage) -> Result<Parser> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang.language())
        .context("Failed to set language")?;
    Ok(parser)
}

pub fn detect_project_type(path: &Path) -> Option<&'static str> {
    let indicators = [
        ("Cargo.toml", "rust"),
        ("package.json", "javascript"),
        ("pyproject.toml", "python"),
        ("setup.py", "python"),
        ("requirements.txt", "python"),
        ("go.mod", "go"),
        ("pom.xml", "java"),
        ("build.gradle", "java"),
        ("CMakeLists.txt", "c/cpp"),
        ("Makefile", "make"),
    ];

    for (file, project_type) in indicators {
        if path.join(file).exists() {
            return Some(project_type);
        }
    }

    None
}

pub fn detect_project_name(path: &Path) -> Option<String> {
    // Try Cargo.toml
    if let Ok(content) = std::fs::read_to_string(path.join("Cargo.toml")) {
        if let Ok(parsed) = content.parse::<toml::Table>() {
            if let Some(package) = parsed.get("package").and_then(|p| p.as_table()) {
                if let Some(name) = package.get("name").and_then(|n| n.as_str()) {
                    return Some(name.to_string());
                }
            }
        }
    }

    // Try package.json
    if let Ok(content) = std::fs::read_to_string(path.join("package.json")) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(name) = parsed.get("name").and_then(|n| n.as_str()) {
                return Some(name.to_string());
            }
        }
    }

    // Try pyproject.toml
    if let Ok(content) = std::fs::read_to_string(path.join("pyproject.toml")) {
        if let Ok(parsed) = content.parse::<toml::Table>() {
            if let Some(project) = parsed.get("project").and_then(|p| p.as_table()) {
                if let Some(name) = project.get("name").and_then(|n| n.as_str()) {
                    return Some(name.to_string());
                }
            }
        }
    }

    // Fall back to directory name
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
}
