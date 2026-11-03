**简体中文** · [English](../../en/architecture/01-request-flow.md)

# 架构 — 请求流程

[← 领域概览](OVERVIEW.md) · [文档索引](../README.md)

## 步骤

1. **规范化** —— `source_lang` 与 `target_lang` 经过 `lang::normalize`。未知目标返回 `INVALID_INPUT`；源目标相同同样拒绝。
2. **打码** —— 所有 `preserve` 正则只编译一次，匹配片段在任何 Provider 看到之前替换为不透明占位符。
3. **组装链** —— `Registry::chain` 依次应用 fallback 列表、配置链，再按优先级追加其余注册项。无法处理该语言对、或无法检测源语言的 Provider 被剔除。
4. **尝试** —— Provider 在 `race_width`（默认 3）大小的滑动窗口里并发执行。每个 Provider 选择凭据、套用预算、调用 `translate_once`、记录一条 `AttemptRecord`；第一个成功者胜出，其余在飞的尝试被取消。
5. **还原** —— 成功后还原占位符。还原失败视为一次失败尝试。
6. **返回** —— `Outcome { response, attempts }`，或链所产生的最具体错误。

## 并发

`race_width`（默认 3）表示同时最多有多少个 Provider 在飞。为 `1` 时严格串行；为 `3` 时前三个同时开始，第一个成功者胜出，其余直接被丢弃，不产生尝试记录。

指定的 Provider 永远单独运行 —— `--provider deepl --fallback google` 不能在 DeepL 即将返回时先返回 Google。

窗口会在 Provider 失败时继续补充，因此当前三个 Provider 都坏掉时，第四个会立刻被启动，而不是依次等完三次超时。

代价是每次请求最多 `race_width` 个并发上游调用，且落败的调用可能已被上游计费。想要省流量就设 `race_width = 1`，响应结构不变。

## 超时

预算取值：请求的 `timeoutMs` → 配置的 `timeout_ms` → 20 秒。它用 `tokio::time::timeout` 包住整个 Provider 调用，因此即使 Provider 忽略自身 socket 超时，也无法超出预算。

超时不原地重试：Provider 已经用完整个预算，重试只会把等待翻倍。链直接换下一跳。

## 重试

同一 Provider 内，可重试错误（限流、配额、网络、5xx）会带抖动指数退避重试，最多 `retries` 次。错误自带 `Retry-After` 时以它为准，并设上限，避免恶意响应把调用方拖住。

## 错误选择

| 情况 | 返回 |
| --- | --- |
| 一个都没尝试，但存在 Provider | `PROVIDER_NOT_CONFIGURED` / `PROVIDER_NOT_FOUND` / `AUTO_DETECT_UNSUPPORTED` |
| 有 Key 的 Provider 全部冷却 | 带最长剩余时间的 `RATE_LIMITED` |
| 指定的 Provider 失败 | 它自身的错误，而不是 `CHAIN_EXHAUSTED` |
| 多个 Provider 失败 | 最后一个错误；若全是上游失败则 `CHAIN_EXHAUSTED` |

## 保护片段引擎

占位符由匹配内容整体推导，而不是固定 token，因此即使译文调整了语序，也不会把片段还原到错误的句子里。还原会校验：占位符缺失或重复时 `restore` 返回错误，该次尝试记为失败。

## 导航

- ← [架构概览](OVERVIEW.md)
- → [凭据与轮换](02-credentials-and-rotation.md)
