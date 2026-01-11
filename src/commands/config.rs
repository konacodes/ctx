use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::output::OutputFormat;

const CONFIG_PATH: &str = ".ctx/config.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_budget")]
    pub budget: usize,

    #[serde(default)]
    pub languages: Option<Vec<String>>,

    #[serde(default)]
    pub ignore: Option<Vec<String>>,
}

fn default_budget() -> usize {
    2000
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Path::new(CONFIG_PATH);

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).context("Failed to read config file")?;

        toml::from_str(&content).context("Failed to parse config file")
    }

    pub fn save(&self) -> Result<()> {
        let path = Path::new(CONFIG_PATH);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;

        Ok(())
    }
}

pub fn run_get(key: &str, format: OutputFormat) -> Result<()> {
    let config = Config::load()?;

    let value = match key {
        "budget" => Some(config.budget.to_string()),
        "languages" => config.languages.map(|l| l.join(", ")),
        "ignore" => config.ignore.map(|i| i.join(", ")),
        _ => {
            anyhow::bail!("Unknown config key: {}", key);
        }
    };

    match format {
        OutputFormat::Human => {
            if let Some(v) = value {
                println!("{}", v);
            } else {
                println!("(not set)");
            }
        }
        OutputFormat::Json | OutputFormat::Compact => {
            let output = serde_json::json!({
                "key": key,
                "value": value
            });
            println!("{}", serde_json::to_string(&output)?);
        }
    }

    Ok(())
}

pub fn run_set(key: &str, value: &str, format: OutputFormat) -> Result<()> {
    let mut config = Config::load()?;

    match key {
        "budget" => {
            config.budget = value.parse().context("Invalid budget value")?;
        }
        "languages" => {
            config.languages = Some(value.split(',').map(|s| s.trim().to_string()).collect());
        }
        "ignore" => {
            config.ignore = Some(value.split(',').map(|s| s.trim().to_string()).collect());
        }
        _ => {
            anyhow::bail!("Unknown config key: {}", key);
        }
    }

    config.save()?;

    match format {
        OutputFormat::Human => {
            println!("Set {} = {}", key, value);
        }
        OutputFormat::Json | OutputFormat::Compact => {
            let output = serde_json::json!({
                "status": "updated",
                "key": key,
                "value": value
            });
            println!("{}", serde_json::to_string(&output)?);
        }
    }

    Ok(())
}

pub fn run_list(format: OutputFormat) -> Result<()> {
    let config = Config::load()?;

    match format {
        OutputFormat::Human => {
            println!("budget = {}", config.budget);
            if let Some(languages) = &config.languages {
                println!("languages = {}", languages.join(", "));
            }
            if let Some(ignore) = &config.ignore {
                println!("ignore = {}", ignore.join(", "));
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        OutputFormat::Compact => {
            println!("{}", serde_json::to_string(&config)?);
        }
    }

    Ok(())
}
