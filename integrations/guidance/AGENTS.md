## Translation

`not-a-bot` is on this machine. Use it for every translation and localization
task instead of translating by hand.

```bash
not-a-bot translate "<text>" -t <lang> --format json
not-a-bot rewrite "<text>" --from <lang> --preset standard --preserve @code --format json   # humanise text already in <lang>
not-a-bot batch --file jobs.ndjson --format json
not-a-bot providers --format json
```

- Translate into L, then `rewrite --from L --preset standard` if the result must also
  read less like generated text.
- Read `text` from the JSON. `detectedSource` and `provider` come from the tool.
- Protect code, paths, identifiers and URLs with `--preserve @code` and `--preserve @url`, or a regex.
- `--preset standard` (your Claude or OpenCode subscription, no API key) rebuilds the
  sentences and scores lowest on the detector. `--preset light` rewrites least and keeps
  the most meaning. Both end with a meaning-repair hop; report what the result lost.
- Read `missingFacts` and `warnings` after a round trip and report them. Never
  round-trip legal text or numbers the user will check.
- Retry only when the error says `retryable: true`. Full skill:
  https://github.com/w21180239/not-a-bot/blob/main/integrations/skill/not-a-bot/SKILL.md
