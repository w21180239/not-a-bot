**简体中文** · [English](../../en/architecture/02-credentials-and-rotation.md)

# 架构 — 凭据与轮换

[← 领域概览](OVERVIEW.md) · [文档索引](../README.md)

## 解析

`CredentialStore::load` 读取三个来源并逐层覆盖：

1. `credentials.toml` —— 节 `[provider.label]`，每组是扁平字段表。
2. 环境变量 —— `NOT_A_BOT_<PROVIDER>_<FIELD>`，生成标签为 `env` 的凭据。
3. 钥匙串 —— 值为 `@keyring` 的字段会被替换为服务 `translators` 下账户 `<provider>.<field>` 的密钥。

当某 Provider 至少有一组凭据且所有必填字段齐全时即为 `configured`。`ProviderInfo.configured` 反映这一点，`build_keyed` 只注册满足条件的官方 Provider。

## 标签与轮换

每组凭据都有标签，轮换按标签轮询：

```
[k1 正常] [k2 冷却中] [k3 正常]
   ↑ 下一次请求从这里开始，k2 冷却期间跳过
```

引擎每次请求为每个 Provider 加载一次环，取第一个不在冷却的槽位；失败时先记录冷却再继续。`rounds` 等于标签数量，因此一次请求可以把所有 Key 都试一遍。

## 冷却分层

| 失败 | 冷却时长 |
| --- | --- |
| `QUOTA_EXHAUSTED` | `retryAfterSecs`，否则 3600 秒 |
| `RATE_LIMITED` | `retryAfterSecs`，否则 60 秒 |
| `AUTH_FAILED` | 900 秒 |

状态按 Provider 持久化到 `~/.config/not-a-bot/cache/keys-<provider>.json`：

```json
{
  "cooled_until": { "default": "2026-11-03T05:12:44Z" },
  "failures": { "default": 2 }
}
```

所有进程读写的都是同一个文件，两个并发调用共享同一视图。`not-a-bot keys --reset` 删除这些文件。

## 无 Key 的 Provider

网页接口只有一个标签为 `default` 的伪凭据，没有第二个 Key 可换，因此使用不受管理的环：

- 不写状态文件。
- `record_failure` 不产生效果。
- 通道永远不会因冷却而消失。

这曾经是一个真实缺陷：冷却无 Key 的 Provider 会让它在此后所有请求中消失，直到冷却结束，看起来就像通道静默失效。

## 全部 Key 冷却

链中所有 Provider 的所有 Key 都在冷却时，一次都不会尝试。引擎返回带最长剩余时间的 `RATE_LIMITED`，而不是 `NO_PROVIDER`，这样 Agent 会退避，而不是断定配置坏了。

## 导航

- ← [请求流程](01-request-flow.md)
- ↑ [架构概览](OVERVIEW.md)
