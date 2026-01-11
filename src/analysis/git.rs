use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeZone, Utc};
use git2::{DiffOptions, Repository, StatusOptions};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub is_dirty: bool,
    pub staged_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub untracked_files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RecentCommit {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub time: String,
    pub time_ago: String,
}

#[derive(Debug, Serialize)]
pub struct FileActivity {
    pub path: String,
    pub commit_count: usize,
    pub last_modified: String,
    pub last_author: String,
}

#[derive(Debug, Serialize)]
pub struct HotDirectory {
    pub path: String,
    pub commit_count: usize,
}

pub fn find_repo(path: &Path) -> Result<Repository> {
    Repository::discover(path).context("Not a git repository")
}

pub fn get_status(repo: &Repository) -> Result<GitStatus> {
    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .and_then(|h| h.shorthand())
        .unwrap_or("HEAD")
        .to_string();

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    let mut staged_files = Vec::new();
    let mut modified_files = Vec::new();
    let mut untracked_files = Vec::new();

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let status = entry.status();

        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged_files.push(path.clone());
        }
        if status.is_wt_modified() || status.is_wt_deleted() {
            modified_files.push(path.clone());
        }
        if status.is_wt_new() {
            untracked_files.push(path);
        }
    }

    let is_dirty = !staged_files.is_empty() || !modified_files.is_empty();

    Ok(GitStatus {
        branch,
        is_dirty,
        staged_files,
        modified_files,
        untracked_files,
    })
}

pub fn get_recent_commits(repo: &Repository, count: usize) -> Result<Vec<RecentCommit>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut commits = Vec::new();
    let now = Utc::now();

    for oid in revwalk.take(count) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        let time = commit.time();
        let datetime = Utc.timestamp_opt(time.seconds(), 0).unwrap();
        let local: DateTime<Local> = datetime.into();

        let duration = now.signed_duration_since(datetime);
        let time_ago = format_duration(duration);

        commits.push(RecentCommit {
            sha: oid.to_string()[..7].to_string(),
            message: commit
                .message()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string(),
            author: commit.author().name().unwrap_or("unknown").to_string(),
            time: local.format("%Y-%m-%d %H:%M").to_string(),
            time_ago,
        });
    }

    Ok(commits)
}

pub fn get_recent_file_activity(repo: &Repository, count: usize) -> Result<Vec<FileActivity>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut file_commits: HashMap<String, (usize, i64, String)> = HashMap::new();
    let now = Utc::now();

    for oid in revwalk.take(100) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        let tree = commit.tree()?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        let mut diff_opts = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

        let author = commit.author().name().unwrap_or("unknown").to_string();
        let time = commit.time().seconds();

        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    let path_str = path.to_string_lossy().to_string();
                    let entry = file_commits
                        .entry(path_str)
                        .or_insert((0, time, author.clone()));
                    entry.0 += 1;
                    if time > entry.1 {
                        entry.1 = time;
                        entry.2 = author.clone();
                    }
                }
                true
            },
            None,
            None,
            None,
        )?;
    }

    let mut activities: Vec<_> = file_commits
        .into_iter()
        .map(|(path, (commit_count, time, last_author))| {
            let datetime = Utc.timestamp_opt(time, 0).unwrap();
            let duration = now.signed_duration_since(datetime);
            FileActivity {
                path,
                commit_count,
                last_modified: format_duration(duration),
                last_author,
            }
        })
        .collect();

    activities.sort_by(|a, b| b.commit_count.cmp(&a.commit_count));
    activities.truncate(count);

    Ok(activities)
}

pub fn get_hot_directories(repo: &Repository, days: i64) -> Result<Vec<HotDirectory>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let cutoff = Utc::now().timestamp() - (days * 24 * 60 * 60);
    let mut dir_commits: HashMap<String, usize> = HashMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        if commit.time().seconds() < cutoff {
            break;
        }

        let tree = commit.tree()?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        let mut diff_opts = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    if let Some(parent) = path.parent() {
                        let dir = if parent.as_os_str().is_empty() {
                            ".".to_string()
                        } else {
                            parent.to_string_lossy().to_string()
                        };
                        *dir_commits.entry(dir).or_insert(0) += 1;
                    }
                }
                true
            },
            None,
            None,
            None,
        )?;
    }

    let mut hot_dirs: Vec<_> = dir_commits
        .into_iter()
        .map(|(path, commit_count)| HotDirectory { path, commit_count })
        .collect();

    hot_dirs.sort_by(|a, b| b.commit_count.cmp(&a.commit_count));
    hot_dirs.truncate(10);

    Ok(hot_dirs)
}

pub fn get_diff_summary(repo: &Repository) -> Result<(usize, usize)> {
    let head = repo.head()?.peel_to_tree()?;
    let mut diff_opts = DiffOptions::new();

    let diff = repo.diff_tree_to_workdir_with_index(Some(&head), Some(&mut diff_opts))?;

    let stats = diff.stats()?;
    Ok((stats.insertions(), stats.deletions()))
}

fn format_duration(duration: chrono::Duration) -> String {
    let seconds = duration.num_seconds();
    if seconds < 60 {
        format!("{}s ago", seconds)
    } else if seconds < 3600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{}h ago", seconds / 3600)
    } else if seconds < 604800 {
        format!("{}d ago", seconds / 86400)
    } else {
        format!("{}w ago", seconds / 604800)
    }
}

pub fn get_files_changed_with(repo: &Repository, file_path: &str, limit: usize) -> Result<Vec<(String, usize)>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut co_changes: HashMap<String, usize> = HashMap::new();

    for oid in revwalk.take(500) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        let tree = commit.tree()?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        let mut diff_opts = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

        let mut files_in_commit = Vec::new();
        let mut contains_target = false;

        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    let path_str = path.to_string_lossy().to_string();
                    if path_str == file_path {
                        contains_target = true;
                    }
                    files_in_commit.push(path_str);
                }
                true
            },
            None,
            None,
            None,
        )?;

        if contains_target {
            for f in files_in_commit {
                if f != file_path {
                    *co_changes.entry(f).or_insert(0) += 1;
                }
            }
        }
    }

    let mut result: Vec<_> = co_changes.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result.truncate(limit);

    Ok(result)
}
