**简体中文** · [English](../../en/troubleshooting/01-common-issues.md)

# 故障排查 — 常见问题

[← 领域概览](OVERVIEW.md) · [文档索引](../README.md)

## `NO_PROVIDER`

链中没有任何 Provider 能处理这次请求。

| 原因 | 检查 | 处理 |
| --- | --- | --- |
| 该语言对把所有 Provider 都过滤掉了 | `not-a-bot providers --format json` | 换目标语言，或换覆盖该语言的 Provider |
| 配置的 `chain` 只列了未注册的 Provider | `not-a-bot providers` | 换成已注册 id，或删掉 `chain` |
| 只有需要 Key 的链且没有凭据 | `not-a-bot keys` | 添加凭据 |

## `PROVIDER_NOT_CONFIGURED`

id 存在于二进制里，但没有注册。对有 Key 的 Provider 意味着凭据不完整，例如百度需要同时有 `appid` 与 `secret_key`。核对字段名后：

```bash
not-a-bot providers --format json | jq '.providers[] | select(.name=="baidu")'
```

## `AUTO_DETECT_UNSUPPORTED`

你指定的 Provider 不能检测源语言：`lingvanex`、`mymemory`、`systran`、`reverso`，以及需要 Key 的 `aliyun`、`niutrans`、`tencent`。要么加 `-s <code>`，要么去掉 `--provider`，让链自己挑一个能检测的。

## 免费接口返回 `RATE_LIMITED`

几个网页接口按 IP 限流：

| Provider | 典型表现 |
| --- | --- |
| `sogou` | `errorCode 20`，以 `RATE_LIMITED` 报出，提示 300 秒 |
| `mymemory` | 每日字符上限，UTC 零点重置 |
| `lara` | 按 IP 的每日额度 |

等满 `retryAfterSecs`。若持续发生，把该 Provider 排到链后面：

```toml
chain = ["edge", "google", "iciba", "sogou"]
```

## `reverso` 总是失败且消息里是 HTML

Cloudflare 拒绝 rustls 客户端的 TLS 指纹。接口本身是活的，浏览器可用，所以这只在你显式要求 `reverso` 时发生 —— 它已被排除在自动链之外。没有靠请求头绕过的办法，换其它 Provider。

## 所有 Provider 都 `TIMEOUT`

1. 确认预算：`not-a-bot translate "hi" -t zh --timeout-ms 30000`。
2. 确认出口：`not-a-bot health --format json`。
3. 在代理后面时设置 `HTTPS_PROXY`，reqwest 会遵循。

记住超时按设计不原地重试，所以一个慢 Provider 只消耗一次预算，然后换下一跳。

## 译文与原文一模一样

静默失败看起来像成功。两道防线：

- 引擎会校验保护片段的还原，异常即视为失败尝试。
- 每个适配器都解析响应体，缺少译文字段时返回 `BAD_RESPONSE`。

若仍然回显原文，说明接口结构变了。抓取响应体并更新适配器。

## 批次输出比输入少

非法 JSON 行被跳过并在 stderr 警告。把 stderr 保留下来重跑，就能看到是哪些行号失败。

## 冷却清不掉

```bash
not-a-bot keys --reset
ls ~/.config/not-a-bot/cache/
```

如果目录不可写，冷却只存在于该进程内存里，每次调用都是干净的。

## Agent 不调用 CLI，而是自己翻译

1. 确认 skill 或指令文件放在该 harness 文档指定的路径。
2. Agent 所在 shell 里 `not-a-bot --version` 必须成功；从图形界面启动的 Agent 常常拿不到同样的 `PATH`。
3. 让 Agent 翻译一句，回复里应出现 `provider`。没有就是根本没执行命令。

## 导航

- ← [故障排查概览](OVERVIEW.md)
- → [错误契约](../api/02-errors.md)
