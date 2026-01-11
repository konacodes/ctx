use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[allow(dead_code)]
const CACHE_FILE: &str = ".ctx/cache/summaries.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryCache {
    pub entries: HashMap<String, CacheEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub path: String,
    pub mtime: u64,
    pub summary: FileSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSummary {
    pub symbols: Vec<SymbolSummary>,
    pub imports: Vec<String>,
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSummary {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub signature: Option<String>,
}

#[allow(dead_code)]
impl SummaryCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn load(project_root: &Path) -> Result<Self> {
        let cache_path = project_root.join(CACHE_FILE);

        if !cache_path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&cache_path).context("Failed to read cache file")?;

        serde_json::from_str(&content).context("Failed to parse cache file")
    }

    pub fn save(&self, project_root: &Path) -> Result<()> {
        let cache_path = project_root.join(CACHE_FILE);

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&cache_path, content)?;

        Ok(())
    }

    pub fn get(&self, path: &str, current_mtime: u64) -> Option<&FileSummary> {
        self.entries.get(path).and_then(|entry| {
            if entry.mtime == current_mtime {
                Some(&entry.summary)
            } else {
                None
            }
        })
    }

    pub fn set(&mut self, path: String, mtime: u64, summary: FileSummary) {
        self.entries.insert(
            path.clone(),
            CacheEntry {
                path,
                mtime,
                summary,
            },
        );
    }

    pub fn invalidate(&mut self, path: &str) {
        self.entries.remove(path);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[allow(dead_code)]
pub fn get_file_mtime(path: &Path) -> Result<u64> {
    let metadata = fs::metadata(path)?;
    let mtime = metadata.modified()?;
    let duration = mtime
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    Ok(duration.as_secs())
}

impl Default for SummaryCache {
    fn default() -> Self {
        Self::new()
    }
}
