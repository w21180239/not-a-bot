**English** · [简体中文](../../zh-cn/cli/01-commands.md)

# CLI — Command reference

## `translate`

```bash
translatex translate "Hello world" -t zh
translatex translate -t zh --json '{"text":"Hello","sourceLang":"en"}'
echo "Hello" | translatex translate -t ja
```

| Flag | Default | Meaning |
| --- | --- | --- |
| `TEXT` | stdin | Text to translate |
| `-t, --to` | required | Target language code |
| `-s, --from` | `auto` | Source language or `auto` |
| `-p, --provider` | `auto` | Pin one provider, or walk the chain |
| `--fallback` | empty | Comma-separated providers tried after the pinned one |
| `--preserve` | empty | Regex for spans to keep verbatim; repeatable |
| `--timeout-ms` | config | Per-request budget in milliseconds |
| `--request-id` | generated | Correlation id, echoed and used for cooldowns |
| `--json` | — | Full request object; replaces the flags above |

Semantics worth knowing:

- `--provider deepl --fallback google,edge` tries DeepL, then Google, then Edge. A pinned provider that cannot serve the request (no auto-detection, unsupported pair) hands over to the fallback list instead of failing.
- `--preserve` runs before the chain. A restore failure counts as a failed attempt and moves to the next provider.
- `--from auto` requires the provider to support detection. Providers with `autoDetect: false` are skipped; pinning one returns `AUTO_DETECT_UNSUPPORTED`.

## `batch`

```bash
translatex batch --file jobs.ndjson --parallel 4
cat jobs.ndjson | translatex batch --parallel 8
```

Input is one `TranslateRequest` per line. Output is one object per line, sorted back into input order even though execution is concurrent:

```json
{"index":0,"ok":true,"result":{…}}
{"index":1,"ok":false,"error":{"code":"RATE_LIMITED",…}}
```

Malformed lines are skipped with a stderr warning; the batch continues.

## `providers`

```bash
translatex providers --format json
translatex providers --format text
```

JSON form is `{providers: ProviderInfo[]}`. Text form is `name<TAB>status`, useful in a shell.

## `languages`

```bash
translatex languages                       # every canonical code
translatex languages --provider google     # that provider's advertised set
```

`*` in a provider's advertised set means it accepts any canonical code. An unknown name returns `PROVIDER_NOT_FOUND`; an implemented but unconfigured one returns `PROVIDER_NOT_CONFIGURED`.

## `keys`

```bash
translatex keys                    # cooldown and failure state for configured providers
translatex keys deepl              # one provider
translatex keys --reset            # clear every cooldown file
```

## `health`

```bash
translatex health --format json
```

Probes each registered provider with its real endpoint. Returns `{healthy, providers: HealthRecord[]}` and exits non-zero when a configured provider is unhealthy.

## `schema`, `manifest`, `init`, `completions`

```bash
translatex schema      > schema.json
translatex manifest    > manifest.json
translatex init --path ~/.config/translatex/config.toml
translatex completions bash
```

`schema` emits draft 2020-12 definitions for `TranslateRequest`, `TranslateResponse`, `ErrorResponse`, `ProviderInfo`, `HealthRecord`, `AttemptRecord`.

## Navigation

- ← [CLI overview](OVERVIEW.md)
- → [Config files](02-config-files.md)
