#!/usr/bin/env bash
# Claude Code user prompt hook - provides fresh context and reminder
set -euo pipefail

# Show current project state
echo "=== Project Status ==="
ctx status --compact 2>/dev/null || ctx status 2>/dev/null || echo "ctx status unavailable"

echo ""
echo "=== Project Structure ==="
ctx map --depth 1 2>/dev/null || echo "ctx map unavailable"

echo ""
echo "=== ctx Reminder ==="
cat << 'EOF'
ALWAYS prefer ctx over grep/find/ls for code exploration:

  ctx search --symbol X   # Find definition (better than grep)
  ctx search --caller X   # Find callers (impossible with grep)
  ctx summarize <path>    # Get file structure (faster than reading)
  ctx related <file>      # Find dependencies and related files

Need help? Run:
  ctx --help              # List all commands
  ctx <command> --help    # Help for specific command
  ctx --capabilities      # Full tool definitions (JSON)
  ctx schema <command>    # Output schema for a command
EOF
