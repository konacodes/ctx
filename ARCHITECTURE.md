# ctx Architecture

## Overview

`ctx` is a context tool designed specifically for AI coding assistants. It provides intelligent codebase exploration capabilities that generate structured, token-efficient context for large language models.

### Target Users

- **AI Coding Assistants**: Claude Code, Aider, Cursor, GitHub Copilot, and similar tools that need to understand codebases
- **Developers**: Engineers who want quick, structured insights into their projects without reading entire files

### Key Value Propositions

1. **AST-Based Analysis**: Uses tree-sitter for accurate symbol extraction, not regex-based text matching
2. **Token-Efficient Output**: Designed to minimize token usage while maximizing useful context
3. **Git-Aware**: Leverages git history for relevance scoring and relationship discovery
4. **Respects Boundaries**: Automatically excludes build artifacts, dependencies, and non-source files

## Project Structure

```
src/
├── main.rs              # CLI entry point and command routing
├── analysis/            # Core analysis modules
│   ├── mod.rs
│   ├── git.rs           # Git repository operations
│   ├── symbols.rs       # Symbol extraction from AST
│   ├── treesitter.rs    # Tree-sitter parser management
│   ├── relevance.rs     # File relevance scoring
│   └── walker.rs        # File system traversal with ignores
├── commands/            # CLI command implementations
│   ├── mod.rs
│   ├── config.rs        # Configuration management
│   ├── context_builder.rs # Shared context building logic
│   ├── diff_context.rs  # Diff with expanded context
│   ├── hook_inject.rs   # Claude Code hook handler
│   ├── init.rs          # Project initialization
│   ├── inject.rs        # Context injection into prompts
│   ├── map.rs           # Directory structure mapping
│   ├── related.rs       # Related file discovery
│   ├── search.rs        # Code search (text, symbol, caller)
│   ├── status.rs        # Project status overview
│   └── summarize.rs     # File/directory summarization
├── cache/               # Caching functionality
│   ├── mod.rs
│   └── summaries.rs     # File summary caching
└── output/              # Output formatting
    └── mod.rs           # Human/JSON/Compact formatters
```

### Module Responsibilities

#### `analysis/`

- **git.rs**: Interfaces with libgit2 for repository operations - status, commits, file activity, hot directories, and co-change analysis
- **symbols.rs**: Extracts symbols (functions, classes, structs, etc.) from tree-sitter AST nodes; handles Rust, Python, JavaScript, and TypeScript
- **treesitter.rs**: Manages tree-sitter parsers, language detection, and project type identification
- **relevance.rs**: Scores files for relevance based on prompt keywords, file paths, and git history
- **walker.rs**: Creates file iterators that respect `.gitignore` and exclude common non-source directories

#### `commands/`

Each command module implements a specific CLI command:

- **status**: Shows branch, commits, hot directories, diff stats
- **map**: Generates annotated directory tree
- **summarize**: Extracts symbols and imports from files
- **search**: Finds text matches, symbol definitions, or function callers
- **related**: Discovers imports, importers, co-changed files, and tests
- **diff-context**: Shows diffs with full function context
- **inject**: Injects computed context into prompts
- **hook-inject**: Handles Claude Code hook protocol

#### `cache/`

- **summaries.rs**: Stores parsed file summaries with mtime-based invalidation

#### `output/`

- **mod.rs**: Three output modes - Human (colored text), JSON (pretty), Compact (minified JSON)

## Core Concepts

### Context Injection

Context injection adds structured metadata to prompts before they reach the LLM. The `inject` and `hook-inject` commands:

1. Analyze the prompt for mentioned files and keywords
2. Query git for recent activity and relevant files
3. Score candidate files for relevance
4. Build a context string within a token budget

Example output:
```
[CTX: project=ctx lang=rust branch=main]
[RECENT: src/main.rs modified 2h ago]
[RELEVANT: src/analysis/symbols.rs (filename mentioned)]
[KEYWORDS: parser, tree-sitter, symbols]
```

### Symbol Extraction via Tree-sitter

Tree-sitter provides concrete syntax trees for accurate symbol extraction. The `symbols.rs` module walks the AST to find:

| Language   | Extracted Symbols |
|------------|-------------------|
| Rust       | Functions, methods, structs, enums, traits, consts, types, modules |
| Python     | Functions, methods, classes |
| JavaScript/TypeScript | Functions, methods, classes, interfaces, types, variables |

Each symbol includes:
- Name
- Kind (function, class, method, etc.)
- Line number
- Signature (for functions/methods)
- Doc comment (if present)

### Relevance Scoring

The `relevance.rs` module assigns scores to files based on:

| Factor | Score Boost |
|--------|-------------|
| Full path mentioned in prompt | +10.0 |
| Filename mentioned in prompt | +5.0 |
| Keyword match in path | +1.0 per match |
| Recent git activity | +0.5 per commit (max 2.5) |
| Relevant file type for keywords | +2.0 |

Files are sorted by score and truncated to fit the token budget.

### Token Budget Management

Context building operates within a configurable token budget (default: 2000). Token estimation uses a simple heuristic:

```rust
fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4  // ~4 chars per token
}
```

Each context line is added only if it fits within the remaining budget.

## Data Flow

### Typical Command Flow

```
User runs: ctx summarize src/main.rs

1. main.rs parses CLI args via clap
2. Dispatches to commands::summarize::run()
3. summarize::run() calls summarize_file()
4. summarize_file():
   a. Reads file content
   b. Detects language via treesitter::SupportedLanguage::from_path()
   c. Parses file via treesitter::parse_file()
   d. Extracts symbols via symbols::extract_symbols()
   e. Extracts imports via symbols::find_imports()
5. Returns FileSummary struct
6. Output formatted based on --format flag (Human/JSON/Compact)
```

### Context Building Flow

```
User runs: echo "fix the parser" | ctx inject --budget 1000

1. Read prompt from stdin
2. Detect project info (name, type, git branch)
3. Query git for recent file activity
4. Extract mentioned files from prompt text
5. Extract keywords from prompt (filtering stop words)
6. Walk codebase (respecting .gitignore)
7. Score all files for relevance to prompt
8. Build context string, adding lines until budget exhausted
9. Output: context + separator + original prompt
```

## Key Design Decisions

### Why Tree-sitter for Parsing

1. **Accuracy**: Real AST parsing vs regex means no false positives/negatives
2. **Incremental**: Tree-sitter supports incremental parsing (future optimization)
3. **Language Support**: Consistent API across languages
4. **Error Recovery**: Handles incomplete/invalid code gracefully

### Why Respecting .gitignore

1. **Performance**: Skipping `node_modules/`, `target/`, etc. is essential for large projects
2. **Relevance**: Build artifacts and dependencies are rarely useful context
3. **User Expectations**: Developers expect their ignore rules to be honored
4. **Fallback Ignores**: Even without `.gitignore`, common directories are excluded via `walker.rs::DEFAULT_IGNORES`

### Output Format Strategy

Three output modes serve different use cases:

- **Human** (`--format human`, default): Colored, readable output for terminal users
- **JSON** (`--json`, `--format json`): Pretty-printed JSON for debugging and inspection
- **Compact** (`--compact`, `--format compact`): Minified JSON for programmatic consumption

All serializable types implement both `Display` (for Human) and `Serialize` (for JSON).

## Adding New Features

### Adding a New Command

1. Create `src/commands/mycommand.rs`:

```rust
use anyhow::Result;
use serde::Serialize;
use crate::output::OutputFormat;

#[derive(Debug, Serialize)]
pub struct MyOutput {
    pub field: String,
}

impl std::fmt::Display for MyOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.field)
    }
}

pub fn run(arg: &str, format: OutputFormat) -> Result<()> {
    let output = MyOutput { field: arg.to_string() };

    match format {
        OutputFormat::Human => println!("{}", output),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&output)?),
        OutputFormat::Compact => println!("{}", serde_json::to_string(&output)?),
    }

    Ok(())
}
```

2. Add to `src/commands/mod.rs`:

```rust
pub mod mycommand;
```

3. Add CLI definition in `src/main.rs`:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands

    /// My new command description
    MyCommand {
        /// Argument description
        arg: String,
    },
}
```

4. Add dispatch in `main.rs::run()`:

```rust
Commands::MyCommand { arg } => {
    commands::mycommand::run(&arg, format)?;
}
```

### Adding Support for a New Language

1. Add the tree-sitter grammar dependency to `Cargo.toml`:

```toml
tree-sitter-go = "0.21"
```

2. Update `src/analysis/treesitter.rs`:

```rust
#[derive(Debug, Clone)]
pub enum SupportedLanguage {
    // ... existing
    Go,
}

impl SupportedLanguage {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            // ... existing
            "go" => Some(Self::Go),
            _ => None,
        }
    }

    pub fn language(&self) -> Language {
        match self {
            // ... existing
            Self::Go => tree_sitter_go::language(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            // ... existing
            Self::Go => "go",
        }
    }
}
```

3. Add symbol extraction in `src/analysis/symbols.rs`:

```rust
pub fn extract_symbols(tree: &Tree, source: &str, lang: &SupportedLanguage) -> Vec<Symbol> {
    match lang {
        // ... existing
        SupportedLanguage::Go => extract_go_symbols(&root, source, &mut symbols),
    }
    symbols
}

fn extract_go_symbols(node: &Node, source: &str, symbols: &mut Vec<Symbol>) {
    // Implement based on Go's tree-sitter grammar node types
    // See: https://github.com/tree-sitter/tree-sitter-go/blob/master/src/node-types.json
}
```

### Extending Relevance Scoring

Add new scoring factors in `src/analysis/relevance.rs`:

```rust
pub fn score_files_for_prompt(...) -> Result<Vec<RelevanceScore>> {
    for path in candidates {
        let mut score = 0.0;
        let mut reasons = Vec::new();

        // ... existing scoring logic

        // Add new factor
        if is_entry_point(path) {
            score += 3.0;
            reasons.push("entry point".to_string());
        }

        // ...
    }
}

fn is_entry_point(path: &str) -> bool {
    let entry_points = ["main.rs", "index.js", "app.py", "__main__.py"];
    entry_points.iter().any(|e| path.ends_with(e))
}
```

## AI Agent Integration

### How AI Agents Should Use ctx

`ctx` is designed to be called by AI agents as a preprocessing step before file operations. Recommended workflow:

1. **Start with status**: Get project overview
   ```bash
   ctx status --json
   ```

2. **Explore structure**: Understand the codebase layout
   ```bash
   ctx map --depth 2 --json
   ```

3. **Find relevant code**: Search before diving into files
   ```bash
   ctx search --symbol "MyClass" --json
   ctx search --caller "process_data" --json
   ```

4. **Understand relationships**: Before modifying a file
   ```bash
   ctx related src/main.rs --json
   ```

5. **Get symbols without full content**: Faster than reading files
   ```bash
   ctx summarize src/ --depth 1 --json
   ```

### Recommended Command Patterns

| Goal | Command |
|------|---------|
| Understand project | `ctx status --json` |
| Find file to edit | `ctx search --symbol "functionName" --json` |
| Understand dependencies | `ctx related path/to/file.rs --json` |
| Get function signatures | `ctx summarize path/to/file.rs --skeleton --json` |
| See recent changes | `ctx diff-context --json` |
| Find callers before refactoring | `ctx search --caller "oldName" --json` |

### JSON Output Schemas

**Status Output**:
```json
{
  "project_name": "ctx",
  "project_type": "rust",
  "branch": "main",
  "is_dirty": false,
  "staged_count": 0,
  "modified_count": 0,
  "untracked_count": 0,
  "recent_commits": [
    {"sha": "abc1234", "message": "...", "author": "...", "time": "...", "time_ago": "2h ago"}
  ],
  "hot_directories": [
    {"path": "src/analysis", "commit_count": 5}
  ],
  "diff_stats": [10, 5]
}
```

**Summarize Output**:
```json
{
  "path": "src/main.rs",
  "language": "rust",
  "lines": 213,
  "symbols": [
    {"name": "main", "kind": "function", "line": 138, "signature": "fn main()", "doc_comment": null}
  ],
  "imports": ["use anyhow::Result;"]
}
```

**Search Output**:
```json
{
  "query": "parse",
  "results": [
    {"path": "src/analysis/treesitter.rs", "line": 49, "column": 1, "text": "[fn] pub fn parse_file(...)", "context": []}
  ]
}
```

**Related Output**:
```json
{
  "source": "src/main.rs",
  "imports": [{"path": "src/commands/mod.rs", "reason": "use crate::commands"}],
  "imported_by": [],
  "co_changed": [{"path": "src/analysis/mod.rs", "reason": "5 commits together"}],
  "test_files": []
}
```

### Error Handling Expectations

- **Exit code 0**: Success
- **Exit code 2**: Error (with message to stderr)
- **JSON errors**: Agents should check for valid JSON before parsing
- **Missing files**: Commands fail gracefully with descriptive error messages

Example error handling in agents:
```python
result = subprocess.run(["ctx", "summarize", path, "--json"], capture_output=True)
if result.returncode != 0:
    # Handle error - message in stderr
    pass
else:
    data = json.loads(result.stdout)
```

## Testing

### How to Run Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Test Organization

Currently, ctx does not have a dedicated test suite. Tests would be organized as:

- **Unit tests**: In-module `#[cfg(test)]` blocks for individual functions
- **Integration tests**: In `tests/` directory for command-level testing

### Adding New Tests

1. **Unit tests** in the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        let keywords = extract_keywords("fix the parser bug");
        assert!(keywords.contains(&"parser".to_string()));
        assert!(!keywords.contains(&"the".to_string())); // stop word
    }
}
```

2. **Integration tests** in `tests/`:

```rust
// tests/commands_test.rs
use std::process::Command;

#[test]
fn test_status_json() {
    let output = Command::new("cargo")
        .args(["run", "--", "status", "--json"])
        .output()
        .expect("Failed to run ctx");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("branch").is_some());
}
```

### Test Fixtures

For testing with sample codebases, create fixtures in `tests/fixtures/`:

```
tests/
└── fixtures/
    ├── rust_project/
    │   ├── Cargo.toml
    │   └── src/
    │       └── main.rs
    └── python_project/
        ├── pyproject.toml
        └── main.py
```
