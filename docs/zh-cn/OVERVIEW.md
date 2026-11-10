**简体中文** · [English](../en/OVERVIEW.md)

# 概览

[← 文档索引](../README.md)

`translatex` 按优先级顺序依次尝试 Provider 来回答翻译请求：免费网页接口排在前面，有凭据时才加入官方 API，LLM 放在最后。

## 这个二进制能做什么

- 通过 28 个 Provider 翻译：14 个免费网页接口、9 个官方 API、5 个 LLM（其中两个调用你本机的 Claude Code 或 OpenCode CLI）。
- 轮换 API Key，并把失败的 Key 跨进程冷却。
- 同时保持最多三个 Provider 在飞，慢接口无法拖住后面健康的通道。
- 经机器翻译或 LLM 的多跳链往返，人性化改写 AI 味文字，并报告丢掉的人名、数字和代码。
- stdout 输出 JSON、NDJSON 或纯文本；诊断信息走 stderr。
- 翻译前用正则把需要保留的片段打码，翻译后还原。
- 同一套引擎以结构化输出的 CLI 形式提供给 Agent。

## 请求流程

```
Agent ──▶ CLI
            │
            ├─ 校验输入、规范化语言码
            ├─ 对保护片段打码
            ├─ 组装链：fallback ─▶ 配置链 ─▶ 其余按优先级
            ├─ 逐个 Provider：取 Key ─▶ 带超时调用 ─▶ 映射错误
            └─ 返回结果，或带尝试轨迹的强类型错误
```

## 优先级方案

| 区间 | 类型 | 示例 |
| --- | --- | --- |
| 1-12 | 免费网页接口 | `google`（1）、`edge`（2）、`iciba`（3）、`mirai`（7）、`systran`（12） |
| 20-27 | 官方 API | `deepl` / `baidu`（20）、`azure`（22）、`tencent`（26）、`youdao`（27） |
| 40-42 | LLM | `openai`（40）、`anthropic`（41）、`deepseek`（42） |
| 60-61 | 本机 agent CLI，只在点名时运行 | `claude-cli`（60）、`opencode-cli`（61） |

数字越小越先尝试。链的组装规则见 [providers/02-priorities-and-fallback.md](providers/02-priorities-and-fallback.md)。

## 文档约定

| 术语 | 含义 |
| --- | --- |
| Provider | 一个翻译后端，用小写稳定 id 标识 |
| 链（Chain） | 本次请求会依次尝试的 Provider 列表 |
| Key 标签 | Provider 内部一组凭据的名字（`default`、`backup`…） |
| 冷却（Cooldown） | 认证/配额/限流失败后临时跳过某个凭据 |
| 保护片段 | 正则匹配到的、必须原样保留的文本 |
| 指定 Provider | 用 `--provider` 显式指定的 Provider |

## 导航

- → [安装](install/OVERVIEW.md)
- → [CLI 参考](cli/OVERVIEW.md)
- → [Provider](providers/OVERVIEW.md)
- → [API 与错误](api/OVERVIEW.md)
- ↑ [文档索引](README.md)
