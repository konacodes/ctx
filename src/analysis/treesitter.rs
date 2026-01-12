use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::{Language, Parser, Tree};

/// Enumeration of programming languages supported for tree-sitter parsing.
///
/// This enum represents the languages that ctx can parse and analyze
/// using tree-sitter grammars. Each variant corresponds to a specific
/// language grammar that enables syntax-aware code analysis.
///
/// # Supported Languages
/// - **Rust**: `.rs` files
/// - **Python**: `.py` files
/// - **JavaScript**: `.js`, `.jsx`, `.mjs`, `.cjs` files
/// - **TypeScript**: `.ts`, `.tsx`, `.mts`, `.cts` files
#[derive(Debug, Clone)]
pub enum SupportedLanguage {
    /// Rust programming language (`.rs` extension).
    Rust,
    /// Python programming language (`.py` extension).
    Python,
    /// JavaScript language including JSX (`.js`, `.jsx`, `.mjs`, `.cjs` extensions).
    JavaScript,
    /// TypeScript language including TSX (`.ts`, `.tsx`, `.mts`, `.cts` extensions).
    TypeScript,
}

impl SupportedLanguage {
    /// Creates a SupportedLanguage from a file extension string.
    ///
    /// # Arguments
    /// * `ext` - File extension without the leading dot (e.g., "rs", "py")
    ///
    /// # Returns
    /// `Some(SupportedLanguage)` if the extension is recognized, `None` otherwise.
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(SupportedLanguage::from_extension("rs"), Some(SupportedLanguage::Rust));
    /// assert_eq!(SupportedLanguage::from_extension("unknown"), None);
    /// ```
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "py" => Some(Self::Python),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Self::TypeScript),
            _ => None,
        }
    }

    /// Creates a SupportedLanguage by extracting and matching the file extension.
    ///
    /// # Arguments
    /// * `path` - Path to a source file
    ///
    /// # Returns
    /// `Some(SupportedLanguage)` if the file has a recognized extension, `None` otherwise.
    ///
    /// # Example
    /// ```ignore
    /// let lang = SupportedLanguage::from_path(Path::new("src/main.rs"));
    /// assert_eq!(lang, Some(SupportedLanguage::Rust));
    /// ```
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_extension)
    }

    /// Returns the tree-sitter Language grammar for this language.
    ///
    /// This is used internally to configure the tree-sitter parser
    /// for the appropriate language grammar.
    ///
    /// # Returns
    /// The tree-sitter `Language` object for parsing this language.
    pub fn language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::language(),
            Self::Python => tree_sitter_python::language(),
            Self::JavaScript => tree_sitter_javascript::language(),
            Self::TypeScript => tree_sitter_typescript::language_typescript(),
        }
    }

    /// Returns the lowercase string name of the language.
    ///
    /// Useful for display purposes and serialization.
    ///
    /// # Returns
    /// A static string: "rust", "python", "javascript", or "typescript".
    pub fn name(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
        }
    }
}

/// Parses source code into a tree-sitter syntax tree.
///
/// Automatically detects the language from the file path's extension
/// and configures the appropriate parser. Returns `None` for unsupported
/// file types rather than failing.
///
/// # Arguments
/// * `path` - Path to the source file (used for language detection)
/// * `source` - The source code content to parse
///
/// # Returns
/// * `Ok(Some(Tree))` - Successfully parsed syntax tree
/// * `Ok(None)` - File type is not supported for parsing
/// * `Err` - Parsing failed for a supported file type
///
/// # Example
/// ```ignore
/// let source = std::fs::read_to_string("src/main.rs")?;
/// if let Some(tree) = parse_file(Path::new("src/main.rs"), &source)? {
///     // Use the syntax tree
/// }
/// ```
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

/// Creates a configured tree-sitter parser for a specific language.
///
/// This function creates a new parser instance and configures it with
/// the appropriate language grammar. Useful when you need to parse
/// multiple files of the same language.
///
/// # Arguments
/// * `lang` - The target language for the parser
///
/// # Returns
/// A configured `Parser` ready to parse source code of the specified language.
///
/// # Example
/// ```ignore
/// let parser = create_parser(&SupportedLanguage::Rust)?;
/// let tree = parser.parse(source, None)?;
/// ```
#[allow(dead_code)]
pub fn create_parser(lang: &SupportedLanguage) -> Result<Parser> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang.language())
        .context("Failed to set language")?;
    Ok(parser)
}

/// Detects the primary programming language/framework of a project.
///
/// Examines the project directory for common configuration files that
/// indicate the project type (e.g., `Cargo.toml` for Rust, `package.json`
/// for JavaScript).
///
/// # Arguments
/// * `path` - Path to the project root directory
///
/// # Returns
/// A static string identifying the project type, or `None` if no
/// recognized configuration files are found.
///
/// # Recognized Project Types
/// - "rust" - `Cargo.toml`
/// - "javascript" - `package.json`
/// - "python" - `pyproject.toml`, `setup.py`, `requirements.txt`
/// - "go" - `go.mod`
/// - "java" - `pom.xml`, `build.gradle`
/// - "c/cpp" - `CMakeLists.txt`
/// - "make" - `Makefile`
///
/// # Example
/// ```ignore
/// let project_type = detect_project_type(Path::new("/home/user/myproject"));
/// // Returns Some("rust") if Cargo.toml exists
/// ```
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

/// Detects the project name from configuration files.
///
/// Attempts to extract the project name by reading common configuration
/// files in order of priority:
/// 1. `Cargo.toml` - `[package].name`
/// 2. `package.json` - `"name"` field
/// 3. `pyproject.toml` - `[project].name`
/// 4. Falls back to the directory name
///
/// # Arguments
/// * `path` - Path to the project root directory
///
/// # Returns
/// The project name as a `String`, or `None` only if even the directory
/// name cannot be determined (rare edge case).
///
/// # Example
/// ```ignore
/// let name = detect_project_name(Path::new("/home/user/my-rust-project"));
/// // Returns Some("my-rust-project") from Cargo.toml or directory name
/// ```
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
