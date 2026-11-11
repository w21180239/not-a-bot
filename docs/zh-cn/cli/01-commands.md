**简体中文** · [English](../../en/cli/01-commands.md)

# CLI — 命令参考

[← 领域概览](OVERVIEW.md) · [文档索引](../README.md)

## `translate`

```bash
not-a-bot translate "Hello world" -t zh
not-a-bot translate -t zh --json '{"text":"Hello","sourceLang":"en"}'
echo "Hello" | not-a-bot translate -t ja
```

| 参数 | 默认 | 说明 |
| --- | --- | --- |
| `TEXT` | stdin | 待翻译文本 |
| `-t, --to` | 必填 | 目标语言码 |
| `-s, --from` | `auto` | 源语言或 `auto` |
| `-p, --provider` | `auto` | 指定一个 Provider，或走整条链 |
| `--fallback` | 空 | 逗号分隔，指定 Provider 之后尝试的列表 |
| `--preserve` | 空 | 需要原样保留的正则，可重复 |
| `--timeout-ms` | 配置值 | 单次请求预算（毫秒） |
| `--request-id` | 自动生成 | 关联 id，回显并用于冷却 |
| `--json` | — | 完整请求对象，覆盖上面的参数 |

需要知道的语义：

- `--provider deepl --fallback google,edge` 依次尝试 DeepL、Google、Edge。若指定 Provider 无法处理该请求（不支持自动检测、语言对不支持），会交给 fallback 列表而不是直接报错。
- `--preserve` 在链之前执行。还原失败记为一次失败尝试，继续下一个 Provider。
- `--from auto` 要求 Provider 支持检测。`autoDetect: false` 的 Provider 会被跳过；强行指定则返回 `AUTO_DETECT_UNSUPPORTED`。

## `rewrite`

```bash
not-a-bot rewrite "In today's fast-paced landscape, …" --from en --format json
not-a-bot polish draft.md --pivot ja
not-a-bot rewrite "…" --from en --preset standard --preserve @code
not-a-bot rewrite "…" --from en --chain "opencode-cli:zh,edge:en,opencode-cli:fix"
```

| 参数 | 默认 | 说明 |
| --- | --- | --- |
| `TEXT` | stdin | 待往返的文本 |
| `-s, --from` | `auto` | 源语言 |
| `--pivot` | `auto` | 中转语言 |
| `--chain` | — | 逗号分隔的跳 `[engine:]lang`；末尾没回到源语言时自动补一跳；`<llm>:fix` 为只修事实的修复跳 |
| `--preset` | 配置 `rewrite_preset`（`standard`） | `light`（两跳机器翻译）、`standard`（LLM 出去再回来，再做事实修复）、`strong`（LLM 出去再回来，不修复，可能丢事实）；后两者要求 `rewrite_llm` 是 LLM 引擎，否则 `INVALID_INPUT` |
| `--depth` | — | 1-3 跳机器翻译，依次经 zh、ja、fi（中文原文为 en、ja、fi） |
| `-p, --provider` | `auto` | 两跳都用指定 Provider |
| `--fallback` | 空 | 指定 Provider 之后尝试的列表 |
| `--preserve` | 空 | 需要原样保留的正则，或具名模式 `@code`、`@url`；可重复 |
| `--timeout-ms` | 配置值 | 每跳预算 |
| `--request-id` | 自动生成 | 关联 id |
| `--json` | — | 完整 `RewriteRequest`，覆盖其它参数 |

选跳规则，先匹配者生效：`--chain`、`--preset`、`--depth`、`--pivot`，最后是配置里的 `rewrite_preset`（默认 `standard`；`light` 是两跳机器翻译）。`claude-cli` 和 `opencode-cli` 只在某一跳、预设或 `--provider` 点名时运行。响应带 `chain`、`hops`、`missingFacts` 和 `warnings`；每次运行后都要读 `missingFacts`。链、预设与修复跳见 [rewrite/02-chains-presets-and-llm.md](../rewrite/02-chains-presets-and-llm.md)。

结果永远回到源语言。`pivot: auto` 对非中文用中文、对中文用英文；技术类英文的官方备选是 `--pivot ja`，见 [rewrite/01-pivot-choice.md](../rewrite/01-pivot-choice.md)。

中转语言等于源语言时会自动换成另一侧，因此 `--from en --pivot en` 实际是 `en -> zh -> en`，不会变成空操作。

## `lint`

```bash
not-a-bot lint draft.md
not-a-bot lint draft.md --source original.md --format json
cat draft.md | not-a-bot lint - --source original.md --strict
```

离线检查 AI 味和丢失的事实：不联网、不调模型、不读配置和凭据。命中只是提示你去看一眼，所以除非加 `--strict`，退出码恒为 `0`。

| 参数 | 默认 | 说明 |
| --- | --- | --- |
| `DRAFT` | 必填 | 草稿文件，`-` 表示 stdin |
| `--source FILE` | — | 原文；启用事实比对 |
| `--strict` | 关 | 有任何 tell 或任何 `missingFromDraft` 条目时退出 `1`（CI、pre-commit） |

tell 带 `kind`：`vocab`（词及其平实替换）、短语族（`deep saying`、`run-up`、`arguing with no one`、`dramatic closer`、`about the document`、`chatbot residue`、`meta wrapper`、`knowledge-limit`）、`staged-contrast`、`us-spelling`、`bold-label-bullet`、`horizontal-rule`、`repeated-openings`（行号 0）、`punctuation`（破折号、带空格的 en dash、带空格的 `--`、弯双引号），加 `--source` 时还有 `introduced-script`。事实比对覆盖代码片段、URL、路径、工单号、`@提及`、数字和人名：`missingFromDraft` 是原文有而草稿没有的 `kind:value`，`newInDraft` 反过来（人名不算新增）。数字改写成单词（`3` 变 `three`）算丢失。

`--format json`（全局默认）输出 `LintResponse`：

```json
{"tells":[{"kind":"vocab","line":1,"match":"delve","hint":"look at / dig into"}],"missingFromDraft":["number:5"],"newInDraft":[]}
```

`--format text` 输出与 humanizer `lint.py` 相同的行：每个 tell 一行 `N: kind: detail`，随后是 `missing-from-draft:`、`new-in-draft:` 行、`-- source ...` 汇总（词召回、长度比）和 `-- N words, ... N hits` 页脚。草稿或原文文件不存在时返回 `INVALID_INPUT`（退出 `1`）。

## `batch`

```bash
not-a-bot batch --file jobs.ndjson --parallel 4
cat jobs.ndjson | not-a-bot batch --parallel 8
```

输入为每行一个 `TranslateRequest`。输出每行一个对象，虽然并发执行，但会按输入顺序排好：

```json
{"index":0,"ok":true,"result":{…}}
{"index":1,"ok":false,"error":{"code":"RATE_LIMITED",…}}
```

格式错误的行会跳过并在 stderr 警告，批次继续执行。

## `providers`

```bash
not-a-bot providers --format json
not-a-bot providers --format text
```

JSON 形式为 `{providers: ProviderInfo[]}`；文本形式为 `name<TAB>status`，方便 shell 处理。

## `languages`

```bash
not-a-bot languages                       # 全部规范语言码
not-a-bot languages --provider google     # 该 Provider 声明的集合
```

Provider 集合里的 `*` 表示接受任意规范语言码。未知名字返回 `PROVIDER_NOT_FOUND`；已实现但未配置返回 `PROVIDER_NOT_CONFIGURED`。

## `keys`

```bash
not-a-bot keys                    # 已配置 Provider 的冷却与失败状态
not-a-bot keys deepl              # 单个 Provider
not-a-bot keys --reset            # 清空全部冷却文件
```

## `health`

```bash
not-a-bot health --format json
```

用真实接口探测每个已注册 Provider，返回 `{healthy, providers: HealthRecord[]}`；有已配置 Provider 不健康时进程以非零退出。

## `schema`、`manifest`、`init`、`completions`

```bash
not-a-bot schema      > schema.json
not-a-bot manifest    > manifest.json
not-a-bot init --path ~/.config/not-a-bot/config.toml
not-a-bot completions bash
```

`schema` 输出 draft 2020-12 定义：`TranslateRequest`、`TranslateResponse`、`ErrorResponse`、`ProviderInfo`、`HealthRecord`、`AttemptRecord`、`RewriteRequest`、`RewriteResponse`、`LintResponse`。

`manifest` 输出 `{interface: "cli", commands: [{name, command, description, inputSchema?}], skill, install}`，Agent 不必解析 `--help` 就能发现操作。

## 导航

- ← [CLI 概览](OVERVIEW.md)
- → [配置文件](02-config-files.md)
