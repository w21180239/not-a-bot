**简体中文** · [English](../../en/cli/01-commands.md)

# CLI — 命令参考

## `translate`

```bash
translatex translate "Hello world" -t zh
translatex translate -t zh --json '{"text":"Hello","sourceLang":"en"}'
echo "Hello" | translatex translate -t ja
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

## `batch`

```bash
translatex batch --file jobs.ndjson --parallel 4
cat jobs.ndjson | translatex batch --parallel 8
```

输入为每行一个 `TranslateRequest`。输出每行一个对象，虽然并发执行，但会按输入顺序排好：

```json
{"index":0,"ok":true,"result":{…}}
{"index":1,"ok":false,"error":{"code":"RATE_LIMITED",…}}
```

格式错误的行会跳过并在 stderr 警告，批次继续执行。

## `providers`

```bash
translatex providers --format json
translatex providers --format text
```

JSON 形式为 `{providers: ProviderInfo[]}`；文本形式为 `name<TAB>status`，方便 shell 处理。

## `languages`

```bash
translatex languages                       # 全部规范语言码
translatex languages --provider google     # 该 Provider 声明的集合
```

Provider 集合里的 `*` 表示接受任意规范语言码。未知名字返回 `PROVIDER_NOT_FOUND`；已实现但未配置返回 `PROVIDER_NOT_CONFIGURED`。

## `keys`

```bash
translatex keys                    # 已配置 Provider 的冷却与失败状态
translatex keys deepl              # 单个 Provider
translatex keys --reset            # 清空全部冷却文件
```

## `health`

```bash
translatex health --format json
```

用真实接口探测每个已注册 Provider，返回 `{healthy, providers: HealthRecord[]}`；有已配置 Provider 不健康时进程以非零退出。

## `schema`、`manifest`、`init`、`completions`

```bash
translatex schema      > schema.json
translatex manifest    > manifest.json
translatex init --path ~/.config/translatex/config.toml
translatex completions bash
```

`schema` 输出 draft 2020-12 定义：`TranslateRequest`、`TranslateResponse`、`ErrorResponse`、`ProviderInfo`、`HealthRecord`、`AttemptRecord`。

## 导航

- ← [CLI 概览](OVERVIEW.md)
- → [配置文件](02-config-files.md)
