<div align="center">

# translatex

**Бесплатный перевод для агента: одна команда, без API-ключа, без регистрации, без настройки.**

Один статический бинарник на 6 МБ, 26 движков перевода в параллельной гонке. JSON на выходе, типизированные ошибки, 24 coding-агента подключаются одним skill-файлом.

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#документация-и-лицензия)
[![Tests](https://img.shields.io/badge/tests-245-brightgreen.svg)](https://github.com/w21180239/translatex)
[![Providers](https://img.shields.io/badge/providers-26-blueviolet.svg)](#провайдеры)
[![Binary](https://img.shields.io/badge/binary-6%20MB-lightgrey.svg)](#установка)
[![Agent-native](https://img.shields.io/badge/agent--native-JSON-purple.svg)](#поддерживаемые-harness)

`translatex translate "Hello world" -t zh`

[English](./README.md) · [简体中文](./README.zh-cn.md) · [Русский](./README.ru.md) · [Документация](./docs/README.md)

</div>

---

## Содержание

- [Почему translatex](#почему-translatex)
- [Возможности](#возможности)
- [Установка](#установка)
- [Быстрый старт](#быстрый-старт)
- [Поддерживаемые harness](#поддерживаемые-harness)
- [Обратный перевод](#обратный-перевод)
- [Провайдеры](#провайдеры)
- [Статус каналов](#статус-каналов-на-2026-11-03)
- [Конфигурация](#конфигурация)
- [CLI](#cli)
- [Ошибки](#ошибки)
- [Документация и лицензия](#документация-и-лицензия)

## Почему translatex

| Почему выбирают | Что это значит |
| --- | --- |
| **Бесплатно, без пробного периода** | 14 публичных эндпоинтов — Google Translate, Bing/Edge, Papago, Reverso, Sogou и другие — без ключа, аккаунта и таблицы квот. |
| **Ноль настройки** | Поставил и переводишь. Конфиг нужен, чтобы закрепить провайдера или добавить ключи, а не чтобы начать. |
| **Быстрый по устройству** | До трёх провайдеров идут гонкой: первый успешный ответ побеждает, остальные отменяются, зависший эндпоинт не задерживает запрос. |
| **Agent-native** | JSON или NDJSON в stdout, логи в stderr, коды возврата, типизированные ошибки с `retryable` и `suggestedAction`, а `manifest` перечисляет все операции. |
| **Без MCP-сервера** | Интеграция — это CLI плюс один skill-документ: ни процесса, ни порта, ни протокола, ничего синхронизировать. |
| **Убирает AI-тон** | `roundtrip` переводит через промежуточный язык и обратно, шаблонные формулировки такой путь не переживают. |
| **Больше ключей — стабильнее** | DeepL, Azure, Google Cloud, Baidu, Tencent и ещё четыре сервиса подключаются, когда нужен SLA; ключи ротируются и остывают сами. |
| **Нечего хостить** | Один статический бинарник на 6 МБ: без рантайма, Docker и базы данных. Ноутбук, CI или контейнер. |
| **Код не страдает** | Код, идентификаторы, URL и версии маскируются до перевода и восстанавливаются после; сбой восстановления валит попытку, а не выдаёт испорченный текст. |

Готов к shell-скриптам, CI и coding-агентам. Если вы как раз собирались регистрировать переводческий API — начните отсюда.

### Как устроена цепочка

```
запрос ─▶ приоритет 1-12   бесплатные публичные эндпоинты (без ключей)
       ─▶ приоритет 20-27  официальные API                 (только при наличии ключей)
       ─▶ приоритет 40-42  LLM                             (крайний случай, по желанию)
```

Меньшее число — выше приоритет. Каждая попытка фиксируется, поэтому при ошибке видно, какие движки проверялись и почему отказали.

## Возможности

| Возможность | Что даёт |
| --- | --- |
| **26 провайдеров** | 14 бесплатных веб-эндпоинтов, 9 официальных API, 3 LLM. LibreTranslate включается через `base_url`. |
| **Цепочка резерва** | Сортировка по приоритету; до трёх провайдеров одновременно, поэтому зависший эндпоинт не скрывает здоровые. Порядок `--fallback` сохраняется. |
| **Обратный перевод** | `roundtrip` переводит через промежуточный язык и обратно, переписывая машинную речь. Промежуточный язык выбран замерами. |
| **Ротация ключей** | Несколько ключей на провайдера, round-robin, состояние cooldown общее для процессов в `~/.config/translatex/cache/`. |
| **Структурированный вывод** | JSON/NDJSON в stdout, логи в stderr, коды возврата для ветвления в агенте. |
| **Защита фрагментов** | Код, переменные и URL маскируются до перевода и восстанавливаются после; сбой восстановления запускает fallback. |
| **24 harness** | Claude Code, Codex, Cursor, Zed и ещё 20 читают один skill или блок инструкций; сервер запускать не нужно. Пути: [integrations/](./integrations/README.md). |
| **Работающий таймаут** | `timeoutMs` из запроса (или `timeout_ms` в конфиге) ограничивает весь вызов провайдера. |
| **Понятные ошибки** | `code`, `retryable`, `retryAfterSecs`, `suggestedAction` в каждом ответе об ошибке. |

## Установка

```bash
npm install -g translatex                 # готовый бинарник для платформы
cargo install translatex                 # или сборка из исходников, Rust 1.89+
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

## Поддерживаемые harness

Интеграция — это команда плюс один документ: skill там, где хост умеет skills,
файл правил там, где читают правила, блок инструкций там, где держат инструкции
проекта. Сервер запускать не нужно.

<div align="center">
<table>
<tr>
<td align="center" width="150"><img src="repo-assets/agents/claude-code.png" width="44" alt="Claude Code"><br/><b>Claude Code</b><br/><sub>skill · плагин</sub></td>
<td align="center" width="150"><img src="repo-assets/agents/codex.png" width="44" alt="Codex CLI"><br/><b>Codex CLI</b><br/><sub>skill · AGENTS.md</sub></td>
<td align="center" width="150"><img src="repo-assets/agents/gemini-cli.png" width="44" alt="Gemini CLI"><br/><b>Gemini CLI</b><br/><sub>skill · GEMINI.md</sub></td>
<td align="center" width="150"><img src="repo-assets/agents/opencode.png" width="44" alt="OpenCode"><br/><b>OpenCode</b><br/><sub>skill · AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/cursor.png" width="44" alt="Cursor"><br/><b>Cursor</b><br/><sub>.cursor/rules</sub></td>
<td align="center"><img src="repo-assets/agents/cline.png" width="44" alt="Cline"><br/><b>Cline</b><br/><sub>.clinerules</sub></td>
<td align="center"><img src="repo-assets/agents/roo-code.png" width="44" alt="Roo Code"><br/><b>Roo Code</b><br/><sub>.roo/rules</sub></td>
<td align="center"><img src="repo-assets/agents/kilo-code.png" width="44" alt="Kilo Code"><br/><b>Kilo Code</b><br/><sub>.kilocode/rules</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/continue.png" width="44" alt="Continue"><br/><b>Continue</b><br/><sub>.continue/rules</sub></td>
<td align="center"><img src="repo-assets/agents/jetbrains.png" width="44" alt="JetBrains AI"><br/><b>JetBrains AI</b><br/><sub>.aiassistant/rules</sub></td>
<td align="center"><img src="repo-assets/agents/zed.png" width="44" alt="Zed"><br/><b>Zed</b><br/><sub>.rules</sub></td>
<td align="center"><img src="repo-assets/agents/aider.png" width="44" alt="Aider"><br/><b>Aider</b><br/><sub>CONVENTIONS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/qwen-code.png" width="44" alt="Qwen Code"><br/><b>Qwen Code</b><br/><sub>QWEN.md</sub></td>
<td align="center"><img src="repo-assets/agents/crush.png" width="44" alt="Crush"><br/><b>Crush</b><br/><sub>CRUSH.md</sub></td>
<td align="center"><img src="repo-assets/agents/goose.png" width="44" alt="Goose"><br/><b>Goose</b><br/><sub>инструкции</sub></td>
<td align="center"><img src="repo-assets/agents/deepseek-harness.png" width="44" alt="DeepSeek Harness"><br/><b>DeepSeek Harness</b><br/><sub>AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/mimo-code.svg" width="44" alt="MiMo Code"><br/><b>MiMo Code</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/atomcode.png" width="44" alt="AtomCode"><br/><b>AtomCode</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/openclaw.png" width="44" alt="OpenClaw"><br/><b>OpenClaw</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/hermes-agent.png" width="44" alt="Hermes Agent"><br/><b>Hermes Agent</b><br/><sub>AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/pi.png" width="44" alt="Pi"><br/><b>Pi</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/reasonix.png" width="44" alt="Reasonix"><br/><b>Reasonix</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/zcode.png" width="44" alt="ZCode"><br/><b>ZCode</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/qwenpaw.svg" width="44" alt="QwenPaw"><br/><b>QwenPaw</b><br/><sub>инструкции</sub></td>
</tr>
</table>

<i>24 harness, два документа. Любой другой хост просто вызывает CLI.</i>
</div>

```bash
# Хосты со skills
mkdir -p .claude/skills/translatex && cp integrations/skill/translatex/SKILL.md .claude/skills/translatex/
mkdir -p ~/.config/opencode/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.config/opencode/skills/translatex/
mkdir -p ~/.agents/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.agents/skills/translatex/   # Codex, Gemini CLI

# Остальные читают тот же текст под своим именем файла
cat integrations/guidance/AGENTS.md >> AGENTS.md
```

| Что нужно | Где смотреть |
| --- | --- |
| Точный путь и команда для каждого harness | [integrations/README.md](./integrations/README.md) |
| Четыре канала и проверка установки | [docs/en/integrations/02-supported-harnesses.md](./docs/en/integrations/02-supported-harnesses.md) |
| Установка плагина Claude Code одной командой | [integrations/claude-code/README.md](./integrations/claude-code/README.md) |

## Обратный перевод

Перевод через язык с другим строем и обратно заставляет текст пересобраться,
поэтому шаблонные формулировки не выживают:

```bash
translatex roundtrip "Our comprehensive solution leverages cutting-edge technology to empower teams." --from en --format json
```

По умолчанию промежуточный язык — китайский для некитайского текста и английский
для китайского. Выбор сделан замерами: немецкий, французский и испанский
возвращают текст почти дословно, японский переписывает так же сильно, как
китайский, русский — сильнее всех, но и уводит дальше всех. `--pivot ja` — для
технического английского. Юридические тексты, API-справки и всё, где важны
цифры и имена, через roundtrip не прогоняйте.

Он переписывает формулировки, но сохраняет предложения: вводная фраза или
страховка вроде «it's important to note» переживает любой язык. Такие куски
удаляют до перевода и говорят, что именно удалили.

## Провайдеры

Приоритет определяет порядок в цепочке. `Символы` — лимит одного запроса до разбиения.

### Бесплатные веб-эндпоинты

| # | Провайдер | ID | Автоопределение | Символы | Примечание |
| --- | --- | --- | --- | --- | --- |
| 1 | Google Translate (web) | `google` | да | 5 000 | `translate_a/single`, best effort |
| 2 | Microsoft Edge Translator | `edge` | да | 50 000 | без ключа |
| 3 | Iciba | `iciba` | да | 3 000 | подпись AES-ECB |
| 4 | iFlytek iFlyrec | `iflyrec` | да | 2 000 | только пары с китайским |
| 4 | ModernMT | `modernmt` | да | 5 000 | md5 + timestamp |
| 5 | Volcengine | `volcengine` | да | 5 000 | `crx/translate/v1/` |
| 5 | Lara Translate | `lara` | да | 500 | дневной лимит на IP |
| 6 | Lingvanex | `lingvanex` | нет | 1 000 | `API_BEARER_TOKEN` |
| 7 | Mirai Translate | `mirai` | да | 500 | только пары с японским, пробный ключ на каждый вызов |
| 8 | Papago (Naver) | `papago` | да | 1 000 | cookie-primed detect + form POST |
| 9 | Reverso Context | `reverso` | нет | 2 000 | трёхбуквенные коды; не входит в авто-цепочку |
| 10 | Sogou | `sogou` | да | 5 000 | md5-подпись; лимит на IP даёт `RATE_LIMITED` |
| 11 | MyMemory | `mymemory` | нет | 5 000 | коды RFC 3066, дневной лимит |
| 12 | SysTran | `systran` | нет | 5 000 | OAuth client credentials из бандла |
| — | LibreTranslate | `libretranslate` | да | 5 000 | только свой сервер: `[provider.libretranslate] base_url` |

### Официальные API — резерв при наличии ключей

| # | Провайдер | ID | Учётные данные |
| --- | --- | --- | --- |
| 20 | DeepL | `deepl` | `api_key` |
| 20 | Baidu | `baidu` | `appid`, `secret_key` |
| 21 | Google Cloud Translation | `googlecloud` | `api_key` |
| 22 | Azure Translator | `azure` | `api_key`, `region` |
| 23 | Alibaba Cloud | `aliyun` | `access_key_id`, `access_key_secret` |
| 24 | Caiyun | `caiyun` | `token` |
| 25 | Niutrans | `niutrans` | `apikey` |
| 26 | Tencent | `tencent` | `secret_id`, `secret_key` |
| 27 | Youdao | `youdao` | `app_key`, `app_secret` |

### LLM — последний резерв

| # | Провайдер | ID | Учётные данные |
| --- | --- | --- | --- |
| 40 | OpenAI | `openai` | `api_key`, опционально `base_url`, `model` |
| 41 | Anthropic | `anthropic` | `api_key`, опционально `model` |
| 42 | DeepSeek | `deepseek` | `api_key`, опционально `model` |

## Статус каналов на 2026-11-03

| Статус | Провайдеры |
| --- | --- |
| Работают полностью | `google`, `edge`, `iciba`, `modernmt`, `lara`, `volcengine`, `lingvanex`, `papago`, `sogou`, `mymemory`, `systran` |
| Только часть пар | `iflyrec`, `mirai` |
| Троттлинг под нагрузкой | `sogou` — `RATE_LIMITED` с `retryAfterSecs` |
| Блокировка по отпечатку TLS | `reverso` — Cloudflare отклоняет rustls JA3; остаётся доступным явно, но пропускается авто-цепочкой |
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
race_width = 3
roundtrip_pivot = "auto"
log = "warn"

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

Порядок: ярлыки из `credentials.toml` идут в порядке файла, переменная `TRANSLATEX_<PROVIDER>_<FIELD>` добавляет ярлык `env` после них, а поле со значением `@keyring` читается из системного keyring. Бесплатным эндпоинтам ничего из этого не нужно.

## CLI

| Команда | Назначение |
| --- | --- |
| `translate [TEXT]` | Один запрос, при отсутствии текста читает stdin |
| `roundtrip [TEXT]` | Перевод через промежуточный язык и обратно |
| `batch` | NDJSON на входе и выходе, порядок сохраняется |
| `providers` | Возможности и состояние конфигурации |
| `languages` | Языковые коды |
| `keys [PROVIDER]` | Состояние cooldown, `--reset` для сброса |
| `health` | Проверка всех настроенных провайдеров |
| `schema` | JSON Schema (draft 2020-12) |
| `manifest` | Машиночитаемый список операций CLI |
| `init` | Шаблон конфигурации |
| `completions SHELL` | Автодополнение |

Коды возврата: `0` успех, `1` ошибка запроса, `2` ошибка конфигурации, `3` цепочка исчерпана, `4` сетевая ошибка.

## Ошибки

| `code` | Повтор | Что делать агенту |
| --- | --- | --- |
| `RATE_LIMITED` | да | Подождать `retryAfterSecs` и повторить |
| `QUOTA_EXHAUSTED` | нет | Сменить ключ или дождаться квоты |
| `NETWORK_ERROR` | да | Экспоненциальная пауза |
| `UPSTREAM_ERROR` | только 5xx | Повтор на 5xx, исправить вход на 4xx |
| `TIMEOUT` | да | Увеличить `timeoutMs` или сменить провайдера |
| `AUTH_FAILED` | нет | Обновить учётные данные |
| `PROVIDER_NOT_CONFIGURED` | нет | Добавить ключ |
| `PROVIDER_NOT_FOUND` | нет | Исправить имя провайдера |
| `AUTO_DETECT_UNSUPPORTED` | нет | Передать `sourceLang` |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | нет | Сменить провайдера или язык |
| `INVALID_INPUT` | нет | Исправить запрос |
| `CHAIN_EXHAUSTED` | нет | Смотреть `attempts` в `--verbose` |

Полный справочник: [docs/en/api/OVERVIEW.md](./docs/en/api/OVERVIEW.md).

## Документация и лицензия

- [docs/en/OVERVIEW.md](./docs/en/OVERVIEW.md) — английская документация
- [docs/zh-cn/OVERVIEW.md](./docs/zh-cn/OVERVIEW.md) — китайская документация
- [docs/en/roundtrip/01-pivot-choice.md](./docs/en/roundtrip/01-pivot-choice.md) — замеры промежуточного языка
- [integrations/README.md](./integrations/README.md) — подключение к 24 harness
- [AGENTS.md](./AGENTS.md) — интеграция с агентами

MIT OR Apache-2.0

Используйте на выбор [Apache License 2.0](LICENSE-APACHE) или [MIT](LICENSE-MIT).
