# OpenCode

OpenCode reads MCP servers from `opencode.json` (project) or
`~/.config/opencode/opencode.json` (global), and skills from
`~/.config/opencode/skills/<name>/SKILL.md`.

## MCP server

Merge this into `opencode.json`:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "translatex": {
        "type": "local",
        "command": ["translatex", "mcp"]
      }
    }
  }
}
```

That is the shape `opencode mcp add` writes. `opencode mcp list` should then
show `✓ translatex connected`. Older configs that put the server directly under
`mcp` (`"mcp": { "translatex": { … } }`) still parse; the schema accepts both.

Restart OpenCode, then ask for a translation. The tools appear as
`translatex_translate`, `translatex_roundtrip`, `translatex_providers`,
`translatex_languages` and `translatex_health`.

## Skill

```bash
mkdir -p ~/.config/opencode/skills/translatex
cp integrations/skill/SKILL.md ~/.config/opencode/skills/translatex/SKILL.md
```

The skill teaches the agent when to use `roundtrip` instead of `translate` and
which errors are worth retrying. Without it the MCP tools still work; with it
the agent reaches for them without being asked.

## Why MCP rather than a plugin

An OpenCode plugin is a JavaScript module loaded into the process. Wrapping a
separate translation binary in one adds a layer without adding capability. The
MCP server already exposes everything and is shared with every other harness.

## Agent-specific defaults

If you want different provider order or racing per project, put a
`config.toml` next to the project and point the server at it:

```json
{
  "mcp": {
    "servers": {
      "translatex": {
        "type": "local",
        "command": ["translatex", "mcp"],
        "environment": { "TRANSLATEX_CONFIG": "./translatex.toml" }
      }
    }
  }
}
```

`translatex.toml`:

```toml
chain = ["edge", "google", "iciba"]
race_width = 2
roundtrip_pivot = "ja"
```
