#!/usr/bin/env python3
"""
MCP (Model Context Protocol) server for ctx.

This wrapper exposes ctx commands as MCP tools, enabling deep integration
with Claude Desktop, ChatGPT Desktop, and other MCP-compatible clients.

Installation:
    pip install mcp

Usage with Claude Desktop:
    Add to ~/.config/claude/claude_desktop_config.json:
    {
        "mcpServers": {
            "ctx": {
                "command": "python",
                "args": ["/path/to/ctx_mcp_server.py"]
            }
        }
    }

Usage standalone:
    python ctx_mcp_server.py
"""

import subprocess
import json
import sys
from typing import Optional

try:
    from mcp.server import Server
    from mcp.types import Tool, TextContent
    from mcp.server.stdio import stdio_server
    import asyncio
except ImportError:
    print("Error: mcp package not installed. Run: pip install mcp", file=sys.stderr)
    sys.exit(1)


def run_ctx(*args: str) -> tuple[str, str, int]:
    """Run a ctx command and return (stdout, stderr, returncode)."""
    try:
        result = subprocess.run(
            ["ctx"] + list(args),
            capture_output=True,
            text=True,
            timeout=60
        )
        return result.stdout, result.stderr, result.returncode
    except FileNotFoundError:
        return "", "ctx not found in PATH. Please install ctx first.", 1
    except subprocess.TimeoutExpired:
        return "", "Command timed out after 60 seconds", 1


server = Server("ctx")


@server.list_tools()
async def list_tools() -> list[Tool]:
    """List all available ctx tools."""
    return [
        Tool(
            name="ctx_status",
            description="Get project status overview including git branch, recent commits, and hot directories. Use this first when exploring a new codebase.",
            inputSchema={
                "type": "object",
                "properties": {},
                "required": []
            }
        ),
        Tool(
            name="ctx_map",
            description="Show project directory structure with file counts. Better than ls/find for understanding codebase layout.",
            inputSchema={
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to map (default: current directory)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum depth to traverse"
                    }
                },
                "required": []
            }
        ),
        Tool(
            name="ctx_summarize",
            description="Extract symbols (functions, classes, etc.) from files using tree-sitter AST parsing. Much faster than reading entire files when you just need structure.",
            inputSchema={
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "File or directory paths to summarize"
                    },
                    "skeleton": {
                        "type": "boolean",
                        "description": "Show only function/class signatures"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum depth for directory summarization"
                    }
                },
                "required": ["paths"]
            }
        ),
        Tool(
            name="ctx_search",
            description="Search codebase for text, symbol definitions, or function callers. Use --symbol for precise definition matching. Use --caller to find all call sites of a function (AST-based, impossible with grep).",
            inputSchema={
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "symbol": {
                        "type": "boolean",
                        "description": "Search for symbol definitions only (not usage)"
                    },
                    "caller": {
                        "type": "boolean",
                        "description": "Find callers of a function (AST-based)"
                    },
                    "context": {
                        "type": "integer",
                        "description": "Lines of context to show around matches"
                    }
                },
                "required": ["query"]
            }
        ),
        Tool(
            name="ctx_related",
            description="Find files related to a given file through imports, reverse imports, co-changes in git history, and associated test files.",
            inputSchema={
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "File path to find relations for"
                    }
                },
                "required": ["file"]
            }
        ),
        Tool(
            name="ctx_diff_context",
            description="Show git diff with expanded function context. Better than raw git diff for understanding what changed.",
            inputSchema={
                "type": "object",
                "properties": {
                    "git_ref": {
                        "type": "string",
                        "description": "Git ref to diff against (default: uncommitted changes)"
                    }
                },
                "required": []
            }
        ),
        Tool(
            name="ctx_schema",
            description="Get JSON schema for a command's output format. Useful for understanding output structure.",
            inputSchema={
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to get schema for (status, map, summarize, search, related, diff-context)"
                    }
                },
                "required": ["command"]
            }
        ),
        Tool(
            name="ctx_version",
            description="Get ctx version and capabilities including supported languages, commands, and features.",
            inputSchema={
                "type": "object",
                "properties": {},
                "required": []
            }
        )
    ]


@server.call_tool()
async def call_tool(name: str, arguments: dict) -> list[TextContent]:
    """Handle tool calls."""

    if name == "ctx_status":
        stdout, stderr, code = run_ctx("status", "--json")

    elif name == "ctx_map":
        args = ["map", "--json"]
        if arguments.get("path"):
            args.append(arguments["path"])
        if arguments.get("depth"):
            args.extend(["--depth", str(arguments["depth"])])
        stdout, stderr, code = run_ctx(*args)

    elif name == "ctx_summarize":
        args = ["summarize", "--json"]
        if arguments.get("skeleton"):
            args.append("--skeleton")
        if arguments.get("depth"):
            args.extend(["--depth", str(arguments["depth"])])
        args.extend(arguments.get("paths", []))
        stdout, stderr, code = run_ctx(*args)

    elif name == "ctx_search":
        args = ["search", "--json"]
        if arguments.get("symbol"):
            args.append("--symbol")
        if arguments.get("caller"):
            args.append("--caller")
        if arguments.get("context"):
            args.extend(["-C", str(arguments["context"])])
        args.append(arguments["query"])
        stdout, stderr, code = run_ctx(*args)

    elif name == "ctx_related":
        stdout, stderr, code = run_ctx("related", "--json", arguments["file"])

    elif name == "ctx_diff_context":
        args = ["diff-context", "--json"]
        if arguments.get("git_ref"):
            args.append(arguments["git_ref"])
        stdout, stderr, code = run_ctx(*args)

    elif name == "ctx_schema":
        stdout, stderr, code = run_ctx("schema", arguments["command"])

    elif name == "ctx_version":
        stdout, stderr, code = run_ctx("version", "--json")

    else:
        return [TextContent(type="text", text=f"Unknown tool: {name}")]

    if code != 0:
        error_msg = stderr if stderr else f"Command failed with exit code {code}"
        return [TextContent(type="text", text=f"Error: {error_msg}\n\nOutput: {stdout}")]

    return [TextContent(type="text", text=stdout)]


async def main():
    """Run the MCP server."""
    async with stdio_server() as (read_stream, write_stream):
        await server.run(read_stream, write_stream)


if __name__ == "__main__":
    asyncio.run(main())
