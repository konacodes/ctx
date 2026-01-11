# ctx

Context tool for coding agents. Helps AI coding assistants (Claude Code, Aider, Cursor, etc.) get relevant codebase context without cluttering the prompt.

## Installation

### Quick Install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/konacodes/ctx/main/install.sh | bash
```

### Custom Install Directory

```bash
CTX_INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/konacodes/ctx/main/install.sh | bash
```

### Build from Source

```bash
git clone https://github.com/konacodes/ctx.git
cd ctx
cargo build --release
cp target/release/ctx ~/.local/bin/
```

## Commands

### `ctx init`
Initialize a `.ctx/` directory with configuration.

```bash
ctx init
```

### `ctx status`
Quick project overview showing branch, recent commits, and hot directories.

```bash
ctx status
ctx status --json
```

### `ctx map`
Show project structure with file counts.

```bash
ctx map
ctx map --depth 2
ctx map src/
```

### `ctx summarize <path>`
Extract symbols and imports from files or directories.

```bash
ctx summarize src/main.rs
ctx summarize src/ --depth 2
ctx summarize src/main.rs --skeleton  # Show only signatures
```

### `ctx search <query>`
Search the codebase for text, symbols, or callers.

```bash
ctx search "error handling"
ctx search --symbol "parse"        # Find symbol definitions
ctx search --caller "validate"     # Find callers of a function
```

### `ctx related <file>`
Find files related to a given file.

```bash
ctx related src/main.rs
```

### `ctx diff-context [ref]`
Show diff with expanded function context.

```bash
ctx diff-context           # Uncommitted changes
ctx diff-context HEAD~3    # Last 3 commits
```

### `ctx inject`
Inject context into a prompt (reads from stdin).

```bash
echo "How do I add authentication?" | ctx inject --budget 1000
```

### `ctx hook-inject`
Claude Code hook handler for automatic context injection.

```bash
echo '{"prompt": "fix the bug"}' | ctx hook-inject
```

### `ctx config`
Manage configuration.

```bash
ctx config list
ctx config get budget
ctx config set budget 3000
```

## Output Formats

All commands support multiple output formats:

```bash
ctx status              # Human-readable (default)
ctx status --json       # Pretty JSON
ctx status --compact    # Minified JSON
```

## Claude Code Integration

Add to your project's `CLAUDE.md` (or `~/.claude/CLAUDE.md` for global):

```markdown
## Codebase Context

Use `ctx` for codebase exploration instead of grep/glob:

- `ctx status` - project overview, branch, recent commits, hot directories
- `ctx map` - annotated directory structure (better than ls/find)
- `ctx summarize <path>` - extract symbols via tree-sitter (faster than reading files)
- `ctx search --symbol X` - find definitions precisely (less noise than grep)
- `ctx search --caller X` - trace call sites (AST-based, grep can't do this)
- `ctx related <file>` - imports, importers, co-changed files, tests
- `ctx diff-context` - changed functions with context

All commands support `--json`. Use ctx first for understanding code structure.
```

## Configuration

Configuration is stored in `.ctx/config.toml`:

```toml
# Default token budget for context injection
budget = 2000

# Languages to parse (auto-detect by default)
# languages = ["rust", "python", "javascript", "typescript"]

# Additional patterns to ignore
# ignore = ["*.min.js", "vendor/"]
```

## Ignored Files

ctx automatically respects `.gitignore` and excludes common non-source directories:

- VCS: `.git`, `.svn`, `.hg`
- Dependencies: `node_modules`, `vendor`, `bower_components`
- Build outputs: `target`, `build`, `dist`, `__pycache__`
- IDE: `.idea`, `.vscode`
- And more...

## License

MIT
