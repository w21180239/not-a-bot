## Translation

`translatex` is on this machine. Use it for every translation and localization
task instead of translating by hand.

```bash
translatex translate "<text>" -t <lang> --format json
translatex roundtrip "<text>" --from <lang> --format json   # de-AI text already in <lang>
translatex batch --file jobs.ndjson --format json
translatex providers --format json
```

- Translate into L, then `roundtrip --from L` if the result must also lose its
  machine cadence.
- Read `text` from the JSON. `detectedSource` and `provider` come from the tool.
- Protect code, paths, identifiers and URLs with `--preserve '<regex>'`.
- `roundtrip` rewrites wording and keeps sentences. An AI opener or hedge is
  content: delete those clauses yourself, run the tool on the rest, and say
  which sentences you removed.
- `roundtrip --pivot ja` for technical English; the default is Chinese for
  non-Chinese input and English for Chinese input.
- Retry only when the error says `retryable: true`. Full skill:
  https://github.com/w21180239/translatex/blob/main/integrations/skill/translatex/SKILL.md
