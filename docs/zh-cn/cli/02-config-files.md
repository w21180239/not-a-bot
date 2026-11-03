**简体中文** · [English](../../en/cli/02-config-files.md)

# CLI — 配置与凭据文件

## 位置

| 文件 | 默认路径 | 覆盖方式 |
| --- | --- | --- |
| 配置 | `~/.config/translatex/config.toml` | `--config`、`TRANSLATEX_CONFIG`、`TRANSLATEX_HOME`、`XDG_CONFIG_HOME` |
| 凭据 | `~/.config/translatex/credentials.toml` | `--credentials`、`TRANSLATEX_CREDENTIALS` |
| 冷却状态 | `~/.config/translatex/cache/keys-<provider>.json` | 跟随配置目录 |

文件不存在不是错误，会使用默认值。

## `config.toml`

```toml
# 替换内置 auto 链
chain = ["edge", "google", "iciba", "deepl", "openai"]

default_target = "zh"
timeout_ms = 20000     # 单次 Provider 调用的整体预算
retries = 1            # 可重试错误的额外尝试次数（超时不重试）
race_width = 3         # 同时在飞的 Provider 数；1 为严格串行
roundtrip_pivot = "zh" # `translatex roundtrip` 的默认中转语言
concurrency = 4        # 预留：批次并发
log = "warn"

[provider.libretranslate]
base_url = "http://localhost:5000"

[provider.openai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"

[provider.azure]
region = "eastasia"
```

规则：

- 开启了 `deny_unknown_fields`。拼错字段会直接得到 `CONFIG_ERROR`，不会被静默忽略。
- `chain` 中的 Provider 按书写顺序排到最前；其余已注册的按优先级升序追加，因此网页接口仍然排在官方 API 之前。
- `timeout_ms`、`retries` 是请求默认值；请求里的 `timeoutMs` 覆盖超时。
- `race_width` 控制同时在飞的 Provider 数；`1` 恢复严格串行。`--race` 参数覆盖单次调用。
- 请求里写 `"pivot": "auto"` 时，`roundtrip_pivot` 就是默认中转语言。
- `[provider.<name>]` 只接受 `base_url`、`model`、`region`、`instance`、`tier`，其他键同样会被拒绝。

## `credentials.toml`

```toml
[deepl.default]
api_key = "..."

[deepl.backup]
api_key = "..."

[azure.default]
api_key = "..."
region = "eastasia"

[baidu.default]
appid = "..."
secret_key = "..."

[tencent.default]
secret_id = "..."
secret_key = "..."
```

节名是 `<provider>.<label>`。标签就是轮换顺序，会出现在 `keys` 输出与 `--verbose` 的尝试记录里。

## 优先级

后者覆盖前者：

1. 内置免费接口（不涉及凭据）。
2. `~/.config/translatex/credentials.toml`。
3. `TRANSLATEX_<PROVIDER>_<FIELD>` 环境变量。

字段名沿用 Provider 自己的命名：`TRANSLATEX_DEEPL_API_KEY`、`TRANSLATEX_AZURE_REGION`、`TRANSLATEX_BAIDU_SECRET_KEY`。

## 钥匙串占位符

任意字段都可以写 `"@keyring"`：

```toml
[deepl.default]
api_key = "@keyring"
```

此时从系统钥匙串读取服务 `translators` 下名为 `<provider>.<field>` 的条目。这与 Python `translators` 分支使用的账户布局一致，已有钥匙串可以直接复用。条目缺失会得到指名该账户的 `CONFIG_ERROR`。

## 校验

```bash
translatex providers --format json   # 每个 Provider 的 configured
translatex keys deepl --format json  # 标签与冷却状态
```

## 导航

- ← [CLI 命令](01-commands.md)
- → [环境变量](../env/01-variables.md)
