use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::analysis::git;
use crate::analysis::treesitter;
use crate::output::OutputFormat;

#[derive(Debug, Serialize)]
pub struct ProjectStatus {
    pub project_name: Option<String>,
    pub project_type: Option<String>,
    pub branch: String,
    pub is_dirty: bool,
    pub staged_count: usize,
    pub modified_count: usize,
    pub untracked_count: usize,
    pub recent_commits: Vec<git::RecentCommit>,
    pub hot_directories: Vec<git::HotDirectory>,
    pub diff_stats: Option<(usize, usize)>,
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Project info
        if let Some(name) = &self.project_name {
            if let Some(ptype) = &self.project_type {
                writeln!(f, "{} ({})", name.bold(), ptype)?;
            } else {
                writeln!(f, "{}", name.bold())?;
            }
        }

        // Branch and status
        let status_icon = if self.is_dirty { "*" } else { "" };
        writeln!(f, "Branch: {}{}", self.branch.cyan(), status_icon)?;

        // File status
        if self.staged_count > 0 || self.modified_count > 0 || self.untracked_count > 0 {
            let mut status_parts = Vec::new();
            if self.staged_count > 0 {
                status_parts.push(format!("{} staged", self.staged_count));
            }
            if self.modified_count > 0 {
                status_parts.push(format!("{} modified", self.modified_count));
            }
            if self.untracked_count > 0 {
                status_parts.push(format!("{} untracked", self.untracked_count));
            }
            writeln!(f, "Changes: {}", status_parts.join(", "))?;
        }

        // Diff stats
        if let Some((ins, del)) = self.diff_stats {
            if ins > 0 || del > 0 {
                writeln!(
                    f,
                    "Diff: {} {}",
                    format!("+{}", ins).green(),
                    format!("-{}", del).red()
                )?;
            }
        }

        // Recent commits
        if !self.recent_commits.is_empty() {
            writeln!(f, "\n{}", "Recent commits:".dimmed())?;
            for commit in self.recent_commits.iter().take(5) {
                writeln!(
                    f,
                    "  {} {} ({})",
                    commit.sha.yellow(),
                    commit.message,
                    commit.time_ago.dimmed()
                )?;
            }
        }

        // Hot directories
        if !self.hot_directories.is_empty() {
            writeln!(f, "\n{}", "Hot directories (this week):".dimmed())?;
            for dir in self.hot_directories.iter().take(5) {
                writeln!(f, "  {} ({} commits)", dir.path, dir.commit_count)?;
            }
        }

        Ok(())
    }
}

pub fn run(format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = git::find_repo(&cwd)?;

    let git_status = git::get_status(&repo)?;
    let recent_commits = git::get_recent_commits(&repo, 5).unwrap_or_default();
    let hot_directories = git::get_hot_directories(&repo, 7).unwrap_or_default();
    let diff_stats = git::get_diff_summary(&repo).ok();

    let project_name = treesitter::detect_project_name(&cwd);
    let project_type = treesitter::detect_project_type(&cwd).map(|s| s.to_string());

    let status = ProjectStatus {
        project_name,
        project_type,
        branch: git_status.branch,
        is_dirty: git_status.is_dirty,
        staged_count: git_status.staged_files.len(),
        modified_count: git_status.modified_files.len(),
        untracked_count: git_status.untracked_files.len(),
        recent_commits,
        hot_directories,
        diff_stats,
    };

    match format {
        OutputFormat::Human => println!("{}", status),
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        OutputFormat::Compact => {
            println!("{}", serde_json::to_string(&status)?);
        }
    }

    Ok(())
}
