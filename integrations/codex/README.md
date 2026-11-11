# Codex CLI

Codex reads two things: project instructions and cross-runtime skills. Use
either or both; the binary is the same.

## Skill

```bash
mkdir -p ~/.agents/skills/not-a-bot
cp integrations/skill/not-a-bot/SKILL.md ~/.agents/skills/not-a-bot/SKILL.md
```

`~/.agents/skills/` is the cross-runtime alias Codex, Gemini CLI and Copilot CLI
share, so one copy serves all three.

## Project instructions

```bash
cat integrations/guidance/AGENTS.md >> AGENTS.md
```

Or for every repository:

```bash
cat integrations/guidance/AGENTS.md >> ~/.codex/AGENTS.md
```

## Verify

```bash
not-a-bot translate "Deploy failed" -t zh --format json
codex exec "translate 'Deploy failed' to Chinese"
```

Codex should run the CLI rather than type a translation. If it answers from its
own knowledge, the instructions file is in the wrong place — check that `AGENTS.md`
sits at the repository root and that the session was started there.

## Notes

- The CLI is the whole integration: no server, no config entry, no restart.
- The CLI is fast enough for interactive use: a cold run is ~40 ms plus one
  network round trip.
- Structured output is on by default; `--format json` makes it explicit.
