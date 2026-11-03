**简体中文** · [English](../../en/providers/02-priorities-and-fallback.md)

# Provider — 优先级与回退

## 链的组装

`Registry::chain` 按以下顺序生成尝试列表：

1. **Fallback 列表** —— `--fallback` / `fallbackChain`，按书写顺序。
2. **配置链** —— `config.toml` 的 `chain` 或 `TRANSLATEX_CHAIN`。
3. **其余已注册 Provider** —— 按 `priority` 升序。

去重按解析后的 Provider id 进行，因此别名与目标不会同时出现。用 `--provider` 指定时：

- 能处理该请求：它排第一，fallback 列表紧随其后。
- 不能处理（不支持自动检测、语言对不支持）且存在 fallback：由 fallback 接管。
- 不能处理且没有 fallback：返回 `AUTO_DETECT_UNSUPPORTED` 或 `LANGUAGE_PAIR_NOT_SUPPORTED`，并指名该 Provider。

## 过滤

未指定 Provider 时，任一检查失败即从链中移除：

| 检查 | 何时生效 | 结果 |
| --- | --- | --- |
| 支持目标语言 | 始终，除非 `target` 为 `auto` | 该 Provider 无法译入该语言 |
| 支持源语言检测 | `sourceLang` 为 `auto` | 该 Provider 需要显式源语言 |

`supports_pair` 用于支持不对称的接口。`iflyrec` 覆写它，要求一侧必须是中文。

## 优先级区间

| 区间 | 类型 | 理由 |
| --- | --- | --- |
| 1-3 | 快且质量高的网页接口 | Google、Edge、爱词霸覆盖大部分流量 |
| 4-8 | 专用网页接口 | iFlyrec（中文）、ModernMT、火山、Lara、Lingvanex、Mirai、Papago |
| 9-12 | 兜底网页接口 | Reverso、搜狗、MyMemory、SysTran |
| 20-27 | 官方 API | 稳定、有 SLA，但需要 Key |
| 40-42 | LLM | 成本最高、延迟最大、最不可控 |

数字越小越靠前。区间是约定而非巧合：默认链中网页接口不应排在官方 API 之后，除非它在特定语言对上确实更可靠。

## 尝试行为

每个 Provider 最多调用 `retries + 1` 次：

| 错误类别 | 同 Provider 重试 | Key 冷却 | 换下一个 |
| --- | --- | --- | --- |
| `RATE_LIMITED` | 是，退避后 | 是 | 是 |
| `QUOTA_EXHAUSTED` | 是 | 是 | 是 |
| `AUTH_FAILED` | 否 | 是 | 是 |
| `UPSTREAM_ERROR` 5xx | 是 | 否 | 是 |
| `UPSTREAM_ERROR` 4xx | 否 | 否 | 是 |
| `NETWORK_ERROR` | 是 | 否 | 是 |
| `TIMEOUT` | 否 | 否 | 是 |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | 否 | 否 | 是 |
| `INVALID_INPUT` | 否 | 否 | 否 —— 立即返回 |

超时故意不原地重试：该 Provider 已经用完了整个预算，再试只会把等待翻倍。

## 冷却

冷却的目的是在凭据之间轮换，而不是让 Provider 下线：

- 有 Key 的 Provider 把冷却状态写入 `~/.config/translatex/cache/keys-<provider>.json`，所有进程共享。
- 无 Key 的 Provider 使用不受管理的环。它的唯一伪凭据永不冷却，因为没有第二个 Key 可换，冷却只会悄悄让这个通道消失。
- 如果所有 Provider 的所有 Key 都在冷却，错误是带最长剩余时间的 `RATE_LIMITED`，而不是 `NO_PROVIDER`。

## 导航

- ← [支持的 Provider](01-supported-providers.md)
- → [新增 Provider](03-adding-a-provider.md)
