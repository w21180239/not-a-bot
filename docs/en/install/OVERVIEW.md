**English** · [简体中文](../../zh-cn/install/OVERVIEW.md)

# Install — Overview

## Requirements

- Rust 1.80 or newer when building from source.
- No runtime dependencies: the release binary is self-contained.
- Network access to the endpoint you want to use. Several web endpoints are region-sensitive.

## Three ways to install

| Method | Command | Notes |
| --- | --- | --- |
| Cargo | `cargo install translatex` | Compiles with LTO; a few minutes |
| Prebuilt | download from [Releases](https://github.com/w21180239/translatex/releases/latest) | Windows, Linux, macOS |
| Source | `cargo build --release` | Binary at `target/release/translatex` |

## Verify

```bash
translatex --version
translatex providers --format text
```

`providers` lists the registered providers after credential gating. Free endpoints always appear; keyed providers appear only when credentials resolve.

## First request

```bash
translatex translate "Hello world" -t zh
```

If no provider answers, the error tells you why:

```bash
translatex translate "Hello world" -t zh --verbose
translatex health --format json
```

## Shell completion

```bash
translatex completions bash       > /etc/bash_completion.d/translatex
translatex completions zsh        > ~/.zfunc/_translatex
translatex completions fish       > ~/.config/fish/completions/translatex.fish
translatex completions powershell > $PROFILE.d/translatex.ps1
```

## Next

- [Clients and agents](../../../AGENTS.md)
- [CLI reference](../cli/OVERVIEW.md)
- [Provider catalogue](../providers/01-supported-providers.md)
