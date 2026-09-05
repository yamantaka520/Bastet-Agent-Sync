# Changelog

## 0.3.0 — Local conversation sync preview

- Connect selected local conversation sources to compressed native snapshots and encrypted Drive exchange, independently of Agent Memory OS.
- Automatically add missing receiving sessions; preserve existing different versions and offer separate-folder recovery in a five-language snapshot library.
- Show per-source empty, syncing, complete, partial and failure results. Deduplicate shared Claude/Claude Code and Codex/Work profiles.
- Use SQLite online backup for Agy, retain Claude subagent files, batch large captures and cache Drive downloads by provider revision.
- Keep cloud chats, full settings/skills, external attachments and unverified native continuation explicitly outside this milestone; see the native adapter contract.

## 0.2.1

- Automatic Agent Memory OS export, encrypted Drive exchange and backed-up official merge; manual/interval/15-second scheduling, pause and per-cycle counters.
- ChatGPT Work listed separately; unsupported sources no longer veto ready memory synchronization.
- Manual JSONL inspection moved under Advanced. Native conversation adapters remain unavailable.

## 0.2.0 — Runtime status and signed updater

- Replace permanently disabled Start with explicit native preflight and visible reasons; connect Drive status to wizard state. No automatic Agent worker is claimed.
- Add emoji status, native version/build below the logo, and a version-setting/checking workflow.
- Add signed online update checks, download/install progress and restart, with a protected signing key and manual four-platform draft release workflow. Published-feed and self-update acceptance remain pending.


## Imported Google authorization recovery — 2026-09-05

- Verify imported OAuth configuration by reading it back before marking setup complete.
- First authorization opens browser consent directly; reconnect retains the saved-login refresh route. Distinguish client, login-store and browser-launch failures in five languages.
- macOS real-account smoke reached the Google callback and all five wizard steps completed; cross-platform and two-device acceptance remain open.


## Agent Memory OS adapter preview — 2026-09-05

- Add Agent Memory OS as a seventh selectable source with environment/custom-path discovery.
- Add bounded official JSONL v1–3 inspection and lossless snapshot capture/restore; test replica transfer plus encryption without touching a live memory store.
- Add five-language inspection UI and document private-export, deletion/ACL and trusted-import boundaries. Automatic export, Drive orchestration and live import remain gated.


## Browser authorization cancellation — 2026-09-05

- Cancel browser OAuth waiting without resetting setup; retry uses fresh state/PKCE. Five-language UI explains phases that cannot be cancelled.
- Bound callback header reads and explicitly configure accepted sockets, also fixing a macOS CI fixture race caused by inherited nonblocking mode.


## Account-aware setup — 2026-09-05

- Show saved/connected Google identity in five languages and check the stable Drive permission ID on authorization/refresh before continuing setup.
- Preserve progress on account mismatch; retain compatibility with old wizard files and require explicit connection to adopt their account.
- Record integrated ChatGPT desktop local-task adapter scope, with cloud chat integration and native restoration still unverified.


## 0.1.0-dev — Resumable setup wizard, 2026-09-05

- Added five-stage guided setup and a manual mode sharing the same saved, validated progress.
- Added resume navigation and explicit restart with archived records; no Drive files, credentials or keys are deleted.
- Added native Desktop OAuth configuration import, manual folder ID/link verification and durable space/key preparation.
- Added recovery-kit export/readback, verified import and final folder/key proof check; secrets never enter renderer state.
- Added per-step restart, interrupted proof upload, wrong-key, manual-mode and cancellation tests. Real-account acceptance remains pending.

## 0.1.0-dev — M3 preview, 2026-09-05

- Added desktop OAuth PKCE/state flow, fixed HTTPS endpoints and refresh-token credential storage with no file fallback.
- Added encrypted snapshot envelopes, random recovery keys, authenticated space binding and bounded Drive upload/download/listing operations.
- Added durable preallocated-ID recovery for uncertain folder creation, conservative HTTP errors and bounded backoff policy.
- Connected validated replicas to a durable encrypted exchange queue with key proof, ambiguous-upload reconciliation, direction handling and conflict-preserving two-replica tests.
- Added five-language cloud configuration status and synthetic encryption/recovery check; unconfigured builds cannot start Google login.
- Added crypto, callback, recovery, HTTP fixture and UI tests. Google OAuth client is not configured; real-account transfer, native credential-store verification, Picker, space/key wizard and GUI queue orchestration remain pending.

## 0.1.0-dev — M2, 2026-09-05

- Added immutable SHA-256 text snapshots, a space identity and a locked local replica with recoverable checkpoints.
- Added one-shot upload/download/bidirectional local-folder transport, pending ancestry, preserved branches and explicit conflict resolution.
- Require an explicit original base for the public export API so newly received remote updates cannot silently rebase offline edits.
- Added an isolated native GUI diagnostic in five languages, without accessing selected agent profiles or Drive folders.
- Added interrupted-file, corruption, lineage, direction, recovery and hostile-path tests.
- Fixed Windows documentation validation by reading Markdown as UTF-8.
- Real agent transfer, encryption, automatic scheduling and native session restoration remain pending.

## 0.1.0-dev — M1, 2026-09-05

- Established requirements and a five-milestone implementation roadmap.
- Added the Tauri/React desktop foundation, five-language UI and guides, native folder picker, candidate agent discovery, validated settings persistence and localized custom tray menu.
- Added macOS/Windows/Linux CI and 11 automated tests; built and smoke-tested the macOS app.
- Synchronization and cross-computer session restoration are not released.
