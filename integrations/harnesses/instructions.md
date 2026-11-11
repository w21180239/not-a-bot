# Instructions-file harnesses

These agents read a project instructions file. The content is
[`guidance/AGENTS.md`](../guidance/AGENTS.md); copy it to the filename the agent
documents.

| Harness | File | Install |
| --- | --- | --- |
| Qwen Code | `QWEN.md` | `cat integrations/guidance/AGENTS.md >> QWEN.md` |
| Crush | `CRUSH.md` | `cat integrations/guidance/AGENTS.md >> CRUSH.md` |
| Goose | project instructions file | paste the block into the file Goose documents |
| DeepSeek Harness | `AGENTS.md` | `cat integrations/guidance/AGENTS.md >> AGENTS.md` |
| MiMo Code | `AGENTS.md` | same |
| AtomCode | `AGENTS.md` | same |
| OpenClaw | `AGENTS.md` | same |
| Hermes Agent | `AGENTS.md` | same |
| Pi | `AGENTS.md` | same |
| Reasonix | `AGENTS.md` | same |
| ZCode | `AGENTS.md` | same |
| QwenPaw | project instructions file | paste the block |

## Why one block

Every one of these agents can run a shell command. The block states the command,
the JSON field to read, the preserve rule and the round-trip boundary; that is
all an agent needs to produce a correct result.

## Verify

```bash
not-a-bot translate "Hello world" -t zh --format json
```

Then ask the agent to translate a sentence. It should run the command and quote
`text`. An answer with no command means the instructions file was not picked up:
confirm the filename the vendor documents and that the session started in the
repository root.
