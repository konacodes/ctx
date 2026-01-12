# ctx

Context tool for coding agents. Helps AI coding assistants (Claude Code, Aider, Cursor, etc.) get relevant codebase context without cluttering the prompt.

## Installation

### Quick Install (recommended)

The install script handles everything automatically:

```bash
curl -fsSL https://raw.githubusercontent.com/konacodes/ctx/main/install.sh | bash
```

This will:
- Clone the repository and build from source
- Install the `ctx` binary to `~/.local/bin`
- Install Claude Code skills to `~/.claude/skills/ctx`
- Install hooks to `~/.ctx/hooks`
- Clean up temporary files

When installing the hooks, note that it only copies them a universal directory. Enter Claude Code, type `/hooks` to install the `~/.ctx/hooks/hook-starup.sh` to SessionStart and `~/.ctx/hooks/hook-user.sh` to UserPromptSubmit

**Custom install directory:**
```bash
CTX_INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/konacodes/ctx/main/install.sh | bash
```

### Build from Source (Manual)

If you prefer to handle installation yourself:

```bash
# Clone and build
git clone https://github.com/konacodes/ctx.git
cd ctx
cargo build --release

# Install binary
cp target/release/ctx ~/.local/bin/

# (Optional) Install Claude Code skills
mkdir -p ~/.claude/skills
cp -r .claude/skills/ctx ~/.claude/skills/

# (Optional) Install hooks
mkdir -p ~/.ctx/hooks
cp hooks/*.sh ~/.ctx/hooks/
chmod +x ~/.ctx/hooks/*.sh
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

## AI Agent Integration

ctx is designed for seamless integration with AI coding agents. It supports multiple integration methods:

### Agent Skills (Recommended)

ctx includes a `.skills/ctx/SKILL.md` file following the [Agent Skills standard](https://agentskills.io). This is supported by Claude Code, Cursor, VS Code Copilot, and OpenAI.

**Project-level (automatic):** If ctx's `.skills/` directory is in your project, agents will discover it automatically.

**Global installation:**
```bash
mkdir -p ~/.config/skills
cp -r /path/to/ctx/.skills/ctx ~/.config/skills/
```

### MCP Server (Claude Desktop, ChatGPT Desktop)

ctx provides an MCP server wrapper for deep integration:

```bash
# Install MCP package
pip install mcp

# Add to Claude Desktop config (~/.config/claude/claude_desktop_config.json):
{
  "mcpServers": {
    "ctx": {
      "command": "python",
      "args": ["/path/to/ctx/mcp/ctx_mcp_server.py"]
    }
  }
}
```

### Universal Discovery

Any AI agent can query ctx's capabilities:

```bash
ctx --capabilities  # Full JSON tool definitions
ctx version --json  # Version and supported features
ctx schema status   # JSON schema for specific command output
```

### CLAUDE.md Integration

For Claude Code, add to your project's `CLAUDE.md` (or `~/.claude/CLAUDE.md` for global):

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

### Why ctx over grep/find?

| Task | ctx | Traditional |
|------|-----|-------------|
| Find function definition | `ctx search --symbol parse` | `grep -r "def parse\|fn parse"` (noisy) |
| Find callers | `ctx search --caller validate` | **Impossible with grep** |
| Understand file | `ctx summarize src/main.rs` | `cat src/main.rs` (too much) |
| Project structure | `ctx map --depth 2` | `find . -type f \| head` |

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
