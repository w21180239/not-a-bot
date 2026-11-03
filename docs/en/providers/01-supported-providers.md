**English** · [简体中文](../../zh-cn/providers/01-supported-providers.md)

# Providers — Supported backends

[← Domain overview](OVERVIEW.md) · [Documentation index](../README.md)

`chars` is the adapter's request limit before splitting. `auto` marks whether the endpoint can detect the source language. Status reflects a live sweep, not the code path.

## Free web endpoints

| # | Provider | Id | Auto | Chars | Endpoint / mechanism | Status |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Google Translate | `google` | yes | 5 000 | `translate.googleapis.com/translate_a/single`, `client=gtx` | verified |
| 2 | Microsoft Edge Translator | `edge` | yes | 50 000 | `edge.microsoft.com/translate/auth` + `api-edge.cognitive.microsofttranslator.com` | verified |
| 3 | Iciba | `iciba` | yes | 3 000 | `dict.iciba.com`, AES-ECB signed body, encrypted response | verified |
| 4 | iFlytek iFlyrec | `iflyrec` | yes | 2 000 | `fanyi.iflyrec.com/TranslationService/v1/textAutoTranslation`, numeric language ids | zh-anchored pairs only |
| 4 | ModernMT | `modernmt` | yes | 5 000 | `webapi.modernmt.com/translate`, md5 verify + timestamp | verified |
| 5 | Volcengine | `volcengine` | yes | 5 000 | `translate.volcengine.com/crx/translate/v1/` | verified |
| 5 | Lara Translate | `lara` | yes | 500 | `laratranslate.com`, bearer token scraped from the page | verified, daily credit |
| 6 | Lingvanex | `lingvanex` | no | 1 000 | `lingvanex.com/translation/api`, `API_BEARER_TOKEN` from the page | verified |
| 7 | Mirai Translate | `mirai` | yes | 500 | `trial.miraitranslate.com`, `var tran` key scraped per call + trace ping | verified, Japanese-anchored pairs only |
| 8 | Papago | `papago` | yes | 1 000 | `papago.naver.com`, cookie-primed detect then form POST | verified |
| 9 | Reverso Context | `reverso` | no | 2 000 | `api.reverso.net/translate/v1/translation`, three-letter codes | Cloudflare blocks rustls JA3; excluded from the auto chain |
| 10 | Sogou | `sogou` | yes | 5 000 | `fanyi.sogou.com/api/transpc/text/result`, md5 sign | per-IP throttle → `RATE_LIMITED` |
| 11 | MyMemory | `mymemory` | no | 5 000 | `api.mymemory.translated.net/get`, RFC 3066 codes | verified, daily cap |
| 12 | SysTran | `systran` | no | 5 000 | `translate.systran.net`, OAuth client credentials from the bundle | verified |
| — | LibreTranslate | `libretranslate` | yes | 5 000 | any self-hosted `/translate` | requires `base_url` |

## Official APIs

| # | Provider | Id | Credential fields | Notes |
| --- | --- | --- | --- | --- |
| 20 | DeepL | `deepl` | `api_key` | Free and Pro keys share the same shape |
| 20 | Baidu | `baidu` | `appid`, `secret_key` | MD5-signed, per-language code table |
| 21 | Google Cloud Translation | `googlecloud` | `api_key` | `v2` endpoint, `q` repeated per segment |
| 22 | Azure Translator | `azure` | `api_key`, `region` | Subscription key plus `Ocp-Apim-Subscription-Region` |
| 23 | Alibaba Cloud | `aliyun` | `access_key_id`, `access_key_secret` | RPC signature v1 |
| 24 | Caiyun | `caiyun` | `token` | `api.interpreter.caiyunai.com` |
| 25 | Niutrans | `niutrans` | `apikey` | https API v2 |
| 26 | Tencent | `tencent` | `secret_id`, `secret_key` | TC3-HMAC-SHA256 |
| 27 | Youdao | `youdao` | `app_key`, `app_secret` | v3 sign with SHA-256 |

All official providers are registered only when their credential set is complete. `translatex providers` reflects that.

## LLM providers

| # | Provider | Id | Fields |
| --- | --- | --- | --- |
| 40 | OpenAI | `openai` | `api_key`, optional `base_url`, `model` |
| 41 | Anthropic | `anthropic` | `api_key`, optional `model` |
| 42 | DeepSeek | `deepseek` | `api_key`, optional `model` |

LLMs translate through a chat completion prompt. They are the last resort because they cost per token and can paraphrase instead of translating.

## Retired names

| Name | Resolution |
| --- | --- |
| `iflytek` | Aliased to `iflyrec`. `fanyi.xfyun.cn` requires a logged-in account; the old endpoints answer 404. |

Aliases resolve at lookup time, so `--provider iflytek` works and never duplicates the chain.

## Navigation

- ← [Providers overview](OVERVIEW.md)
- → [Priorities and fallback](02-priorities-and-fallback.md)
