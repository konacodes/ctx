//! Integration tests for the ctx CLI tool

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tempfile::TempDir;

// ============================================================================
// Helper Functions
// ============================================================================

/// Creates a temporary directory with test files for integration testing
fn create_temp_test_directory() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create a simple Rust file
    let rust_file = temp_dir.path().join("main.rs");
    fs::write(
        &rust_file,
        r#"//! Main module documentation

fn main() {
    println!("Hello, world!");
    helper_function();
}

fn helper_function() {
    let x = 42;
    println!("The answer is {}", x);
}

pub struct TestStruct {
    pub field: i32,
}

impl TestStruct {
    pub fn new(value: i32) -> Self {
        Self { field: value }
    }
}
"#,
    )
    .expect("Failed to write main.rs");

    // Create a Python file
    let python_file = temp_dir.path().join("script.py");
    fs::write(
        &python_file,
        r#""""A simple Python script for testing."""

import os
import sys

def greet(name: str) -> str:
    """Greet someone by name."""
    return f"Hello, {name}!"

class Calculator:
    """A simple calculator class."""

    def __init__(self):
        self.result = 0

    def add(self, x: int, y: int) -> int:
        """Add two numbers."""
        return x + y

if __name__ == "__main__":
    print(greet("World"))
"#,
    )
    .expect("Failed to write script.py");

    // Create a JavaScript file
    let js_file = temp_dir.path().join("app.js");
    fs::write(
        &js_file,
        r#"/**
 * Main application module
 */

function initialize() {
    console.log("Initializing...");
}

function fetchData(url) {
    return fetch(url).then(r => r.json());
}

class AppController {
    constructor() {
        this.initialized = false;
    }

    start() {
        this.initialized = true;
        initialize();
    }
}

module.exports = { initialize, fetchData, AppController };
"#,
    )
    .expect("Failed to write app.js");

    // Create a subdirectory with files
    let sub_dir = temp_dir.path().join("src");
    fs::create_dir(&sub_dir).expect("Failed to create src directory");

    let lib_file = sub_dir.join("lib.rs");
    fs::write(
        &lib_file,
        r#"//! Library module

pub mod utils;

pub fn lib_function() -> i32 {
    42
}
"#,
    )
    .expect("Failed to write lib.rs");

    // Create a text file that should be searchable
    let readme = temp_dir.path().join("README.md");
    fs::write(
        &readme,
        r#"# Test Project

This is a test project for ctx integration testing.

## Features

- Rust support
- Python support
- JavaScript support
"#,
    )
    .expect("Failed to write README.md");

    temp_dir
}

/// Initialize a git repository in the given directory
fn init_git_repo(dir: &PathBuf) {
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("Failed to run git init");

    // Configure git user for commits
    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .expect("Failed to configure git email");

    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir)
        .output()
        .expect("Failed to configure git name");

    // Add all files and create initial commit
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output()
        .expect("Failed to run git add");

    StdCommand::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(dir)
        .output()
        .expect("Failed to run git commit");
}

/// Get a Command for running ctx
fn ctx_cmd() -> Command {
    Command::cargo_bin("ctx").expect("Failed to find ctx binary")
}

// ============================================================================
// Status Command Tests
// ============================================================================

#[test]
fn test_status_command() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Branch:"));
}

#[test]
fn test_status_json_flag() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("--json")
        .arg("status")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");

    // Check expected fields exist
    assert!(parsed.get("branch").is_some(), "JSON should contain 'branch' field");
    assert!(parsed.get("is_dirty").is_some(), "JSON should contain 'is_dirty' field");
    assert!(parsed.get("staged_count").is_some(), "JSON should contain 'staged_count' field");
    assert!(parsed.get("modified_count").is_some(), "JSON should contain 'modified_count' field");
    assert!(parsed.get("recent_commits").is_some(), "JSON should contain 'recent_commits' field");
}

// ============================================================================
// Map Command Tests
// ============================================================================

#[test]
fn test_map_command() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .arg("map")
        .assert()
        .success();
}

#[test]
fn test_map_command_with_depth() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .args(["map", "--depth", "1"])
        .assert()
        .success();
}

#[test]
fn test_map_json_output() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("--json")
        .arg("map")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Map output should be valid JSON");

    assert!(parsed.get("directories").is_some(), "JSON should contain 'directories' field");
}

// ============================================================================
// Summarize Command Tests
// ============================================================================

#[test]
fn test_summarize_file() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .args(["summarize", "main.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn test_summarize_skeleton() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .args(["summarize", "main.rs", "--skeleton"])
        .assert()
        .success();
}

#[test]
fn test_summarize_json_output() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("--json")
        .args(["summarize", "main.rs"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Summarize output should be valid JSON");

    assert!(parsed.get("path").is_some(), "JSON should contain 'path' field");
    assert!(parsed.get("lines").is_some(), "JSON should contain 'lines' field");
    assert!(parsed.get("symbols").is_some(), "JSON should contain 'symbols' field");
}

#[test]
fn test_summarize_directory() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .args(["summarize", "src"])
        .assert()
        .success();
}

// ============================================================================
// Search Command Tests
// ============================================================================

#[test]
fn test_search_basic() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .args(["search", "function"])
        .assert()
        .success();
}

#[test]
fn test_search_with_context() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .args(["search", "hello", "-C", "3"])
        .assert()
        .success();
}

#[test]
fn test_search_symbol() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .args(["search", "--symbol", "main"])
        .assert()
        .success();
}

#[test]
fn test_search_json_output() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("--json")
        .args(["search", "test"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Search output should be valid JSON");

    assert!(parsed.get("query").is_some(), "JSON should contain 'query' field");
    assert!(parsed.get("results").is_some(), "JSON should contain 'results' field");
}

// ============================================================================
// Init Command Tests
// ============================================================================

#[test]
fn test_init_command() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .ctx directory"));

    // Verify .ctx directory was created
    assert!(temp_dir.path().join(".ctx").exists());
    assert!(temp_dir.path().join(".ctx/config.toml").exists());
    assert!(temp_dir.path().join(".ctx/cache").exists());
}

#[test]
fn test_init_already_exists() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    // Run init twice
    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .success();

    let mut cmd2 = ctx_cmd();
    cmd2.current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("already exists"));
}

#[test]
fn test_init_json_output() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("--json")
        .arg("init")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Init output should be valid JSON");

    assert!(parsed.get("status").is_some(), "JSON should contain 'status' field");
}

// ============================================================================
// JSON Output Format Tests
// ============================================================================

#[test]
fn test_json_output_format() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    // Test --json flag on multiple commands
    let commands = vec![
        vec!["status"],
        vec!["map"],
        vec!["summarize", "main.rs"],
        vec!["search", "fn"],
    ];

    for args in commands {
        let mut cmd = ctx_cmd();
        let output = cmd
            .current_dir(temp_dir.path())
            .arg("--json")
            .args(&args)
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let _: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|e| panic!("Command {:?} should produce valid JSON: {}", args, e));
    }
}

#[test]
fn test_compact_output_format() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("--compact")
        .arg("status")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Compact JSON should be on a single line (no pretty printing)
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 1, "Compact output should be a single line");

    // Should still be valid JSON
    let _: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Compact output should be valid JSON");
}

#[test]
fn test_format_flag_json() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    let output = cmd
        .current_dir(temp_dir.path())
        .args(["--format", "json", "status"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _: serde_json::Value = serde_json::from_str(&stdout)
        .expect("--format json should produce valid JSON");
}

#[test]
fn test_format_flag_compact() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    let output = cmd
        .current_dir(temp_dir.path())
        .args(["--format", "compact", "status"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 1, "--format compact should produce single-line output");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_file_not_found() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .args(["summarize", "nonexistent_file.rs"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_not_git_repo() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create a simple file without git init
    fs::write(temp_dir.path().join("test.txt"), "hello")
        .expect("Failed to write test file");

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn test_invalid_subcommand() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .arg("invalid_command")
        .assert()
        .failure();
}

#[test]
fn test_missing_required_argument() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    // summarize requires a path argument
    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .arg("summarize")
        .assert()
        .failure();

    // search requires a query argument
    let mut cmd2 = ctx_cmd();
    cmd2.current_dir(temp_dir.path())
        .arg("search")
        .assert()
        .failure();
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Initialize git in empty directory
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .arg("map")
        .assert()
        .success();
}

#[test]
fn test_search_no_results() {
    let temp_dir = create_temp_test_directory();
    init_git_repo(&temp_dir.path().to_path_buf());

    let mut cmd = ctx_cmd();
    cmd.current_dir(temp_dir.path())
        .args(["search", "zzzyyyxxx_nonexistent_pattern_12345"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No results found"));
}

#[test]
fn test_help_flag() {
    let mut cmd = ctx_cmd();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Context tool for coding agents"));
}

#[test]
fn test_version_flag() {
    let mut cmd = ctx_cmd();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("ctx"));
}

#[test]
fn test_subcommand_help() {
    let subcommands = vec!["status", "map", "summarize", "search", "init"];

    for subcmd in subcommands {
        let mut cmd = ctx_cmd();
        cmd.args([subcmd, "--help"])
            .assert()
            .success();
    }
}
