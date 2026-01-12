#!/usr/bin/env bash
# Claude Code startup hook - initializes ctx and provides context
set -euo pipefail

# Initialize ctx if not already done
ctx init --json 2>/dev/null || true

# Show project structure
echo "=== Project Structure ==="
ctx map --depth 2

echo ""
echo "=== ctx Usage Guide ==="
cat << 'EOF'
Use `ctx` for codebase exploration instead of grep/find/ls:

WHEN TO USE:
- Exploring unfamiliar code → ctx map, ctx status
- Finding function definitions → ctx search --symbol <name>
- Finding who calls a function → ctx search --caller <name> (grep can't do this!)
- Understanding a file → ctx summarize <path>
- Finding related files → ctx related <file>
- Reviewing changes → ctx diff-context

QUICK REFERENCE:
  ctx status              # Project overview, branch, recent commits
  ctx map                 # Directory structure
  ctx summarize <path>    # Extract symbols from file/directory
  ctx search --symbol X   # Find where X is defined
  ctx search --caller X   # Find all callers of X
  ctx related <file>      # Imports, importers, co-changed files
  ctx diff-context        # Changed functions with context

All commands support --json for structured output.
EOF
