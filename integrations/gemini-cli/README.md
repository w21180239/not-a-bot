# Gemini CLI

Gemini CLI reads `GEMINI.md` for instructions and `~/.agents/skills/` for
cross-runtime skills.

## Skill

```bash
mkdir -p ~/.agents/skills/not-a-bot
cp integrations/skill/not-a-bot/SKILL.md ~/.agents/skills/not-a-bot/SKILL.md
```

## Instructions

```bash
cat integrations/guidance/AGENTS.md >> GEMINI.md
```

## Verify

```bash
not-a-bot translate "Hello world" -t zh --format json
gemini -p "translate 'Hello world' to Chinese"
```

If Gemini answers without a tool call, confirm the file is `GEMINI.md` at the
repository root, not `AGENTS.md` alone.
