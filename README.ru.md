<div align="center">

# translatex

**Гуманизатор AI-текста в командной строке: переписывает текст так, чтобы он читался как написанный человеком. Плюс бесплатный перевод для агента: один бинарник, без API-ключа.**

Перепишите сгенерированный текст на своей подписке Claude или OpenCode. Имена, числа и код остаются на месте, а если что-то потерялось, translatex скажет об этом. Тот же бинарник на 6 МБ переводит через 28 движков (14 из них бесплатные веб-эндпоинты без ключа) и поставляется с одним skill-файлом, который подключают 24 coding-агента.

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#документация-и-лицензия)
[![Tests](https://img.shields.io/badge/tests-302-brightgreen.svg)](https://github.com/w21180239/translatex)
[![Providers](https://img.shields.io/badge/providers-28-blueviolet.svg)](#провайдеры)
[![Binary](https://img.shields.io/badge/binary-6%20MB-lightgrey.svg)](#установка)
[![Agent-native](https://img.shields.io/badge/agent--native-JSON-purple.svg)](#поддерживаемые-harness)

`translatex roundtrip "<текст с AI-интонацией>" --from en --preset standard`

[English](./README.md) · [简体中文](./README.zh-cn.md) · [Русский](./README.ru.md) · [Документация](./docs/README.md)

</div>

---

## Содержание

- [Что вы получаете](#что-вы-получаете)
- [До и после](#до-и-после)
- [Какой пресет выбрать](#какой-пресет-выбрать)
- [Результаты замеров](#результаты-замеров)
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

## Что вы получаете

Гуманизатор AI-текста и бесплатный переводческий CLI в одном бинарнике.

- **Гуманизация AI-текста.** `translatex roundtrip` переписывает текст с AI-интонацией (шаблонные вступления, «not just X, it's Y», «it's worth noting»), чтобы он читался как написанный человеком. Работает на вашей подписке Claude Code или OpenCode: без API-ключа и без отдельного счёта. По сути это обратный перевод (back-translation), который делает LLM: через другой язык и обратно, с переписыванием на обоих шагах.
- **Бесплатный обратный перевод.** Пресет по умолчанию идёт через бесплатные веб-движки и ничего не стоит. Он меняет слова, но сохраняет строение предложений: быстро, переписывает слабее.
- **Факты на месте.** После каждого прогона имена, числа, ID, URL и код сверяются с оригиналом. Потерянное попадает в `missingFacts` вместе с предупреждением.
- **Шаг исправления.** `strong` добавляет последний шаг, на котором LLM предлагает мелкие правки. translatex применяет только точные и небольшие, так что переписанный текст остаётся, а потерянные факты возвращаются.
- **Перевод для агентов.** `translatex translate` гоняет до трёх бесплатных движков (Google, Edge, Papago, Sogou и другие), отвечает JSON и возвращает типизированные ошибки. Без регистрации и настройки.
- **Никаких обещаний про AI-детекторы.** Цель: текст, который читается как написанный человеком, с сохранёнными фактами. Ни один метод не делает текст надёжно необнаруживаемым, и translatex этого не обещает.

Ищете AI humanizer, способ убрать AI-тон или переписать сгенерированный текст из скрипта или coding-агента? Этот CLI вызывают Claude Code, Codex, OpenCode, Cursor и ещё 20 harness, а переписывает DeepSeek, Haiku или любая модель из вашей подписки.

## До и после

Исходный текст: абзац с AI-интонацией, именем и числом.

```
In today's fast-paced landscape, Aroha's team isn't just shipping a feature, they're fundamentally transforming how 4,200 customers manage their invoices. It's worth noting that this comprehensive solution leverages cutting-edge automation to empower finance teams and unlock their full potential.
```

```bash
translatex roundtrip "<абзац выше>" --from en --preset standard --format json
```

Результат `--preset standard` (OpenCode CLI с DeepSeek, 31 с на два шага), один прогон 2026-11-11, основные поля:

```json
{
  "text": "Aroha's team is doing more than shipping one feature: they're changing how 4,200 customers manage their invoices. Automation takes the manual work off finance teams, so their time goes to work that's worth more.",
  "chain": ["zh", "en"],
  "missingFacts": [],
  "latencyMs": 31544
}
```

Вступление, оговорка и модные слова исчезли, Aroha и 4,200 уцелели. Одна контрастная конструкция («more than shipping one feature») осталась, так что результат хороший черновик, который человек ещё может улучшить.

Тот же абзац через пресет `light` по умолчанию (два бесплатных шага машинного перевода, 1,0 с), один прогон 2026-11-11:

```json
{
  "text": "In today's fast-paced environment, the Araha team is not just releasing features—they are fundamentally changing how their 4,200 customers manage their invoices. Notably, this integrated solution leverages cutting-edge automation technologies to empower finance teams and maximize their potential.",
  "chain": ["zh", "ja", "en"],
  "missingFacts": ["Aroha"],
  "warnings": ["fact check: missing Aroha"]
}
```

Машинный перевод сменил слова и оставил все предложения: «not just X» и «in today's» уцелели, а имя превратилось в Araha. Проверка фактов это поймала.

## Какой пресет выбрать

| Пресет | Что делает | Сила переписывания | Скорость | Потеряно фактов (10 текстов) | Нужно |
| --- | --- | --- | --- | --- | --- |
| `light` (по умолчанию) | Два шага машинного перевода (en, zh, ja, en) | Слабая: слова меняются, предложения нет | около 1 с | 5 | ничего |
| `standard` | LLM туда и обратно, оба шага «переведи и перепиши как человек» | Убирает структурные шаблоны | 8–14 с | 0 | подписка Claude или OpenCode |
| `strong` | LLM-шаги в zh и ja, шаги машинного перевода через fi и обратно, затем шаг исправления | Самое сильное переписывание, потерянные факты возвращены | около 25 с | 1 | подписка |

Для гуманизации английской прозы берите `standard`, для максимального переписывания `strong`, а когда важнее скорость или нулевая стоимость, `light`. Если в тексте есть код или ссылки, добавьте `--preserve @code --preserve @url`. `--depth 1-3` и `--chain` позволяют собрать другие цепочки вручную.

## Результаты замеров

Прогон 2026-11-11: по 10 текстов на конфигурацию, один прогон (5 английских текстов с AI-интонацией, 3 английских рабочих текста с множеством фактов, 2 китайских). Скрипт и корпус: [`benches/roundtrip-depth/`](./benches/roundtrip-depth/README.md).

| Конфигурация | Пересечение слов с оригиналом | Шаблонов осталось (из 10) | Потеряно фактов | Задержка |
| --- | --- | --- | --- | --- |
| light, 1 шаг (`--depth 1`, режим 0.1.x) | 0,556 | 7 | 5 | 0,5 с |
| light, 2 шага (режим 0.3) | 0,520 | 5 | 5 | 0,9 с |
| 3 шага машинного перевода (`--depth 3`) | 0,480 | 6 | 8 | 1,1 с |
| standard через opencode-cli (DeepSeek) | 0,446 | 0 | 0 | 14 с |
| standard через claude-cli (Haiku) | 0,535 | 0 | 0 | 8 с |
| strong через claude-cli / opencode-cli | 0,369 / 0,396 | 0 / 1 | 1 / 1 | 24 / 28 с |

Пересечение считается как Jaccard по значимым словам: чем меньше, тем сильнее переписано. Шаблоны: «not just/only X», «more than just», «it's worth noting», «in today's», «at its core», 不仅, 更是, 值得注意, 当今. Потерянные факты: имена, числа, ID и фрагменты кода, которые проверка отметила как отсутствующие.

Что показывают данные:

- Машинный перевод меняет слова и сохраняет строение предложений, поэтому «not just X, it's Y» переживает любую глубину машинного перевода.
- Даже один шаг машинного перевода искажает имена (Aroha в Aloha, Tom в Tomi) и раскрывает коды (NZD в «New Zealand dollars»).
- Третий шаг машинного перевода теряет больше фактов, чем даёт переписывания, поэтому по умолчанию два шага.
- Шаги LLM убирают структурные шаблоны и на этом корпусе сохранили все факты.
- Шаг машинного перевода после шага LLM может вернуть контраст («ei ainoastaan ... vaan myös» вернулось как «not only ... but also»), поэтому та же цепочка без шага исправления оставляет несколько шаблонов.
- Шаг исправления снизил потери фактов в этой цепочке с 3–5 до 1 и сохранил большую часть переписывания; это `strong`.
- Прежний вывод (2026-11-04): немецкий, французский и испанский почти не переписывают, китайский и японский переписывают сильнее всего.

Ограничения: 10 текстов и один прогон дают малую выборку; список шаблонов и проверка фактов эвристические; пересечение поощряет изменения, а не качество. AI-детекторы здесь не измерялись.

## Почему translatex

| Почему выбирают | Что это значит |
| --- | --- |
| **Гуманизация на вашей подписке** | `claude-cli` и `opencode-cli` запускают `claude -p` и `opencode run` на тарифе, который вы уже оплачиваете. Без API-ключа и нового аккаунта. |
| **Факты проверяются** | После каждого прогона перечислены имена, числа, ID, URL и код, которые не вернулись. |
| **Бесплатно, без пробного периода** | 14 публичных эндпоинтов — Google Translate, Bing/Edge, Papago, Reverso, Sogou и другие — без ключа, аккаунта и таблицы квот. |
| **Ноль настройки** | Поставил и переводишь. Конфиг нужен, чтобы закрепить провайдера или добавить ключи, а не чтобы начать. |
| **Быстрый по устройству** | До трёх провайдеров идут гонкой: первый успешный ответ побеждает, остальные отменяются, зависший эндпоинт не задерживает запрос. |
| **Agent-native** | JSON или NDJSON в stdout, логи в stderr, коды возврата, типизированные ошибки с `retryable` и `suggestedAction`, а `manifest` перечисляет все операции. |
| **Без MCP-сервера** | Интеграция — это CLI плюс один skill-документ: ни процесса, ни порта, ни протокола, ничего синхронизировать. |
| **Больше ключей — стабильнее** | DeepL, Azure, Google Cloud, Baidu, Tencent и ещё четыре сервиса подключаются, когда нужен SLA; ключи ротируются и остывают сами. |
| **Нечего хостить** | Один статический бинарник на 6 МБ: без рантайма, Docker и базы данных. Ноутбук, CI или контейнер. |
| **Код не страдает** | Regex-фрагменты или именованные шаблоны `@code` и `@url` маскируются до машинного перевода и восстанавливаются после; сбой восстановления валит попытку, а не выдаёт испорченный текст. |

Готов к shell-скриптам, CI и coding-агентам. Если вы как раз собирались регистрировать переводческий API или вставлять AI-текст в сайт-переписчик — начните отсюда.

### Как устроена цепочка

```
запрос ─▶ приоритет 1-12   бесплатные публичные эндпоинты (без ключей)
       ─▶ приоритет 20-27  официальные API                 (только при наличии ключей)
       ─▶ приоритет 40-42  LLM                             (крайний случай, по желанию)
       ─▶ приоритет 60-61  claude-cli, opencode-cli        (локальные CLI агентов, только по имени)
```

Меньшее число — выше приоритет. Каждая попытка фиксируется, поэтому при ошибке видно, какие движки проверялись и почему отказали.

## Возможности

| Возможность | Что даёт |
| --- | --- |
| **Гуманизация / de-AI** | `roundtrip` идёт через один или несколько шагов и обратно. Шаги машинного перевода бесплатны, шаги LLM пересобирают предложения. Пресеты `light`, `standard`, `strong` или свой `--chain`. |
| **Проверка фактов** | После каждого прогона `missingFacts` и предупреждение: имена, числа, ID, URL и код, потерянные в результате. |
| **Шаг исправления** | `<llm>:fix` просит у LLM мелкие правки и применяет только точные. Потерянные факты возвращаются всегда, остальные правки делят общий бюджет, так что оригинал не скопировать обратно. |
| **28 провайдеров** | 14 бесплатных веб-эндпоинтов, 9 официальных API, 5 LLM (OpenAI, Anthropic, DeepSeek и локальные `claude-cli` и `opencode-cli`). LibreTranslate включается через `base_url`. |
| **Цепочка резерва** | Сортировка по приоритету; до трёх провайдеров одновременно, поэтому зависший эндпоинт не скрывает здоровые. Порядок `--fallback` сохраняется. |
| **Ротация ключей** | Несколько ключей на провайдера, round-robin, состояние cooldown общее для процессов в `~/.config/translatex/cache/`. |
| **Структурированный вывод** | JSON/NDJSON в stdout, логи в stderr, коды возврата для ветвления в агенте. |
| **Защита фрагментов** | Regex-фрагменты или `@code` и `@url` маскируются до машинного перевода и восстанавливаются после; шагам LLM передают список фрагментов и проверяют результат. |
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
translatex translate 'Run `make test` before pushing.' -t zh --preserve @code

# Гуманизация AI-текста: LLM через китайский и обратно (ваша подписка Claude или OpenCode)
translatex roundtrip "In today's fast-paced landscape, it's important to note that …" --from en --preset standard --preserve @code

# Бесплатно и быстро: два шага машинного перевода, затем прочитать missingFacts
translatex roundtrip "In today's fast-paced landscape, it's important to note that …" --from en
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

`roundtrip` переводит текст через один или несколько шагов и возвращает его на исходный язык. Язык с другим строем заставляет текст пересобраться, поэтому шаблонные формулировки генерации не возвращаются как были. Шаги машинного перевода делают это попутно, а шаги LLM получают прямую инструкцию переводить и переписывать как человек.

```bash
translatex roundtrip "<text>" --from en --preset standard --preserve @code --format json
translatex roundtrip "<text>" --from en --depth 3                    # zh, ja, fi и обратно, только машинный перевод
translatex roundtrip "<text>" --from en --chain "opencode-cli:zh,edge:en,opencode-cli:fix"
translatex roundtrip --from en --pivot ja < draft.md                 # псевдоним: translatex polish
```

- **Выбор шагов.** Побеждает первое совпадение: `--chain "<[engine:]lang,...>"`, `--preset <name>`, `--depth 1-3`, `--pivot <lang>`, затем `roundtrip_preset` из конфига (по умолчанию `light`). Если цепочка не возвращается на исходный язык, шаг назад добавляется сам.
- **LLM-провайдеры.** `claude-cli` запускает `claude -p` на вашей подписке Claude (модель по умолчанию haiku), `opencode-cli` запускает `opencode run` на вашей квоте OpenCode (модель по умолчанию `opencode-go/deepseek-v4.1-flash`). Ключ не нужен, шаг занимает секунды, запускаются они только по имени: `auto` их не выбирает. `roundtrip_llm` (по умолчанию `opencode-cli`) задаёт движок для пресетов `standard` и `strong`.
- **Привязка к оригиналу.** Шаги LLM получают и исходный текст: сначала минимальной правкой восстанавливают смысл, потерянный ранними шагами машинного перевода, потом переписывают.
- **Шаг исправления.** `<llm>:fix` возвращает JSON-список мелких правок (`from`, `to`, `why`). translatex применяет только точные и небольшие. Правки, возвращающие потерянный факт, применяются всегда; остальные делят небольшой бюджет, так что шаг не скопирует оригинал обратно. Каждая правка записана в `edits` с `kind` (`fact` или `meaning`), `applied` и `reason`.
- **Проверка фактов.** `missingFacts` перечисляет имена, числа, ID, фрагменты кода и URL, которые есть в оригинале и отсутствуют в результате (числа сравниваются по цифрам, имена по целым словам), и добавляется предупреждение `fact check:`. Имя, совпадающее с обычным словом начала предложения, может проскочить.
- **Предпочтительные движки.** Названный движок машинного перевода предпочитается, но не закрепляется. При повторяемой ошибке (лимит, таймаут, сеть, квота) шаг переходит на другой движок, обходя движки соседних шагов, и добавляет предупреждение. Шаги LLM не переключаются.
- **Плейсхолдеры.** Шаги машинного перевода маскируют защищённые фрагменты. Шаги LLM получают фрагменты как есть, список в промпте, а пропавший фрагмент даёт предупреждение.

Для кода и ссылок используйте `--preserve @code` и `--preserve @url`, а не regex вроде `` `[^`]*` ``: в Windows npm-обёртка `.cmd` идёт через cmd.exe, и он вырезает `^`, когда Python или Node запускают translatex без shell.

Поля ответа: `text`, `detectedSource`, `chain` (пройденные языки), `hops` (`from`, `to`, `provider`, `text`, `latencyMs`, у шага исправления ещё `edits`), `provider` (последний шаг), `missingFacts`, `warnings`, `latencyMs`, `requestId`, `chars`.

Юридические тексты, API-справки, конфигурацию и числа, которые читатель будет проверять, через roundtrip не прогоняйте: каждый шаг перефразирует, а проверка фактов эвристическая. Цепочки, пресеты и правила правок: [docs/en/roundtrip/02-chains-presets-and-llm.md](./docs/en/roundtrip/02-chains-presets-and-llm.md). Замеры и метод: [docs/en/roundtrip/01-pivot-choice.md](./docs/en/roundtrip/01-pivot-choice.md).

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
| 60 | Claude Code CLI | `claude-cli` | без ключа; запускает `claude -p` на вашей подписке Claude, опционально `model` (по умолчанию haiku) и `bin` |
| 61 | OpenCode CLI | `opencode-cli` | без ключа; запускает `opencode run` на вашей квоте OpenCode, опционально `model` (по умолчанию `opencode-go/deepseek-v4.1-flash`) и `bin` |

`claude-cli` и `opencode-cli` не входят в автоматическую цепочку. Они запускаются по имени: шаг `roundtrip`, пресет, `--provider` или `--fallback`.

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
roundtrip_preset = "light"          # light, standard, strong
roundtrip_llm = "opencode-cli"   # или "claude-cli"; движок для пресетов standard и strong
log = "warn"

[provider.libretranslate]
base_url = "http://localhost:5000"

[provider.claude-cli]
model = "haiku"
bin = "claude"

[provider.opencode-cli]
model = "opencode-go/deepseek-v4.1-flash"
bin = "opencode"
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
| `roundtrip [TEXT]` | Через один или несколько шагов и обратно, гуманизация AI-текста (`--chain`, `--preset`, `--depth`, `--pivot`) |
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
- [docs/en/roundtrip/OVERVIEW.md](./docs/en/roundtrip/OVERVIEW.md) — обратный перевод: цепочки, пресеты, замеры
- [benches/roundtrip-depth/README.md](./benches/roundtrip-depth/README.md) — скрипт замеров и результаты
- [integrations/README.md](./integrations/README.md) — подключение к 24 harness
- [AGENTS.md](./AGENTS.md) — интеграция с агентами

MIT OR Apache-2.0

Используйте на выбор [Apache License 2.0](LICENSE-APACHE) или [MIT](LICENSE-MIT).
