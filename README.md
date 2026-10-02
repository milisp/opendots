# opendots

Opensource OpenAI dots and Grok bot alternative

- [GitHub Releases](https://modern-github-release/#/repo/milisp/opendots)

![opendots](https://github.com/user-attachments/assets/11b8e1f9-6133-4221-8722-6429edb53c7f)

## Development

Tauri hosts the React web application. Bot functionality is served by a local Axum HTTP API on an OS-assigned loopback port. API operations do not use Tauri commands. Tauri native plugins are only used for shell conveniences such as folder selection and desktop notifications.

```sh
bun install
bun tauri dev
```

## Bots

Bot profiles and conversations are stored in the OS application data directory under `opendots/bots.db`; routines are stored under `opendots/automations.json`; each bot's memory is isolated under `opendots/bots/<id>/memory`. Bots use `keke agent stdio`. Trust levels are Read-only, Ask, and Autonomous. MCP selections are resolved from `~/.codex/config.toml`.
