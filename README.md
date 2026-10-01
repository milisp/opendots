# opendots

Opensource OpenAI dots alternative

## Development

Tauri hosts the React web application. Bot functionality is served by a local Axum HTTP API on an OS-assigned loopback port. API operations do not use Tauri commands. Tauri native plugins are only used for shell conveniences such as folder selection and desktop notifications.

```sh
bun install
bun tauri dev
```

## Bots

Bot profiles and conversations are stored in the OS application data directory under `opendots/bots.db`; routines are stored under `opendots/automations.json`; each bot's memory is isolated under `opendots/bots/<id>/memory`. Bots use `keke agent stdio`. Trust levels are Read-only, Ask, and Autonomous. MCP selections are resolved from `~/.codex/config.toml`.
