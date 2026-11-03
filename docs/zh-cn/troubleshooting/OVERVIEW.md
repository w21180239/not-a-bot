**简体中文** · [English](../../en/troubleshooting/OVERVIEW.md)

# 故障排查 — 概览

先跑两条命令，它们能回答大部分问题：

```bash
translatex providers --format json | jq '.providers[] | {name, kind, priority, configured}'
translatex health --format json
```

## 文件索引

| 文件 | 范围 |
| --- | --- |
| [01-common-issues.md](01-common-issues.md) | 症状 → 原因 → 处理，按错误类别组织 |

## 快速分诊

| 症状 | 先查什么 |
| --- | --- |
| `NO_PROVIDER` | 有免费接口注册吗？该语言对的链是否为空？ |
| `PROVIDER_NOT_CONFIGURED` | 凭据字段是否齐全？`translatex keys <provider>` |
| `PROVIDER_NOT_FOUND` | 拼写错误，或从未配置的有 Key Provider |
| `AUTO_DETECT_UNSUPPORTED` | 改用 `-s en`，不要依赖 `auto` |
| `CHAIN_EXHAUSTED` | 加 `--verbose` 看逐跳轨迹 |
| 处处 `TIMEOUT` | 出口网络、代理，或 `timeoutMs` 太小 |
| 译文等于原文 | 接口结构变了，按缺陷处理 |

## 导航

- ← [文档索引](../README.md)
- → [常见问题](01-common-issues.md)
