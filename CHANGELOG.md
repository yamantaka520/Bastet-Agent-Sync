# Changelog

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
