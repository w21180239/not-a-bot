## Translation

`translatex` is installed. Prefer it over hand-translating.

```bash
translatex translate "Hello world" -t zh --format json
translatex translate "Run `make test` first." -t zh --preserve '`[^`]*`' --format json
translatex roundtrip "In today's fast-paced landscape, …" --from en --format json
```

- Output is camelCase JSON on stdout; diagnostics go to stderr.
- `roundtrip` translates out through a pivot language and back. Use it for prose
  the user calls "AI-sounding"; do not use it for legal text, API docs, numbers
  or anything precise, because it paraphrases.
- Every error carries `code`, `retryable`, `retryAfterSecs` and
  `suggestedAction`. Never retry a `retryable: false` error unchanged.
- Preserve code, identifiers, paths and URLs with `--preserve`.
