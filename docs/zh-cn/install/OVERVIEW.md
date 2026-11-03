**简体中文** · [English](../../en/install/OVERVIEW.md)

# 安装 — 概览

## 要求

- 从源码构建需要 Rust 1.80 或更高。
- 无运行时依赖：release 二进制自带全部依赖。
- 能访问目标接口的网络。部分网页接口对地区敏感。

## 三种安装方式

| 方式 | 命令 | 说明 |
| --- | --- | --- |
| Cargo | `cargo install translatex` | 开启 LTO 编译，需要几分钟 |
| 预编译 | 从 [Releases](https://github.com/w21180239/translatex/releases/latest) 下载 | Windows、Linux、macOS |
| 源码 | `cargo build --release` | 产物在 `target/release/translatex` |

## 验证

```bash
translatex --version
translatex providers --format text
```

`providers` 列出经过凭据筛选后实际注册的 Provider。免费接口总会出现；需要 Key 的 Provider 只有在凭据可用时才出现。

## 第一次请求

```bash
translatex translate "Hello world" -t zh
```

如果没有 Provider 可用，错误会说明原因：

```bash
translatex translate "Hello world" -t zh --verbose
translatex health --format json
```

## Shell 补全

```bash
translatex completions bash       > /etc/bash_completion.d/translatex
translatex completions zsh        > ~/.zfunc/_translatex
translatex completions fish       > ~/.config/fish/completions/translatex.fish
translatex completions powershell > $PROFILE.d/translatex.ps1
```

## 下一步

- [Agent 集成](../../../AGENTS.md)
- [CLI 参考](../cli/OVERVIEW.md)
- [Provider 目录](../providers/01-supported-providers.md)
