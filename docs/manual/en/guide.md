# 🐈 Bastet Agent Sync — Guide

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

## 0.4.0

0.4.0 syncs local conversations and Agent Memory OS. Select sources, save and Start; manual JSONL export is unnecessary. Claude/Claude Code share local coding history; Codex/Work share local rollout storage. Agy uses SQLite snapshots; Grok and Pi use native session files.

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

## 🐈 Download and install

[Versions and downloads](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/latest): click the version for direct installer links in its release notes. Windows detects and installs missing embedded WebView2; macOS uses system WebKit; Linux users can run the downloaded `sh install-linux.sh` for apt/dnf detection, checksums and dependency installation. Node.js and Rust are unnecessary. Configure AMOS CLI and agent accounts separately. macOS is not notarized and Windows has no Authenticode certificate; OS trust prompts may appear.

If macOS blocks the first launch, confirm the download source, then use System Settings → Privacy & Security → Open Anyway for this app. [Apple](https://support.apple.com/en-au/guide/mac-help/mh40616/mac)


### 🔐 0.4.1

Use **🔐 Prepare credential access** before syncing to read saved credentials together. On macOS choose Always Allow for each requested Bastet item. Successful accesses are cached in memory until exit, forgetting login, restarting setup or changing client configuration. Closing to tray retains the cache. Press again after editing credentials externally. This action does not verify Google access or start sync. Each computer needs authorization; updates may ask again because Developer ID signing is not yet configured.
