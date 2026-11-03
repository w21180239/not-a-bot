# VS Code / GitHub Copilot

VS Code reads MCP servers from `.vscode/mcp.json` in the workspace.

```json
{
  "servers": {
    "translatex": {
      "type": "stdio",
      "command": "translatex",
      "args": ["mcp"]
    }
  }
}
```

Start the server from the MCP view, then reference it in Copilot Chat with
`#translatex`. The tools are the same five as every other harness.

Note the schema difference: VS Code uses `servers` and requires
`"type": "stdio"`, while most other clients use `mcpServers` without a type.
