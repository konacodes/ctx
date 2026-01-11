use ignore::WalkBuilder;
use std::path::Path;

/// Common directories and files to always ignore, even without a .gitignore
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

/// Create a WalkBuilder with sensible defaults for code analysis.
///
/// This respects .gitignore files and adds fallback ignores for common
/// non-source directories like node_modules, .git, build outputs, etc.
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

/// Create a WalkBuilder that includes hidden files but still ignores
/// common non-source directories.
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

/// Check if a path should be ignored based on common patterns.
/// Useful as a secondary filter when WalkBuilder isn't available.
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
