# OpenCode

OpenCode reads `AGENTS.md` from the repository and skills from
`~/.config/opencode/skills/<name>/SKILL.md`.

## Skill (recommended)

```bash
mkdir -p ~/.config/opencode/skills/translatex
cp integrations/skill/translatex/SKILL.md ~/.config/opencode/skills/translatex/SKILL.md
```

The skill is loaded when its description matches the task, so it costs nothing
in sessions that never translate.

## Project instructions

```bash
cat integrations/guidance/AGENTS.md >> AGENTS.md
```

OpenCode keeps `AGENTS.md` in context for the whole session. Use it when the
agent should always know the command; use the skill when it should discover it
on demand.

## Verify

```bash
translatex translate "Hello world" -t zh --format json
opencode run "translate 'Hello world' to Chinese"
```

The agent should call the binary and report the `provider` field, not invent a
translation.

## Notes

- `opencode.json` needs no entry; the skill and `AGENTS.md` are the whole setup.
- Provider order and racing are controlled by `translatex.toml` or the global
  `~/.config/translatex/config.toml`, not by OpenCode.
- To keep a project self-contained, set `TRANSLATEX_HOME` inside the command
  the agent runs, for example `TRANSLATEX_HOME=./.tx translatex …`.
