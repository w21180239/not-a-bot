**简体中文** · [English](../../en/providers/OVERVIEW.md)

# Provider — 概览

[← 文档索引](../README.md)

Provider 是一个翻译后端。每个 Provider 都会报告：稳定 id、类型、优先级、语言集合、可选的字符上限、能否自动检测源语言。

## 文件索引

| 文件 | 范围 |
| --- | --- |
| [01-supported-providers.md](01-supported-providers.md) | 完整目录：id、类型、优先级、认证、限制、实测状态 |
| [02-priorities-and-fallback.md](02-priorities-and-fallback.md) | 链的组装、匹配规则、按错误决定是否换下一跳 |
| [03-adding-a-provider.md](03-adding-a-provider.md) | 实现并注册新后端 |

## 类型

| 类型 | 含义 | 需要认证 |
| --- | --- | --- |
| `web` | 翻译网站的公开接口，逆向得到 | 否 |
| `official` | 厂商文档化的 API | 是 |
| `llm` | 用对话模型做翻译 | 是 |

## 一句话版本

- 免费网页接口占优先级 1-12，始终注册。
- 官方 API 占 20-27，只在凭据完整时注册。
- LLM 占 40-42，必须显式配置 `api_key`。
- LibreTranslate 只在设置了 `[provider.libretranslate] base_url` 时注册，因为已无公共实例。
- `iflytek` 上游已停用，别名指向 `iflyrec`。

## 运行时查看

```bash
translatex providers --format json
translatex languages --provider edge --format json
translatex health --format json
```

`providers` 才是本机的权威列表：注册取决于凭据与配置，可能与文档目录不同。

## 导航

- ← [文档索引](../README.md)
- → [支持的 Provider](01-supported-providers.md)
