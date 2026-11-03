# translatex

**Free translation for your agent — no API key, no signup, no config.**

`npm install -g translatex` puts a 6 MB static binary on `PATH`. 26 translation
providers raced in parallel, structured JSON out, one skill file for 24 coding
agents. The binary ships in a per-platform package (`translatex-windows-x64`,
`translatex-darwin-arm64`, …) pulled in as an optional dependency, so the
install runs no scripts at all.

```bash
npm install -g translatex
translatex translate "Hello world" -t zh
```

## Agent setup

The integration is the CLI plus a skill. Give your agent the skill from
[`integrations/skill/translatex/SKILL.md`](https://github.com/w21180239/translatex/blob/main/integrations/skill/translatex/SKILL.md),
or the short block in
[`integrations/guidance/AGENTS.md`](https://github.com/w21180239/translatex/blob/main/integrations/guidance/AGENTS.md)
for harnesses that read a project instructions file.

```bash
translatex translate "Hello world" -t zh --format json
translatex roundtrip "In today's landscape, …" --from en --format json
```

Full harness matrix: https://github.com/w21180239/translatex/tree/main/integrations

## Offline install

```bash
TRANSLATEX_BINARY_PATH=/path/to/translatex npm install -g translatex
```

Or skip npm entirely:

```bash
cargo install translatex --locked
```

Full documentation: https://github.com/w21180239/translatex
