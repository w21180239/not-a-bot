**简体中文** · [English](../../en/architecture/OVERVIEW.md)

# 架构 — 概览

[← 文档索引](../README.md)

```
src/
├── main.rs              入口，交给 cli::run
├── cli.rs               参数解析、分发、注册表构建
├── retry.rs             带抖动的指数退避
├── core/
│   ├── config.rs        Settings、默认链、路径
│   ├── creds.rs         凭据加载、钥匙串展开、环境变量覆盖
│   ├── crypto.rs        Iciba 适配器使用的 AES-ECB/CBC 工具
│   ├── engine.rs        回退循环、密钥选择、保护片段、超时
│   ├── http.rs          共享客户端、签名工具、响应处理
│   ├── keys.rs          单凭据冷却状态，跨进程
│   ├── lang.rs          规范语言码、别名、Provider 映射表
│   ├── preserve.rs      正则打码与还原
│   ├── types.rs         CLI 与 Provider 共用的线格式类型
│   └── error.rs         错误分类、线格式映射、HTTP 状态映射
└── providers/           每个后端一个模块，另有注册表与 trait
```

## 文件索引

| 文件 | 范围 |
| --- | --- |
| [01-request-flow.md](01-request-flow.md) | 校验、打码、链组装、尝试、还原 |
| [02-credentials-and-rotation.md](02-credentials-and-rotation.md) | 凭据解析、轮换轮次、冷却分层 |

## 设计约束

1. **stdout 是数据通道。** 只写调用方要的载荷，所有诊断走 stderr。
2. **Provider 无状态。** 它只接收凭据与请求；重试、冷却、回退由引擎负责。
3. **错误先有类型，再有格式。** `Error::*` 决定是否重试、是否冷却、退出码；`WireError` 只是序列化视图。
4. **链的顺序是数据。** 引擎不硬编码 Provider 名，只向注册表索取。

## 导航

- ← [文档索引](../README.md)
- → [请求流程](01-request-flow.md)
