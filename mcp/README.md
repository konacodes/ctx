# ctx MCP Server

This directory contains the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server wrapper for ctx.

## What is MCP?

MCP is an open standard for connecting AI models to external tools and data sources. It's supported by:
- Claude Desktop
- ChatGPT Desktop
- OpenAI API
- Google AI
- LangChain
- And many more

## Installation

1. Install the MCP Python package:
   ```bash
   pip install mcp
   ```

2. Make sure `ctx` is in your PATH:
   ```bash
   which ctx  # Should show the ctx binary location
   ```

## Usage with Claude Desktop

Add to `~/.config/claude/claude_desktop_config.json` (Linux) or `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "ctx": {
      "command": "python",
      "args": ["/path/to/ctx/mcp/ctx_mcp_server.py"]
    }
  }
}
```

Then restart Claude Desktop. The ctx tools will be available automatically.

## Available Tools

| Tool | Description |
|------|-------------|
| `ctx_status` | Project overview: branch, commits, hot directories |
| `ctx_map` | Directory structure with file counts |
| `ctx_summarize` | Extract symbols from files via tree-sitter |
| `ctx_search` | Search for text, symbol definitions, or function callers |
| `ctx_related` | Find related files through imports and git history |
| `ctx_diff_context` | Git diff with expanded function context |
| `ctx_schema` | Get JSON schema for command outputs |
| `ctx_version` | Get ctx version and capabilities |

## Testing

Run the server directly to test:
```bash
python ctx_mcp_server.py
```

The server communicates via stdin/stdout using JSON-RPC 2.0.

## LangChain Integration

If you're using LangChain, the MCP adapter will automatically convert these tools:

```python
from langchain_mcp import MCPToolkit

toolkit = MCPToolkit(server_params={
    "command": "python",
    "args": ["/path/to/ctx_mcp_server.py"]
})
tools = toolkit.get_tools()
```
