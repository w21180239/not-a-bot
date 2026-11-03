# translatex (npm)

Installs the `translatex` binary and exposes it on `PATH`. The package itself
is a shim; the download happens once at install time from the matching GitHub
release.

```bash
npm install -g translatex
translatex translate "Hello world" -t zh
```

## MCP server

Any MCP client can launch the same binary:

```json
{
  "mcpServers": {
    "translatex": {
      "command": "translatex",
      "args": ["mcp"]
    }
  }
}
```

Tools: `translate`, `roundtrip`, `providers`, `languages`, `health`.

## Offline install

```bash
TRANSLATEX_BINARY_PATH=/path/to/translatex npm install -g translatex
```

Or skip npm entirely:

```bash
cargo install translatex --locked
```

Full documentation: https://github.com/w21180239/translatex
