# Claude Code

The binary is the integration; a skill tells Claude when to run it.

## Skill (recommended)

Project scope:

```bash
mkdir -p .claude/skills/not-a-bot
cp integrations/skill/not-a-bot/SKILL.md .claude/skills/not-a-bot/SKILL.md
```

Personal, every project:

```bash
mkdir -p ~/.claude/skills/not-a-bot
cp integrations/skill/not-a-bot/SKILL.md ~/.claude/skills/not-a-bot/SKILL.md
```

Claude Code discovers the skill from the frontmatter description and loads the
body when a translation task appears. No config file, no restart.

## Plugin

The plugin bundles the same skill so a team can install it in one command:

```bash
# inside Claude Code
/plugin marketplace add w21180239/not-a-bot
/plugin install not-a-bot@not-a-bot
```

The marketplace entry is [`.claude-plugin/marketplace.json`](../../.claude-plugin/marketplace.json);
the plugin lives in [`plugin/`](plugin/) and contains only
`skills/not-a-bot/SKILL.md`.

## Project instructions instead of a skill

For a repository that prefers one shared instructions file, append the block:

```bash
cat integrations/guidance/AGENTS.md >> CLAUDE.md
```

`CLAUDE.md` is always in context; the skill is loaded on demand and costs fewer
tokens in sessions that never translate. Both work.

## Verify

```bash
claude plugin details not-a-bot          # after the plugin install
not-a-bot translate "Hello world" -t zh --format json
```

`plugin details` should report **Skills (1)**.
