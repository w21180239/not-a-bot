# OpenCode

OpenCode reads `AGENTS.md` from the repository and skills from
`~/.config/opencode/skills/<name>/SKILL.md`.

## Skill (recommended)

```bash
mkdir -p ~/.config/opencode/skills/not-a-bot
cp -r integrations/skill/not-a-bot/. ~/.config/opencode/skills/not-a-bot/
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
not-a-bot translate "Hello world" -t zh --format json
opencode run "translate 'Hello world' to Chinese"
```

The agent should call the binary and report the `provider` field, not invent a
translation.

## Notes

- `opencode.json` needs no entry; the skill and `AGENTS.md` are the whole setup.
- Provider order and racing are controlled by `not-a-bot.toml` or the global
  `~/.config/not-a-bot/config.toml`, not by OpenCode.
- To keep a project self-contained, set `NOT_A_BOT_HOME` inside the command
  the agent runs, for example `NOT_A_BOT_HOME=./.tx not-a-bot …`.
