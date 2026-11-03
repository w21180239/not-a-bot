# translatex

**Перевод для агентов: сначала веб-эндпоинты, официальные API — как резерв.** Один статический бинарник: бесплатные веб-эндпоинты в приоритете, платные API подключаются при наличии ключей, ответ всегда в структурированном JSON.

[English](./README.md) · [简体中文](./README.zh-cn.md) · [Русский](./README.ru.md) · [Документация](./docs/README.md)

---

## Почему веб-эндпоинты первыми

Официальный API перевода стоит денег, требует регистрации и полностью отключается при просроченном счёте. Публичные веб-эндпоинты за Google Translate, Bing, iFlytek, Papago и Reverso отвечают на те же запросы бесплатно и с тем же качеством. `translatex` использует их как основной путь, а платные API — как резерв для гарантированного SLA и языков, которые бесплатные эндпоинты не принимают.

```
запрос ─▶ приоритет 1-12   бесплатные веб-эндпоинты   (без ключей)
       ─▶ приоритет 20-27  официальные API            (только при наличии ключей)
       ─▶ приоритет 40-42  LLM                        (крайний случай, по желанию)
```

Меньшее число — выше приоритет. Каждая попытка фиксируется, поэтому при ошибке видно, какие движки проверялись и почему отказали.

## Возможности

| Возможность | Что даёт |
| --- | --- |
| **26 провайдеров** | 14 бесплатных веб-эндпоинтов, 9 официальных API, 3 LLM. LibreTranslate включается через `base_url`. |
| **Цепочка резерва** | Сортировка по приоритету; порядок `--fallback` сохраняется; закреплённый провайдер всё равно уходит в fallback. |
| **Ротация ключей** | Несколько ключей на провайдера, round-robin, состояние cooldown общее для процессов в `~/.config/translatex/cache/`. |
| **Структурированный вывод** | JSON/NDJSON в stdout, логи в stderr, коды возврата для ветвления в агенте. |
| **Защита фрагментов** | Код, переменные и URL маскируются до перевода и восстанавливаются после; сбой восстановления запускает fallback. |
| **MCP-сервер** | `translatex mcp` по stdio: `translate`, `providers`, `languages`, `health`. |
| **Работающий таймаут** | `timeoutMs` из запроса (или `timeout_ms` в конфиге) ограничивает весь вызов провайдера. |
| **Понятные ошибки** | `code`, `retryable`, `retryAfterSecs`, `suggestedAction` в каждом ответе об ошибке. |

## Установка

```bash
cargo install translatex                 # Rust 1.80+
# или готовый бинарник:
# https://github.com/w21180239/translatex/releases/latest
```

```bash
translatex --version
translatex providers --format text
```

## Быстрый старт

```bash
# Без настройки, только веб-эндпоинты
translatex translate "Hello world" -t zh

# Закрепить провайдера и оставить резерв
translatex translate "Deploy failed" -t ja --provider deepl --fallback google,edge

# Чтение из stdin
echo "Good morning" | translatex translate -t fr

# Пакетная обработка NDJSON
translatex batch --file examples/batch.ndjson --parallel 4

# Не переводить код в обратных кавычках
translatex translate 'Run `make test` before pushing.' -t zh --preserve '`[^`]*`'
```

Успешный ответ:

```json
{
  "text": "你好，世界",
  "detectedSource": "en",
  "targetLang": "zh",
  "provider": "google",
  "engine": "google",
  "latencyMs": 412,
  "requestId": "c506d3bb-8eb2-451a-9e84-f2be63833f79",
  "chars": 11
}
```

Ошибка:

```json
{
  "error": {
    "code": "PROVIDER_NOT_CONFIGURED",
    "message": "provider deepl is implemented but not configured; add credentials or a base_url",
    "retryable": false,
    "suggestedAction": "configure_a_provider"
  },
  "requestId": ""
}
```

## MCP-сервер

```json
{
  "mcpServers": {
    "translatex": {
      "command": "translatex",
      "args": ["mcp"]
    }
  }
}
```

Сервер читает те же `config.toml` и `credentials.toml`, что и CLI. `tools/call` возвращает `content` и `structuredContent` по спецификации MCP; при ошибке выставляется `isError`.

| Инструмент | Вход | Выход |
| --- | --- | --- |
| `translate` | `TranslateRequest` | `TranslateResponse` |
| `providers` | `{}` | `{providers: ProviderInfo[]}` |
| `languages` | `{provider?: string}` | `{provider, languages: string[]}` |
| `health` | `{}` | `{healthy, providers: HealthRecord[]}` |

## Провайдеры

Приоритет определяет порядок в цепочке. `символы` — лимит одного запроса до разбиения.

### Бесплатные веб-эндпоинты

| # | Провайдер | ID | Автоопределение | Символы |
| --- | --- | --- | --- | --- |
| 1 | Google Translate (web) | `google` | да | 5 000 |
| 2 | Microsoft Edge Translator | `edge` | да | 50 000 |
| 3 | Iciba | `iciba` | да | 3 000 |
| 4 | iFlytek iFlyrec | `iflyrec` | да | 2 000 |
| 4 | ModernMT | `modernmt` | да | 5 000 |
| 5 | Volcengine | `volcengine` | да | 5 000 |
| 5 | Lara Translate | `lara` | да | 500 |
| 6 | Lingvanex | `lingvanex` | нет | 1 000 |
| 7 | Mirai Translate | `mirai` | да | 500 | пары только с японским |
| 8 | Papago (Naver) | `papago` | да | 1 000 |
| 9 | Reverso Context | `reverso` | нет | 2 000 |
| 10 | Sogou | `sogou` | да | 5 000 |
| 11 | MyMemory | `mymemory` | нет | 5 000 |
| 12 | SysTran | `systran` | нет | 5 000 |
| — | LibreTranslate | `libretranslate` | да | 5 000 |

Ограничения: `iflyrec` работает только с парами «китайский ↔ другой», `mirai` ориентирован на японский, `libretranslate` требует собственный `base_url`.

### Официальные API (20-27) и LLM (40-42)

`deepl`, `baidu`, `googlecloud`, `azure`, `aliyun`, `caiyun`, `niutrans`, `tencent`, `youdao` — при наличии ключей. `openai`, `anthropic`, `deepseek` — последний резерв.

## Статус каналов на 2026-11-03

| Статус | Провайдеры |
| --- | --- |
| Работают полностью | `google`, `edge`, `iciba`, `modernmt`, `lara`, `volcengine`, `lingvanex`, `papago`, `sogou`, `mymemory`, `systran` |
| Только часть пар | `iflyrec`, `mirai` |
| Троттлинг под нагрузкой | `sogou` — `RATE_LIMITED` с `retryAfterSecs` |
| Блокировка по отпечатку TLS | `reverso` — Cloudflare отклоняет rustls JA3 |
| Только свой сервер | `libretranslate` |
| Выведен вышестоящим | `iflytek` — псевдоним `iflyrec` |

Проверяйте на своей сети: `translatex health --format json`.

## Конфигурация

`~/.config/translatex/config.toml`:

```toml
chain = ["edge", "google", "iciba", "deepl", "openai"]
default_target = "zh"
timeout_ms = 20000
retries = 1

[provider.libretranslate]
base_url = "http://localhost:5000"
```

`~/.config/translatex/credentials.toml`:

```toml
[deepl.default]
api_key = "..."

[deepl.backup]
api_key = "..."
```

Порядок разрешения: переменные окружения → файл учётных данных → системный keyring (`@keyring`) → бесплатные эндпоинты.

## CLI

| Команда | Назначение |
| --- | --- |
| `translate [TEXT]` | Один запрос, при отсутствии текста читает stdin |
| `batch` | NDJSON на входе и выходе, порядок сохраняется |
| `providers` | Возможности и состояние конфигурации |
| `languages` | Языковые коды |
| `keys [PROVIDER]` | Состояние cooldown, `--reset` для сброса |
| `health` | Проверка всех настроенных провайдеров |
| `schema` | JSON Schema (draft 2020-12) |
| `manifest` | Манифест инструментов MCP |
| `mcp` | MCP-сервер по stdio |
| `init` | Шаблон конфигурации |
| `completions SHELL` | Автодополнение |

Коды возврата: `0` успех, `1` ошибка запроса, `2` ошибка конфигурации, `3` цепочка исчерпана, `4` сетевая ошибка.

## Документация и лицензия

- [docs/en/OVERVIEW.md](./docs/en/OVERVIEW.md) — английская документация
- [docs/zh-cn/OVERVIEW.md](./docs/zh-cn/OVERVIEW.md) — китайская документация
- [AGENTS.md](./AGENTS.md) — интеграция с агентами

MIT OR Apache-2.0
