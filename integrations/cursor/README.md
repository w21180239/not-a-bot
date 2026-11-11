# Cursor

Cursor reads project rules from `.cursor/rules/*.mdc` and, in recent versions,
`AGENTS.md`.

## Rule file

```bash
mkdir -p .cursor/rules
cp integrations/cursor/not-a-bot.mdc .cursor/rules/not-a-bot.mdc
```

[`not-a-bot.mdc`](not-a-bot.mdc) sets `alwaysApply: false`, so Cursor loads it
when the request looks like a translation task instead of padding every prompt.

## AGENTS.md instead

```bash
cat integrations/guidance/AGENTS.md >> AGENTS.md
```

Use one or the other. Two copies drift.

## Verify

```bash
not-a-bot translate "Hello world" -t zh --format json
```

Then ask Cursor to translate a sentence. A correct run shows a terminal command
and a `provider` name in the reply.
