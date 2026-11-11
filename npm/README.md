# not-a-bot

**Get AI-assisted writing past readers, reviewers and AI detectors, with every fact intact.**

`npm install -g not-a-bot` puts a 6 MB static binary on `PATH`. `rewrite`
rewrites AI-generated text to read like a person wrote it, on your own Claude or
OpenCode subscription, and reports any name, number or code that went missing.
28 translation providers raced in parallel, structured JSON out, one skill file
for 24 coding agents. The binary ships in a per-platform package (`not-a-bot-windows-x64`,
`not-a-bot-darwin-arm64`, …) pulled in as an optional dependency, so the
install runs no scripts at all.

```bash
npm install -g not-a-bot
not-a-bot translate "Hello world" -t zh
```

## Agent setup

The integration is the CLI plus a skill. Give your agent the skill from
[`integrations/skill/not-a-bot/SKILL.md`](https://github.com/w21180239/not-a-bot/blob/main/integrations/skill/not-a-bot/SKILL.md),
or the short block in
[`integrations/guidance/AGENTS.md`](https://github.com/w21180239/not-a-bot/blob/main/integrations/guidance/AGENTS.md)
for harnesses that read a project instructions file.

```bash
not-a-bot translate "Hello world" -t zh --format json
not-a-bot rewrite "In today's landscape, …" --from en --preset standard --preserve @code --format json
```

Full harness matrix: https://github.com/w21180239/not-a-bot/tree/main/integrations

## Offline install

```bash
NOT_A_BOT_BINARY_PATH=/path/to/not-a-bot npm install -g not-a-bot
```

Or skip npm entirely:

```bash
cargo install --git https://github.com/w21180239/not-a-bot --locked
```

Full documentation: https://github.com/w21180239/not-a-bot
