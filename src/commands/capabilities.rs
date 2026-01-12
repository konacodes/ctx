//! Capabilities output for AI agent discovery.
//!
//! This module provides comprehensive tool discovery information
//! in a format suitable for AI agents to understand ctx's capabilities.

use anyhow::Result;
use serde::Serialize;
use serde_json::json;

/// Full capabilities report for AI agent discovery.
#[derive(Debug, Serialize)]
pub struct Capabilities {
    /// Tool name
    pub name: String,
    /// Tool version
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Repository URL
    pub repository: String,
    /// License
    pub license: String,
    /// Integration capabilities
    pub integrations: Integrations,
    /// Available tools/commands
    pub tools: Vec<ToolDefinition>,
    /// Supported output formats
    pub output_formats: Vec<String>,
    /// Exit codes for error handling
    pub exit_codes: ExitCodes,
}

#[derive(Debug, Serialize)]
pub struct Integrations {
    /// Agent Skills support
    pub agent_skills: bool,
    /// MCP server available
    pub mcp_server: bool,
    /// Structured JSON output
    pub structured_output: bool,
    /// JSON error output
    pub json_errors: bool,
    /// JSON schema for outputs
    pub json_schemas: bool,
}

#[derive(Debug, Serialize)]
pub struct ExitCodes {
    pub success: i32,
    pub user_error: i32,
    pub runtime_error: i32,
    pub git_error: i32,
    pub io_error: i32,
}

#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    /// Command name
    pub name: String,
    /// Description
    pub description: String,
    /// Usage example
    pub usage: String,
    /// When to use this tool
    pub when_to_use: String,
    /// Input schema
    pub input_schema: serde_json::Value,
}

/// Run the capabilities output.
pub fn run() -> Result<()> {
    let capabilities = Capabilities {
        name: "ctx".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Context tool for AI coding agents. Provides AST-aware codebase analysis using tree-sitter.".to_string(),
        repository: "https://github.com/konacodes/ctx".to_string(),
        license: "MIT".to_string(),
        integrations: Integrations {
            agent_skills: true,
            mcp_server: true,
            structured_output: true,
            json_errors: true,
            json_schemas: true,
        },
        output_formats: vec![
            "human".to_string(),
            "json".to_string(),
            "compact".to_string(),
        ],
        exit_codes: ExitCodes {
            success: 0,
            user_error: 1,
            runtime_error: 2,
            git_error: 3,
            io_error: 4,
        },
        tools: vec![
            ToolDefinition {
                name: "status".to_string(),
                description: "Project status overview including git branch, recent commits, and hot directories".to_string(),
                usage: "ctx status [--json]".to_string(),
                when_to_use: "When first exploring a codebase or checking project state".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            ToolDefinition {
                name: "map".to_string(),
                description: "Show project directory structure with file counts. Better than ls/find.".to_string(),
                usage: "ctx map [path] [--depth N] [--json]".to_string(),
                when_to_use: "When understanding codebase layout and directory structure".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Path to map"},
                        "depth": {"type": "integer", "description": "Maximum depth"}
                    },
                    "required": []
                }),
            },
            ToolDefinition {
                name: "summarize".to_string(),
                description: "Extract symbols (functions, classes, etc.) from files using tree-sitter AST parsing".to_string(),
                usage: "ctx summarize <paths...> [--skeleton] [--depth N] [--json]".to_string(),
                when_to_use: "When you need to understand file structure without reading entire files".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "paths": {"type": "array", "items": {"type": "string"}, "description": "Files/directories to summarize"},
                        "skeleton": {"type": "boolean", "description": "Show only signatures"},
                        "depth": {"type": "integer", "description": "Max depth for directories"}
                    },
                    "required": ["paths"]
                }),
            },
            ToolDefinition {
                name: "search".to_string(),
                description: "Search codebase for text, symbol definitions (--symbol), or function callers (--caller). The --caller flag uses AST analysis and is impossible with grep.".to_string(),
                usage: "ctx search <query> [--symbol] [--caller] [-C N] [--json]".to_string(),
                when_to_use: "When finding where something is defined (--symbol) or who calls a function (--caller)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "symbol": {"type": "boolean", "description": "Find symbol definitions only"},
                        "caller": {"type": "boolean", "description": "Find function callers (AST-based)"},
                        "context": {"type": "integer", "description": "Lines of context"}
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "related".to_string(),
                description: "Find files related to a given file through imports, reverse imports, git co-changes, and test associations".to_string(),
                usage: "ctx related <file> [--json]".to_string(),
                when_to_use: "When understanding file dependencies and what else might be affected by changes".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "file": {"type": "string", "description": "File to find relations for"}
                    },
                    "required": ["file"]
                }),
            },
            ToolDefinition {
                name: "diff-context".to_string(),
                description: "Show git diff with expanded function context. Better than raw git diff.".to_string(),
                usage: "ctx diff-context [ref] [--json]".to_string(),
                when_to_use: "When reviewing changes and understanding what functions were modified".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "git_ref": {"type": "string", "description": "Git ref to diff against"}
                    },
                    "required": []
                }),
            },
            ToolDefinition {
                name: "schema".to_string(),
                description: "Get JSON schema for a command's output format".to_string(),
                usage: "ctx schema <command>".to_string(),
                when_to_use: "When you need to understand the structure of JSON output".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "Command name"}
                    },
                    "required": ["command"]
                }),
            },
            ToolDefinition {
                name: "version".to_string(),
                description: "Show version and supported languages/features".to_string(),
                usage: "ctx version [--json]".to_string(),
                when_to_use: "When checking ctx capabilities".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        ],
    };

    println!("{}", serde_json::to_string_pretty(&capabilities)?);
    Ok(())
}
