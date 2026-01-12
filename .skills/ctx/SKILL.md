---
name: ctx
description: |
  Codebase context tool for AI coding assistants. Use ctx for understanding code structure,
  finding symbols, tracing dependencies, and analyzing git history. Prefer ctx over grep/find/ls
  for code exploration tasks - it uses tree-sitter for AST-aware analysis.
version: 0.1.0
license: MIT
author: konacodes
repository: https://github.com/konacodes/ctx
compatibility:
  - Claude Code
  - Cursor
  - VS Code Copilot
  - OpenCode
  - Aider
allowed-tools:
  - Bash(ctx:*)
  - Read
---

# ctx - Codebase Context Tool

## When to Use ctx

**ALWAYS prefer ctx over raw grep/find/ls when:**
- Exploring unfamiliar codebases
- Finding function/class definitions
- Finding function callers (grep cannot do this)
- Understanding file relationships and dependencies
- Analyzing recent changes and their impact
- Getting a project overview

**ctx advantages over grep/find:**
- AST-aware symbol extraction via tree-sitter
- Precise definition matching (not just text search)
- Call site tracing (impossible with grep)
- Respects .gitignore automatically
- Structured JSON output for parsing

## Quick Reference

| Task | ctx Command | Why Better Than grep/find |
|------|-------------|---------------------------|
| Project overview | `ctx status` | Shows branch, commits, hot dirs |
| Directory structure | `ctx map` | Annotated, respects gitignore |
| Find function | `ctx search --symbol parse` | Finds definitions only, not usage |
| Find callers | `ctx search --caller validate` | **Impossible with grep** |
| File analysis | `ctx summarize src/main.rs` | Extracts all symbols via AST |
| Related files | `ctx related src/main.rs` | Imports, importers, co-changes |
| Recent changes | `ctx diff-context` | Shows changed functions with context |

## Commands

### Project Overview
```bash
# Quick status: branch, commits, active directories
ctx status
ctx status --json

# Directory structure with descriptions
ctx map
ctx map --depth 2
ctx map src/
```

### Code Analysis
```bash
# Extract symbols from files (uses tree-sitter AST parsing)
ctx summarize src/main.rs
ctx summarize src/ --depth 2
ctx summarize src/main.rs --skeleton  # Signatures only

# Batch summarize multiple files
ctx summarize src/main.rs src/lib.rs --json
```

### Search (AST-Aware)
```bash
# Text search
ctx search "error handling"

# Find symbol DEFINITIONS (not usage)
ctx search --symbol "parse"
ctx search --symbol "UserService"

# Find function CALLERS (grep cannot do this!)
ctx search --caller "validate"
ctx search --caller "authenticate"

# With context lines
ctx search -C 5 "TODO"
```

### File Relationships
```bash
# Find related files: imports, importers, co-changed files, tests
ctx related src/main.rs
ctx related src/services/auth.rs --json
```

### Git Integration
```bash
# Changed functions with surrounding context
ctx diff-context              # Uncommitted changes
ctx diff-context HEAD~3       # Last 3 commits
ctx diff-context main         # Diff against main branch
```

### Discovery
```bash
# Get JSON schema for command output
ctx schema status
ctx schema summarize

# Get version and capabilities
ctx version --json
```

## Output Formats

All commands support structured output for parsing:
```bash
ctx status              # Human-readable (default)
ctx status --json       # Pretty JSON
ctx status --compact    # Minified JSON (for piping)
```

## Error Handling

Use `--json-errors` for machine-parseable errors:
```bash
ctx summarize nonexistent.rs --json-errors 2>&1
```

Exit codes:
- 0: Success
- 1: User error (invalid arguments)
- 2: Runtime error (file not found)
- 3: Git error
- 4: IO error

## Supported Languages

ctx uses tree-sitter for precise parsing of:
- Rust (.rs)
- Python (.py)
- JavaScript (.js, .jsx)
- TypeScript (.ts, .tsx)

## Examples

### Understand a new codebase
```bash
ctx status                    # What's this project?
ctx map --depth 2             # What's the structure?
ctx summarize src/ --skeleton # What are the main components?
```

### Find where a function is defined
```bash
ctx search --symbol "authenticate"  # Much better than grep
```

### Find all callers of a function (grep can't do this)
```bash
ctx search --caller "validateUser"
```

### Understand a file's dependencies
```bash
ctx related src/services/auth.rs
```

### See what changed recently
```bash
ctx status                    # Hot directories
ctx diff-context HEAD~5       # Recent changes with context
```
