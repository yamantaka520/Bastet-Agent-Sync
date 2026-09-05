<div align="center">
<img src="assets/calico.png" width="160" alt="Bastet calico cat mascot" />

# 🐈 Bastet Agent Sync

**Your agents. Your conversations. Across your computers.**

![Stage](https://img.shields.io/badge/stage-M3%20preview-orange)
![Platforms](https://img.shields.io/badge/targets-macOS%20%7C%20Windows%20%7C%20Linux-blue)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)

[繁體中文](docs/manual/zh-Hant/guide.md) · [简体中文](docs/manual/zh-Hans/guide.md) · [English](docs/manual/en/guide.md) · [日本語](docs/manual/ja/guide.md) · [한국어](docs/manual/ko/guide.md)
</div>

A local-first desktop companion for synchronizing agent settings, skills, memory and conversations through Google Drive. Part of the Bastet family.

> **0.3.2 — local conversation sync preview.** Selected local sources now capture compressed native history and exchange it through encrypted Drive objects. Missing sessions are added automatically; existing different versions remain conflicts. AMOS retains its official backup/merge. Cloud chats, cloud Work, full settings/skills, external attachments and physical two-device acceptance remain open. [Current support](docs/NATIVE_SESSIONS.md) · [Evidence](docs/VALIDATION.md).

M3 preview adds a five-language resumable setup wizard, manual configuration, recovery-kit export/import and encrypted transport primitives. Google login requires a distributor-configured or explicitly imported Desktop OAuth client. [Setup guide](docs/SETUP_WIZARD.md). [Cloud contract and remaining gates](docs/CLOUD_SECURITY.md).

## ✨ Planned experience

1. Discover Claude, Claude Code, Codex, Google Agy CLI, Grok Build CLI and Pi Agent.
2. Select individual agents or all available profiles.
3. Choose a Google Drive folder with guided setup.
4. Pick an interval or near-real-time sync, then press Start.
5. Continue on another computer, with conflicts and conversation branches preserved.

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
