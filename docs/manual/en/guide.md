# 🐈 Bastet Agent Sync — Guide & plan

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

A desktop companion for keeping your agents and conversations with you.

> The snapshot core is implemented. Click **Run isolated check** in the desktop app to transfer synthetic text between two temporary replicas and a shared folder, preserve two branches, repeat without extra transfers and recover after reopening. The check does not access selected agents or the Drive folder.

## Planned setup

1. Choose a language and device name.
2. Discover and select Claude, Claude Code, Codex, Google Agy CLI, Grok Build CLI and Pi Agent; custom paths are supported.
3. Connect Google Drive and choose or create a shared folder. Linux will use the planned direct API connection.
4. Choose bidirectional, upload-only or download-only, and manual, interval or near-real-time scheduling. Press Start after setup.
5. Close to tray when enabled, or pause/resume from the tray.

## Current foundation

Discover candidate data directories without reading conversations. Choose a local folder and save preferences. Native agent sync remains unavailable until the cloud connection and adapters are verified. Browser preview cannot access native functionality.

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

## M2: isolated core check

The snapshot core is implemented. Click **Run isolated check** in the desktop app to transfer synthetic text between two temporary replicas and a shared folder, preserve two branches, repeat without extra transfers and recover after reopening. The check does not access selected agents or the Drive folder.

This is not native agent synchronization. Google Drive login, encrypted transport, native session restoration and scheduling remain pending. The v1 core accepts prepared text files only, preserves all versions and does not propagate deletions.

[M2 details](../../SNAPSHOT_PROTOCOL.md)

## 🐈 M3 cloud preview

OAuth/credential-store code, encrypted Drive API operations and a synthetic encryption/recovery check are implemented. The default build has no OAuth client, so login is disabled. Local HTTP fixture tests are not real Google or two-device tests. The encrypted queue now exchanges two isolated replicas with preserved branches and durable retry IDs. The space/key wizard is now implemented (see below); Picker and GUI sync orchestration remain pending.

[Technical contract / 技術文件](../../CLOUD_SECURITY.md)

## 🐈 Guided setup, resume and manual configuration

The Drive wizard has five steps: login configuration, Google authorization, folder, encryption/recovery, and final review. Completed steps and mode are saved automatically. Reopen to resume, or Start again to archive the old progress without deleting Drive data or keys. Manual setup expands every section with the same validation. Import your own Desktop OAuth JSON if the build has no client; a folder ID/link can be entered directly. Save a recovery kit before verifying a new space, or import the other computer’s kit to join. Completion verifies setup only; Agent sync remains gated.

[Setup contract / 設定文件](../../SETUP_WIZARD.md)

## 🐈 Account checks and integrated ChatGPT

The wizard now displays the connected or saved Google account and refuses to resume under a different account. Remove the local login and reconnect with the original account, or restart setup to switch accounts. Old progress remains readable; explicit reconnect binds its identity.

Integrated ChatGPT desktop local Codex/Worktree tasks are included in the adapter target; local ChatGPT Work requires format/resume verification. General ChatGPT chats and cloud tasks need separate supported integration. Native cross-computer restoration is not yet verified.

[OpenAI documentation](https://learn.chatgpt.com/docs/environments/modes) · [ChatGPT / Codex](https://learn.chatgpt.com/docs/use-chatgpt)

## 🐈 Cancel Google authorization

During Connect, you can cancel the browser authorization wait without losing saved steps, then reconnect. Closing the browser alone waits for the three-minute timeout. Token exchange and account checks must finish; the app explains when cancellation is unavailable.

## 🐈 Agent Memory OS

Now available as a seventh source. Select an existing memory home or use AGENT_MEMORY_HOME. Export with the official AMOS CLI and use the JSONL inspection panel. Versions 1–3 are supported, up to 1 MiB. This preview preserves bundle contents but does not automatically export, publish to Drive or import into the live store. Local exports may contain private memories; tombstones and organization records require import review.

[Adapter contract](../../AGENT_MEMORY_OS.md)

If authorization fails before the browser opens, the app now distinguishes imported-client access, saved-login access and browser-launch errors. Quit and reopen an updated development app; re-import the JSON if client access fails. Import must pass credential-store readback before step 1 completes.
