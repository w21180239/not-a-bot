# Claude Code

Three ways in, from most to least integrated.

## 1. Plugin (MCP server + skill)

The plugin bundles the MCP server and a skill that tells Claude when to reach for
it, so nothing has to be repeated in a prompt.

```bash
# Inside Claude Code
/plugin marketplace add w21180239/translatex
/plugin install translatex@translatex
```

The marketplace entry is `.claude-plugin/marketplace.json`; the plugin itself
lives in [`plugin/`](plugin/).

## 2. MCP server only

Project scope, checked into the repo:

```bash
cp integrations/claude-code/.mcp.json .mcp.json
```

Or register it for every project:

```bash
claude mcp add --scope user translatex -- translatex mcp
```

[`.mcp.json`](.mcp.json) contains:

```json
{
  "mcpServers": {
    "translatex": {
      "command": "translatex",
      "args": ["mcp"]
    }
  }
}
```

Tools: `translate`, `roundtrip`, `providers`, `languages`, `health`.

## 3. Skill only

For a session that should use the CLI directly:

```bash
mkdir -p .claude/skills/translatex
cp integrations/skill/SKILL.md .claude/skills/translatex/SKILL.md
```

## Credentials

The MCP server reads `~/.config/translatex/credentials.toml` and
`TRANSLATEX_*` environment variables. To give the server a key without touching
the host config, add an `env` block:

```json
{
  "mcpServers": {
    "translatex": {
      "command": "translatex",
      "args": ["mcp"],
      "env": { "TRANSLATEX_DEEPL_API_KEY": "${DEEPL_API_KEY}" }
    }
  }
}
```

Restart Claude Code after changing credentials; the server builds its provider
registry once at startup.

## Verify

```bash
translatex mcp <<'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
EOF
```

Two JSON responses, the second listing five tools.
