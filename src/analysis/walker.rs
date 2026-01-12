use ignore::WalkBuilder;
use std::path::Path;

/// Default patterns for directories and files to ignore during file system traversal.
///
/// These patterns are applied as fallbacks even when no `.gitignore` file is present,
/// ensuring common non-source artifacts are excluded from code analysis.
///
/// # Categories
/// - **Version control**: `.git`, `.svn`, `.hg`, `.bzr`
/// - **Dependencies**: `node_modules`, `vendor`, `bower_components`
/// - **Build outputs**: `target`, `build`, `dist`, `__pycache__`
/// - **IDE/Editor**: `.idea`, `.vscode`, `*.swp`
/// - **OS files**: `.DS_Store`, `Thumbs.db`
/// - **Environment/secrets**: `.env`, `*.pem`, `*.key`
/// - **Temporary files**: `tmp`, `temp`, `*.log`
const DEFAULT_IGNORES: &[&str] = &[
    // Version control
    ".git",
    ".svn",
    ".hg",
    ".bzr",

    // Dependencies
    "node_modules",
    "vendor",
    "bower_components",
    ".pnpm",

    // Build outputs
    "target",
    "build",
    "dist",
    "out",
    "_build",
    ".next",
    ".nuxt",
    ".output",
    "__pycache__",
    "*.pyc",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    "*.egg-info",
    ".eggs",

    // IDE/Editor
    ".idea",
    ".vscode",
    ".vs",
    "*.swp",
    "*.swo",
    "*~",
    ".project",
    ".classpath",
    ".settings",

    // OS files
    ".DS_Store",
    "Thumbs.db",
    "desktop.ini",

    // Package manager caches
    ".npm",
    ".yarn",
    ".pnpm-store",
    ".cache",

    // Environment and secrets
    ".env",
    ".env.local",
    ".env.*.local",
    "*.pem",
    "*.key",

    // Coverage and test outputs
    "coverage",
    ".coverage",
    "htmlcov",
    ".nyc_output",

    // Logs
    "*.log",
    "logs",

    // Temporary files
    "tmp",
    "temp",
    ".tmp",
    ".temp",
];

/// Creates a file system walker configured for source code analysis.
///
/// This function returns a `WalkBuilder` preconfigured with sensible defaults
/// for traversing a codebase. It respects `.gitignore` files and applies
/// additional ignore patterns for common non-source directories.
///
/// # Arguments
/// * `root` - The root directory to start walking from
///
/// # Returns
/// A configured `WalkBuilder` that can be used to iterate over source files.
/// Call `.build()` on the result to get an iterator.
///
/// # Configuration
/// - Respects `.gitignore`, global gitignore, and `.git/info/exclude`
/// - Excludes hidden files and directories by default
/// - Does not follow symbolic links
/// - Applies [`DEFAULT_IGNORES`] patterns as fallback exclusions
///
/// # Example
/// ```ignore
/// let walker = create_walker(Path::new("/path/to/project"));
/// for entry in walker.build() {
///     if let Ok(entry) = entry {
///         println!("{}", entry.path().display());
///     }
/// }
/// ```
///
/// # See Also
/// - [`create_walker_with_hidden`] - Variant that includes hidden files
/// - [`should_ignore`] - Manual ignore checking for paths
pub fn create_walker(root: &Path) -> WalkBuilder {
    let mut builder = WalkBuilder::new(root);

    // Respect .gitignore files
    builder.git_ignore(true);
    builder.git_global(true);
    builder.git_exclude(true);

    // Don't traverse hidden files/directories by default
    // This excludes .git, .env, etc.
    builder.hidden(true);

    // But we do want to follow symlinks for actual source files
    builder.follow_links(false);

    // Add our default ignores as overrides (works even without .gitignore)
    if let Some(overrides) = ignore::overrides::OverrideBuilder::new(root)
        .add("!**/.git/**")
        .ok()
        .and_then(|b| {
            // Add all default ignores
            for pattern in DEFAULT_IGNORES {
                // Convert pattern to a negation (ignore pattern)
                let ignore_pattern = format!("!**/{}", pattern);
                if b.add(&ignore_pattern).is_err() {
                    // Try as a glob pattern
                    let _ = b.add(&format!("!{}", pattern));
                }
            }
            b.build().ok()
        })
    {
        builder.overrides(overrides);
    }

    builder
}

/// Creates a file system walker that includes hidden files.
///
/// Similar to [`create_walker`], but configured to traverse hidden files
/// and directories (those starting with `.`). This is useful when you need
/// to analyze configuration files like `.eslintrc` or `.prettierrc`.
///
/// # Arguments
/// * `root` - The root directory to start walking from
///
/// # Returns
/// A configured `WalkBuilder` that includes hidden files but still excludes
/// version control directories and other non-source artifacts.
///
/// # Configuration
/// - Respects `.gitignore` files
/// - **Includes** hidden files and directories
/// - Explicitly excludes `.git`, `.svn`, `.hg`, `node_modules`, etc.
/// - Does not follow symbolic links
///
/// # Example
/// ```ignore
/// let walker = create_walker_with_hidden(Path::new("/path/to/project"));
/// for entry in walker.build() {
///     // Will include files like .eslintrc, .prettierrc, etc.
/// }
/// ```
#[allow(dead_code)]
pub fn create_walker_with_hidden(root: &Path) -> WalkBuilder {
    let mut builder = WalkBuilder::new(root);

    // Respect .gitignore files
    builder.git_ignore(true);
    builder.git_global(true);
    builder.git_exclude(true);

    // Include hidden files (for finding dotfiles like .env examples)
    builder.hidden(false);

    builder.follow_links(false);

    // Still need to manually exclude .git and other VCS directories
    if let Some(overrides) = ignore::overrides::OverrideBuilder::new(root)
        .add("!**/.git/**")
        .ok()
        .and_then(|b| {
            b.add("!**/.svn/**").ok();
            b.add("!**/.hg/**").ok();
            b.add("!**/node_modules/**").ok();
            b.add("!**/__pycache__/**").ok();
            b.add("!**/target/**").ok();
            b.add("!**/.next/**").ok();
            b.add("!**/.nuxt/**").ok();
            b.build().ok()
        })
    {
        builder.overrides(overrides);
    }

    builder
}

/// Checks if a path should be ignored based on common non-source patterns.
///
/// This function provides a standalone way to check if a path matches
/// common ignore patterns, useful as a secondary filter when `WalkBuilder`
/// is not available or when filtering paths from other sources.
///
/// # Arguments
/// * `path` - The path to check
///
/// # Returns
/// `true` if the path should be ignored, `false` otherwise.
///
/// # Checked Patterns
/// - Paths containing `.git/`
/// - Hidden directories (except `.github`, `.gitlab`, `.circleci`)
/// - Known non-source directories (`node_modules`, `target`, `build`, etc.)
/// - Compiled/temporary file extensions (`.pyc`, `.swp`, `.log`)
///
/// # Example
/// ```ignore
/// assert!(should_ignore(Path::new("node_modules/package/index.js")));
/// assert!(should_ignore(Path::new(".git/config")));
/// assert!(!should_ignore(Path::new("src/main.rs")));
/// assert!(!should_ignore(Path::new(".github/workflows/ci.yml")));
/// ```
#[allow(dead_code)]
pub fn should_ignore(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Check for .git directory anywhere in path
    if path_str.contains("/.git/") || path_str.contains("\\.git\\") {
        return true;
    }

    // Check path components
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            let name_str = name.to_string_lossy();

            // Skip hidden directories (except for specific allowed ones)
            if name_str.starts_with('.') {
                match name_str.as_ref() {
                    ".github" | ".gitlab" | ".circleci" => continue,
                    _ => return true,
                }
            }

            // Check against known ignore patterns
            match name_str.as_ref() {
                "node_modules" | "vendor" | "bower_components" |
                "__pycache__" | ".pytest_cache" | ".mypy_cache" |
                "target" | "build" | "dist" | "out" | "_build" |
                ".next" | ".nuxt" | ".output" |
                "coverage" | ".nyc_output" |
                ".idea" | ".vscode" | ".vs" => return true,
                _ => {}
            }
        }
    }

    // Check file extension
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy();
        match ext_str.as_ref() {
            "pyc" | "pyo" | "swp" | "swo" | "log" => return true,
            _ => {}
        }
    }

    false
}
