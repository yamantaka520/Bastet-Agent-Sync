# Changelog

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
