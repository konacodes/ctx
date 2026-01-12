use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeZone, Utc};
use git2::{DiffOptions, Repository, StatusOptions};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// Represents the current status of a git repository's working tree.
///
/// This struct provides a snapshot of the repository state, including
/// branch information and categorized lists of changed files.
///
/// # Fields
/// * `branch` - Current branch name (or "HEAD" if detached)
/// * `is_dirty` - Whether there are uncommitted changes
/// * `staged_files` - Files added to the index (ready to commit)
/// * `modified_files` - Files modified in the working directory
/// * `untracked_files` - Files not yet tracked by git
#[derive(Debug, Serialize)]
pub struct GitStatus {
    /// The name of the current branch, or "HEAD" if in detached HEAD state.
    pub branch: String,
    /// True if there are staged or modified files (uncommitted changes).
    pub is_dirty: bool,
    /// Paths of files staged in the index (new, modified, or deleted).
    pub staged_files: Vec<String>,
    /// Paths of files modified in the working directory but not yet staged.
    pub modified_files: Vec<String>,
    /// Paths of files not tracked by git.
    pub untracked_files: Vec<String>,
}

/// Represents a single commit from the repository history.
///
/// Contains key metadata about a commit for display purposes,
/// including abbreviated SHA, first line of message, and timing information.
///
/// # Fields
/// * `sha` - Abbreviated (7-character) commit SHA
/// * `message` - First line of the commit message
/// * `author` - Name of the commit author
/// * `time` - Formatted timestamp (YYYY-MM-DD HH:MM)
/// * `time_ago` - Human-readable relative time (e.g., "2h ago")
#[derive(Debug, Serialize)]
pub struct RecentCommit {
    /// Abbreviated commit SHA (first 7 characters).
    pub sha: String,
    /// First line of the commit message (subject line).
    pub message: String,
    /// Name of the commit author.
    pub author: String,
    /// Formatted local timestamp (YYYY-MM-DD HH:MM).
    pub time: String,
    /// Human-readable relative time (e.g., "2h ago", "3d ago").
    pub time_ago: String,
}

/// Represents activity metrics for a single file in the repository.
///
/// Tracks how frequently a file has been modified in recent commits,
/// useful for identifying actively developed or "hot" files in a codebase.
///
/// # Fields
/// * `path` - File path relative to repository root
/// * `commit_count` - Number of commits that touched this file
/// * `last_modified` - Human-readable time since last modification
/// * `last_author` - Name of the last person to modify this file
#[derive(Debug, Serialize)]
pub struct FileActivity {
    /// File path relative to the repository root.
    pub path: String,
    /// Number of commits that modified this file (within the analyzed range).
    pub commit_count: usize,
    /// Human-readable time since last modification (e.g., "2h ago").
    pub last_modified: String,
    /// Name of the author who last modified this file.
    pub last_author: String,
}

/// Represents a directory with high recent commit activity.
///
/// Identifies directories where active development is occurring,
/// based on the number of file changes within that directory
/// over a specified time period.
///
/// # Fields
/// * `path` - Directory path relative to repository root
/// * `commit_count` - Total number of file changes in this directory
#[derive(Debug, Serialize)]
pub struct HotDirectory {
    /// Directory path relative to the repository root.
    /// Root-level files are represented as ".".
    pub path: String,
    /// Total count of file modifications within this directory.
    pub commit_count: usize,
}

/// Discovers and opens a git repository starting from the given path.
///
/// Searches upward from the given path to find a `.git` directory,
/// similar to how git itself locates repositories.
///
/// # Arguments
/// * `path` - Starting path for repository discovery (can be any subdirectory)
///
/// # Returns
/// * `Ok(Repository)` - The discovered git repository
/// * `Err` - If no git repository is found in the path hierarchy
///
/// # Example
/// ```ignore
/// let repo = find_repo(Path::new("/home/user/myproject/src"))?;
/// // Finds /home/user/myproject/.git
/// ```
pub fn find_repo(path: &Path) -> Result<Repository> {
    Repository::discover(path).context("Not a git repository")
}

/// Retrieves the current status of the repository's working tree.
///
/// Collects information about the current branch and categorizes all
/// changed files into staged, modified, and untracked groups.
///
/// # Arguments
/// * `repo` - Reference to an open git repository
///
/// # Returns
/// A [`GitStatus`] struct containing branch name and categorized file lists.
///
/// # Example
/// ```ignore
/// let repo = find_repo(Path::new("."))?;
/// let status = get_status(&repo)?;
/// println!("On branch: {}", status.branch);
/// if status.is_dirty {
///     println!("Working directory has changes");
/// }
/// ```
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

/// Retrieves the most recent commits from the repository history.
///
/// Walks the commit history starting from HEAD and collects metadata
/// about each commit up to the specified count.
///
/// # Arguments
/// * `repo` - Reference to an open git repository
/// * `count` - Maximum number of commits to retrieve
///
/// # Returns
/// A vector of [`RecentCommit`] structs ordered from newest to oldest.
///
/// # Example
/// ```ignore
/// let commits = get_recent_commits(&repo, 10)?;
/// for commit in commits {
///     println!("{} {} - {}", commit.sha, commit.time_ago, commit.message);
/// }
/// ```
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

/// Analyzes recent commit history to find the most actively modified files.
///
/// Scans the last 100 commits and aggregates file modification statistics,
/// returning the files with the highest commit counts.
///
/// # Arguments
/// * `repo` - Reference to an open git repository
/// * `count` - Maximum number of files to return
///
/// # Returns
/// A vector of [`FileActivity`] structs sorted by commit count (descending).
/// Each entry includes the file path, number of commits, last modification
/// time, and last author.
///
/// # Use Cases
/// - Identifying hot spots in the codebase
/// - Finding files that may need code review attention
/// - Understanding which files change together frequently
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

/// Identifies directories with the most commit activity within a time window.
///
/// Analyzes commits within the specified number of days and counts
/// file modifications per directory to find where active development
/// is concentrated.
///
/// # Arguments
/// * `repo` - Reference to an open git repository
/// * `days` - Number of days to look back from now
///
/// # Returns
/// A vector of up to 10 [`HotDirectory`] structs sorted by commit count
/// (descending). Root-level files are grouped under ".".
///
/// # Example
/// ```ignore
/// let hot_dirs = get_hot_directories(&repo, 7)?; // Last week
/// for dir in hot_dirs {
///     println!("{}: {} changes", dir.path, dir.commit_count);
/// }
/// ```
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

/// Gets a summary of uncommitted changes as insertion/deletion counts.
///
/// Computes the total number of lines added and removed across all
/// uncommitted changes (both staged and unstaged) compared to HEAD.
///
/// # Arguments
/// * `repo` - Reference to an open git repository
///
/// # Returns
/// A tuple of `(insertions, deletions)` representing the total line counts.
///
/// # Example
/// ```ignore
/// let (added, removed) = get_diff_summary(&repo)?;
/// println!("+{} -{} lines", added, removed);
/// ```
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

/// Finds files that frequently change together with a given file.
///
/// Analyzes the last 500 commits to identify files that are commonly
/// modified in the same commits as the target file. This is useful for
/// understanding file relationships and dependencies.
///
/// # Arguments
/// * `repo` - Reference to an open git repository
/// * `file_path` - Path of the file to analyze (relative to repo root)
/// * `limit` - Maximum number of co-changed files to return
///
/// # Returns
/// A vector of tuples `(file_path, count)` sorted by count descending,
/// where count is the number of commits where both files were modified.
///
/// # Use Cases
/// - Finding related files during code review
/// - Understanding implicit dependencies
/// - Identifying files that should be tested together
///
/// # Example
/// ```ignore
/// let related = get_files_changed_with(&repo, "src/lib.rs", 5)?;
/// for (path, count) in related {
///     println!("{} changed together {} times", path, count);
/// }
/// ```
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
