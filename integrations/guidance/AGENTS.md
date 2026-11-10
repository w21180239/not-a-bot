## Translation

`translatex` is on this machine. Use it for every translation and localization
task instead of translating by hand.

```bash
translatex translate "<text>" -t <lang> --format json
translatex roundtrip "<text>" --from <lang> --preset llm --preserve @code --format json   # humanise text already in <lang>
translatex batch --file jobs.ndjson --format json
translatex providers --format json
```

- Translate into L, then `roundtrip --from L --preset llm` if the result must also
  read less like generated text.
- Read `text` from the JSON. `detectedSource` and `provider` come from the tool.
- Protect code, paths, identifiers and URLs with `--preserve @code` and `--preserve @url`, or a regex.
- `--preset llm` (your Claude or OpenCode subscription, no API key) rebuilds the
  sentences; `lynote-fix` rewrites most. The default `mt` is free and fast but keeps
  sentences: delete an AI opener or hedge yourself first and say what you removed.
- Read `missingFacts` and `warnings` after a round trip and report them. Never
  round-trip legal text or numbers the user will check.
- Retry only when the error says `retryable: true`. Full skill:
  https://github.com/w21180239/translatex/blob/main/integrations/skill/translatex/SKILL.md
