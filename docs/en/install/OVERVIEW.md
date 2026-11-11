**English** · [简体中文](../../zh-cn/install/OVERVIEW.md)

# Install — Overview

[← Documentation index](../README.md)

Install the binary, run one request, and turn on shell completion.

## Requirements

- Rust 1.89 or newer when building from source.
- No runtime dependencies: the release binary is self-contained.
- Network access to the endpoint you want to use. Several web endpoints are region-sensitive.

## Three ways to install

| Method | Command | Notes |
| --- | --- | --- |
| Cargo | `cargo install not-a-bot` | Compiles with LTO; a few minutes |
| Prebuilt | download from [Releases](https://github.com/w21180239/not-a-bot/releases/latest) | Windows, Linux, macOS |
| Source | `cargo build --release` | Binary at `target/release/not-a-bot` |

## Verify

```bash
not-a-bot --version
not-a-bot providers --format text
```

`providers` lists the registered providers after credential gating. Free endpoints always appear; keyed providers appear only when credentials resolve.

## First request

```bash
not-a-bot translate "Hello world" -t zh
```

If no provider answers, the error tells you why:

```bash
not-a-bot translate "Hello world" -t zh --verbose
not-a-bot health --format json
```

## Shell completion

```bash
not-a-bot completions bash       > /etc/bash_completion.d/not-a-bot
not-a-bot completions zsh        > ~/.zfunc/_not-a-bot
not-a-bot completions fish       > ~/.config/fish/completions/not-a-bot.fish
```

PowerShell needs the generated script dot-sourced from the profile:

```powershell
not-a-bot completions powershell > "$HOME\Documents\PowerShell\not-a-bot-completion.ps1"
Add-Content $PROFILE '. "$HOME\Documents\PowerShell\not-a-bot-completion.ps1"'
```

## Navigation

- ← [Documentation index](../README.md)
- → [CLI reference](../cli/OVERVIEW.md)
- → [Clients and agents](../../../AGENTS.md)
