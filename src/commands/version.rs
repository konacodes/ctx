use anyhow::Result;
use serde::Serialize;

use crate::output::OutputFormat;

#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub features: Features,
    pub capabilities: Capabilities,
}

#[derive(Debug, Serialize)]
pub struct Features {
    pub languages: Vec<String>,
    pub output_formats: Vec<String>,
    pub commands: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Capabilities {
    pub tree_sitter: bool,
    pub git_integration: bool,
    pub token_estimation: bool,
}

impl std::fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ctx {}", self.version)?;
        writeln!(f)?;
        writeln!(f, "Languages: {}", self.features.languages.join(", "))?;
        writeln!(f, "Output formats: {}", self.features.output_formats.join(", "))?;
        writeln!(f, "Commands: {}", self.features.commands.join(", "))?;
        writeln!(f)?;
        writeln!(f, "Capabilities:")?;
        writeln!(f, "  tree-sitter: {}", if self.capabilities.tree_sitter { "yes" } else { "no" })?;
        writeln!(f, "  git integration: {}", if self.capabilities.git_integration { "yes" } else { "no" })?;
        writeln!(f, "  token estimation: {}", if self.capabilities.token_estimation { "yes" } else { "no" })?;
        Ok(())
    }
}

pub fn run(format: OutputFormat) -> Result<()> {
    let version_info = VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        features: Features {
            languages: vec![
                "rust".to_string(),
                "python".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
            ],
            output_formats: vec![
                "human".to_string(),
                "json".to_string(),
                "compact".to_string(),
            ],
            commands: vec![
                "init".to_string(),
                "status".to_string(),
                "map".to_string(),
                "summarize".to_string(),
                "search".to_string(),
                "related".to_string(),
                "diff-context".to_string(),
                "inject".to_string(),
                "hook-inject".to_string(),
                "config".to_string(),
                "schema".to_string(),
                "version".to_string(),
            ],
        },
        capabilities: Capabilities {
            tree_sitter: true,
            git_integration: true,
            token_estimation: true,
        },
    };

    match format {
        OutputFormat::Human => println!("{}", version_info),
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&version_info)?);
        }
        OutputFormat::Compact => {
            println!("{}", serde_json::to_string(&version_info)?);
        }
    }

    Ok(())
}
