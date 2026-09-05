# 🐈 Bastet Agent Sync — Guide

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

## 0.3.2

0.3.2 syncs local conversations and Agent Memory OS. Select sources, save and Start; manual JSONL export is unnecessary. Claude/Claude Code share local coding history; Codex/Work share local rollout storage. Agy uses SQLite snapshots; Grok and Pi use native session files.

## Setup

Choose language and device name. Select agents and review their data paths. Complete the five-step Drive wizard: OAuth desktop JSON, Google authorization, folder, recovery key, review. Completed steps persist automatically; reopen to continue, restart to archive old progress, or use manual configuration with the same checks. Join the same space with its recovery kit on the other computer.

## Run and restore

Choose upload, download or bidirectional; manual, interval or near-real-time. Save and Start. Per-source results distinguish no data, processing, partial success and errors. Missing receiving sessions are added automatically. Existing different files are preserved as conflicts. Pause, view snapshots and restore into a new folder to keep both versions. Agent Memory OS uses its official backup/merge automatically.

## Boundaries

Cloud-hosted Claude chats, general ChatGPT chats and cloud Work are not exported. Project files, external attachments, full settings/skills, Cowork VMs and path remapping are not complete. Agy/Grok native resume and physical two-computer acceptance remain unverified. New-folder recovery does not install credentials.

## Tray and updates

Enable close-to-tray to keep the app available. Pause before changing settings. The logo shows the build version. Check for updates, review and install, then explicitly restart; a signed published release is required.

## Validation

[Native sessions](../../NATIVE_SESSIONS.md) · [Agent Memory OS](../../AGENT_MEMORY_OS.md) · [Drive wizard](../../SETUP_WIZARD.md) · [Validation](../../VALIDATION.md) · [Plan](../../MASTER_PLAN.md)

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```
