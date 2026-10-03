<div align="center">
<img src="assets/calico.png" width="160" alt="Bastet calico cat mascot" />

# 🐈 Bastet Agent Sync

**Your agents. Your conversations. Across your computers.**

[![Release](https://img.shields.io/github/v/release/yamantaka520/Bastet-Agent-Sync)](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/latest)

![Stage](https://img.shields.io/badge/stage-local%20sync%20preview-orange)
![Platforms](https://img.shields.io/badge/targets-macOS%20%7C%20Windows%20%7C%20Linux-blue)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)

[繁體中文](docs/manual/zh-Hant/guide.md) · [简体中文](docs/manual/zh-Hans/guide.md) · [English](docs/manual/en/guide.md) · [日本語](docs/manual/ja/guide.md) · [한국어](docs/manual/ko/guide.md)
</div>

A local-first desktop companion for synchronizing supported local agent conversations and Agent Memory OS data through a selected encrypted cloud destination. Part of the Bastet family.

> **0.7.0 — selected cloud destination and recovery management.** Choose Google Drive API, an iCloud Drive folder, or a OneDrive folder. Folder modes use installed desktop sync clients and keep encrypted handoff distinct from cloud delivery. [Latest published release](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/latest) · [Folder setup](docs/LOCAL_CLOUD_FOLDERS.md) · [Handoff and limits](docs/CROSS_OS_HANDOFF.md) · [Evidence](docs/VALIDATION.md).

macOS installers are Developer ID signed and notarized. Windows publisher signing remains deferred; updater signatures are verified on every platform. Agy remains explicit database recovery without workspace remapping.

The desktop includes five-language setup, recovery-kit export/import and encrypted transport. The credentials entry explains local credential storage and offers another recovery-kit export. Google login requires a distributor-configured or explicitly imported Desktop OAuth client. iCloud and OneDrive folder modes require their respective desktop clients; direct Apple/Microsoft sign-in is not included. Repeated macOS Keychain prompts after “Always Allow” remain under investigation. [Setup guide](docs/SETUP_WIZARD.md). [Cloud contract and remaining gates](docs/CLOUD_SECURITY.md).

## ✨ Current workflow

1. Discover Claude, Claude Code, Codex, Google Agy CLI, Grok Build CLI, Pi Agent, Agent Memory OS and local ChatGPT Work.
2. Select individual agents or all detected sources; custom paths are supported.
3. Choose one destination: a Google Drive API folder, a downloaded iCloud Drive folder, or a downloaded OneDrive folder. Create or join an encrypted space and save its recovery kit outside the synced folder.
4. Pick an interval or near-real-time sync, then press Start.
5. Receive supported local snapshots on another configured computer; existing different versions are preserved for separate recovery. A successful folder cycle proves local handoff to the provider client, not cloud or second-device delivery. Physical iCloud/OneDrive two-device delivery is unverified. Native continuation limits are documented per agent.

Five interface languages, a menu-bar/system-tray companion, explicit pause and recovery controls. Linux uses the Drive API transport without requiring Google's desktop client.

## 🧭 Build and contribute

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

Install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) first. `npm run dev` is a browser preview with native operations unavailable. Native Start uploads selected local data after Drive setup.

## 📖 Project documents

- [Master plan](docs/MASTER_PLAN.md) and [detailed requirements (繁體中文)](REQUIREMENTS.md)
- [Architecture decision](docs/adr/0001-desktop-foundation.md)
- [Snapshot protocol](docs/SNAPSHOT_PROTOCOL.md)
- [Validation](docs/VALIDATION.md) and [changelog](CHANGELOG.md)
- [Brand asset provenance](assets/README.md)

Contributions should include tests, keep the five locale dictionaries aligned, and avoid publishing credentials or machine-specific operational information. Never test imports by overwriting a live agent profile.

🐈 [Agent Memory OS](docs/AGENT_MEMORY_OS.md): automatic official export, encrypted exchange, backup and merge.
