use anyhow::{bail, Result};
use serde::Serialize;

/// Simplified JSON Schema representation for command outputs
#[derive(Debug, Serialize)]
pub struct CommandSchema {
    pub command: String,
    pub description: String,
    pub output_schema: serde_json::Value,
}

pub fn run(command: &str) -> Result<()> {
    let schema = get_schema(command)?;
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}

fn get_schema(command: &str) -> Result<CommandSchema> {
    match command {
        "status" => Ok(CommandSchema {
            command: "status".to_string(),
            description: "Project status overview including git state, recent commits, and hot directories".to_string(),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_name": { "type": "string", "nullable": true },
                    "project_type": { "type": "string", "nullable": true },
                    "branch": { "type": "string" },
                    "is_dirty": { "type": "boolean" },
                    "staged_count": { "type": "integer" },
                    "modified_count": { "type": "integer" },
                    "untracked_count": { "type": "integer" },
                    "recent_commits": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "sha": { "type": "string" },
                                "message": { "type": "string" },
                                "author": { "type": "string" },
                                "time": { "type": "string" },
                                "time_ago": { "type": "string" }
                            }
                        }
                    },
                    "hot_directories": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "commit_count": { "type": "integer" }
                            }
                        }
                    },
                    "diff_stats": {
                        "type": "array",
                        "nullable": true,
                        "items": { "type": "integer" },
                        "description": "[insertions, deletions]"
                    }
                }
            }),
        }),
        "map" => Ok(CommandSchema {
            command: "map".to_string(),
            description: "Project structure with directory and file information".to_string(),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "directories": {
                        "type": "object",
                        "additionalProperties": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "description": { "type": "string", "nullable": true },
                                "files": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "name": { "type": "string" },
                                            "language": { "type": "string", "nullable": true },
                                            "symbols": { "type": "integer" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        }),
        "summarize" => Ok(CommandSchema {
            command: "summarize".to_string(),
            description: "File or directory summary with symbols and imports".to_string(),
            output_schema: serde_json::json!({
                "type": "object",
                "oneOf": [
                    {
                        "description": "File summary",
                        "properties": {
                            "path": { "type": "string" },
                            "language": { "type": "string", "nullable": true },
                            "lines": { "type": "integer" },
                            "symbols": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" },
                                        "kind": { "type": "string", "enum": ["function", "method", "struct", "class", "enum", "interface", "trait", "const", "variable", "type", "module"] },
                                        "line": { "type": "integer" },
                                        "signature": { "type": "string", "nullable": true },
                                        "doc_comment": { "type": "string", "nullable": true }
                                    }
                                }
                            },
                            "imports": {
                                "type": "array",
                                "items": { "type": "string" }
                            }
                        }
                    },
                    {
                        "description": "Directory summary",
                        "properties": {
                            "path": { "type": "string" },
                            "file_count": { "type": "integer" },
                            "files": {
                                "type": "array",
                                "items": { "$ref": "#/oneOf/0" }
                            }
                        }
                    }
                ]
            }),
        }),
        "search" => Ok(CommandSchema {
            command: "search".to_string(),
            description: "Search results from codebase".to_string(),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "results": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "line": { "type": "integer" },
                                "column": { "type": "integer" },
                                "text": { "type": "string" },
                                "context": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            }),
        }),
        "related" => Ok(CommandSchema {
            command: "related".to_string(),
            description: "Files related to a given file through imports, reverse imports, co-changes, and tests".to_string(),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "imports": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "reason": { "type": "string" }
                            }
                        }
                    },
                    "imported_by": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "reason": { "type": "string" }
                            }
                        }
                    },
                    "co_changed": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "reason": { "type": "string" }
                            }
                        }
                    },
                    "test_files": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "reason": { "type": "string" }
                            }
                        }
                    }
                }
            }),
        }),
        "diff-context" => Ok(CommandSchema {
            command: "diff-context".to_string(),
            description: "Diff analysis with context about modified functions and their callers".to_string(),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "ref_name": { "type": "string" },
                    "files_changed": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "insertions": { "type": "integer" },
                                "deletions": { "type": "integer" },
                                "functions_modified": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "name": { "type": "string" },
                                            "kind": { "type": "string" },
                                            "start_line": { "type": "integer" },
                                            "signature": { "type": "string", "nullable": true }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "callers_affected": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "function_modified": { "type": "string" },
                                "called_from": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            }),
        }),
        _ => bail!("Unknown command: {}. Available commands: status, map, summarize, search, related, diff-context", command),
    }
}
