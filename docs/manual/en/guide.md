# 🐈 Bastet Agent Sync — Guide & plan

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

A desktop companion for keeping your agents and conversations with you.

> Development has started. M1 provides the desktop foundation; actual synchronization and cross-computer resume are not available yet.

## Planned setup

1. Choose a language and device name.
2. Discover and select Claude, Claude Code, Codex, Google Agy CLI, Grok Build CLI and Pi Agent; custom paths are supported.
3. Connect Google Drive and choose or create a shared folder. Linux will use the planned direct API connection.
4. Choose bidirectional, upload-only or download-only, and manual, interval or near-real-time scheduling. Press Start after setup.
5. Close to tray when enabled, or pause/resume from the tray.

## Current foundation

Discover candidate data directories without reading conversations. Choose a local folder and save preferences. The sync action remains unavailable until the engine is implemented. Browser preview cannot access native functionality.

## Conversation continuity

Native restoration requires a verified adapter. Context continuation starts a new conversation and is labelled separately. Preserve attachments, tool results and project paths where export is supported. Concurrent histories stay as branches; credentials remain local.

## Roadmap

M0: repository, plan, five languages and cat identity. M1: desktop, discovery, settings and tray. M2: snapshots, conflicts and local transport. M3: Google Drive and encryption. M4: native agent restoration. M5: scheduling and three-platform distribution.

## Development

Install Node.js, Rust and Tauri platform prerequisites. Run the following commands from the repository root.

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

## Validation & limits

Windows/Linux builds and actual tray behavior require verification. No cloud login, upload or session import is performed in M1. Closing an agent process is different from moving its conversation history.

## References

[Master plan](../../MASTER_PLAN.md) · [Validation](../../VALIDATION.md) · [Requirements](../../../REQUIREMENTS.md) · [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
