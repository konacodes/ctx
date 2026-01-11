use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::output::OutputFormat;

const DEFAULT_CONFIG: &str = r#"# ctx configuration

# Default token budget for context injection
budget = 2000

# Languages to parse (auto-detect by default)
# languages = ["rust", "python", "javascript", "typescript"]

# Additional patterns to ignore (in addition to .gitignore)
# ignore = ["*.min.js", "vendor/"]
"#;

pub fn run(format: OutputFormat) -> Result<()> {
    let ctx_dir = Path::new(".ctx");

    if ctx_dir.exists() {
        if format == OutputFormat::Json {
            println!(r#"{{"status": "exists", "message": ".ctx directory already exists"}}"#);
        } else {
            println!(".ctx directory already exists");
        }
        return Ok(());
    }

    // Create .ctx directory structure
    fs::create_dir_all(ctx_dir.join("cache")).context("Failed to create .ctx/cache directory")?;

    // Write default config
    fs::write(ctx_dir.join("config.toml"), DEFAULT_CONFIG)
        .context("Failed to write config.toml")?;

    // Add .ctx to .gitignore if not already there
    add_to_gitignore()?;

    if format == OutputFormat::Json {
        println!(r#"{{"status": "created", "path": ".ctx"}}"#);
    } else {
        println!("Initialized .ctx directory");
        println!("  Created .ctx/config.toml");
        println!("  Created .ctx/cache/");
    }

    Ok(())
}

fn add_to_gitignore() -> Result<()> {
    let gitignore_path = Path::new(".gitignore");

    if gitignore_path.exists() {
        let content = fs::read_to_string(gitignore_path)?;
        if content.contains(".ctx") {
            return Ok(());
        }
        let new_content = format!("{}\n# ctx cache directory\n.ctx/\n", content.trim_end());
        fs::write(gitignore_path, new_content)?;
    } else {
        fs::write(gitignore_path, "# ctx cache directory\n.ctx/\n")?;
    }

    Ok(())
}
