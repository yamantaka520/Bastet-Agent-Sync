# 🐈 Bastet Agent Sync — Guide

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

## 0.5.0

0.5.0 syncs local conversations and Agent Memory OS. Select sources, save and Start; manual JSONL export is unnecessary. Claude/Claude Code share local coding history; Codex/Work share local rollout storage. Agy uses SQLite snapshots; Grok and Pi use native session files.

## Setup

Choose language and device name. Select agents and review their data paths. Complete the five-step Drive wizard: OAuth desktop JSON, Google authorization, folder, recovery key, review. Completed steps persist automatically; reopen to continue, restart to archive old progress, or use manual configuration with the same checks. Join the same space with its recovery kit on the other computer.

## Run and restore

Choose upload, download or bidirectional; manual, interval or near-real-time. Save and Start. Per-source results distinguish no data, processing, partial success and errors. Missing receiving sessions are added automatically. Existing different files are preserved as conflicts. Pause, view snapshots and restore into a new folder to keep both versions. Agent Memory OS uses its official backup/merge automatically.

## Boundaries

Installed CLI checks: Grok exported both test messages after Bastet's encrypted isolated-profile restore. Agy repeated a marker in its dedicated original test conversation; its restored database passed integrity checks. Restored-profile Agy model continuation and physical two-device acceptance are not claimed. Grok recovery provides copyable POSIX/PowerShell continuation commands. Cloud chats/Work, project mapping, external attachments and complete settings/skill migration remain outside guarantees.

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


Snapshots now collapse by Agent, with Expand all / Collapse all and local save timestamps sorted newest first. Source cards show clear status, uploaded/downloaded bundle counts and locally added items. Common errors include next steps; technical codes remain expandable. The save timestamp is not the original conversation time.


### 0.4.2

In 0.4.2, language can be changed while syncing and is saved automatically, including tray labels. Other unsaved settings remain unchanged. If saving fails, the previous language is retained. On 0.4.1, pause synchronization before changing language, then save setup.

0.5.0: app, Google and sync states share a row below Drive setup status, wrapping on narrow windows. Upload/download rates and totals since launch refresh each second. Rates average approximately three seconds of Drive HTTP payloads. Excludes system-wide traffic, transport overhead, OAuth and updates; upload counts do not imply server acknowledgement.


## 🐈 0.5.0

0.5.0 adds the sync control center: source stages, examined item counts, current payload bytes and a sampled payload ETA; up to 500 persistent cycle records; encrypted device reports with report/observation times. Device reports are not live online status.

Pause, set 1–6 concurrent storage groups (default 3), upload/download KiB/s limits (0 = unlimited), and local allowed hours, then save. Shared profiles remain sequential. Allowed hours govern cycle start; equal times mean all day and overnight windows are supported. A 15-minute pause resumes while the app runs. Limits measure this app's Drive payload consumption, not system traffic.

Pause and Compare a snapshot to inspect local/incoming text or hashes. Keep local and mark reviewed, or keep both in a new folder. A changed local file invalidates review. Measure local app/cache or Drive object usage on demand; Clear download cache preserves replicas, journals and all cloud history.

Portable settings/skills default off. Opt in, Preview draft choices, expand content, uncheck individual files and save. Only allowlisted scalar preferences and supported text skills are included; standard Codex shared user skills are kept separately. Credentials, hooks, MCP/provider definitions and machine paths are excluded from config. Known secret patterns are filtered, but arbitrary content still needs human review. Received packages require comparison and new-folder recovery; nothing is automatically installed or executed.

Installed CLI checks: Grok exported both test messages after Bastet's encrypted isolated-profile restore. Agy repeated a marker in its dedicated original test conversation; its restored database passed integrity checks. Restored-profile Agy model continuation and physical two-device acceptance are not claimed. Grok recovery provides copyable POSIX/PowerShell continuation commands. Cloud chats/Work, project mapping, external attachments and complete settings/skill migration remain outside guarantees.

[Technical contract](../../SYNC_CONTROL.md) · [Validation](../../VALIDATION.md)
