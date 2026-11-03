# Rules-file harnesses

Cline, Roo Code, Kilo Code, Continue, JetBrains AI, Zed and Aider all read a
project rule file. The content is the same block; only the path changes.

## Cline

```bash
mkdir -p .clinerules
cat integrations/guidance/AGENTS.md > .clinerules/translatex.md
```

## Roo Code

```bash
mkdir -p .roo/rules
cat integrations/guidance/AGENTS.md > .roo/rules/translatex.md
```

## Kilo Code

```bash
mkdir -p .kilocode/rules
cat integrations/guidance/AGENTS.md > .kilocode/rules/translatex.md
```

## Continue

```bash
mkdir -p .continue/rules
cat integrations/guidance/AGENTS.md > .continue/rules/translatex.md
```

## JetBrains AI

```bash
mkdir -p .aiassistant/rules
cat integrations/guidance/AGENTS.md > .aiassistant/rules/translatex.md
```

## Zed

```bash
cat integrations/guidance/AGENTS.md > .rules
```

Zed keeps a single `.rules` file at the project root; append if one already
exists.

## Aider

```bash
cat integrations/guidance/AGENTS.md > CONVENTIONS.md
aider --read CONVENTIONS.md
```

Persist it in `.aider.conf.yml`:

```yaml
read:
  - CONVENTIONS.md
```

## Notes

- All seven harnesses run shell commands, so the CLI is the whole integration.
- If a tool also reads `AGENTS.md`, prefer one file over two.
