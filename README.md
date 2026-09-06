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

A local-first desktop companion for synchronizing supported local agent conversations and Agent Memory OS data through Google Drive. Part of the Bastet family.

> **0.4.2 — change language without pausing sync.** Language changes save automatically and preserve other settings. Agent snapshot groups now collapse and show local save times; source cards explain results and next steps. Selected local sources now capture compressed native history and exchange it through encrypted Drive objects. Missing sessions are added automatically; existing different versions remain conflicts. AMOS retains its official backup/merge. Cloud chats, cloud Work, full settings/skills, external attachments and physical two-device acceptance remain open. [Current support](docs/NATIVE_SESSIONS.md) · [Evidence](docs/VALIDATION.md).

The desktop includes a five-language resumable setup wizard, manual configuration, recovery-kit export/import and encrypted transport primitives. Google login requires a distributor-configured or explicitly imported Desktop OAuth client. [Setup guide](docs/SETUP_WIZARD.md). [Cloud contract and remaining gates](docs/CLOUD_SECURITY.md).

## ✨ Current workflow

1. Discover Claude, Claude Code, Codex, Google Agy CLI, Grok Build CLI, Pi Agent, Agent Memory OS and local ChatGPT Work.
2. Select individual agents or all detected sources; custom paths are supported.
3. Choose a Google Drive folder with guided setup.
4. Pick an interval or near-real-time sync, then press Start.
5. Receive supported local snapshots on another configured computer; existing different versions are preserved for separate recovery. Native continuation limits are documented per agent.

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
