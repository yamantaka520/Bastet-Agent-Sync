# 🐈 Agent Memory OS automatic synchronization

Version 0.2.1 connects selection and Start to a real background worker. Users do not pick export files: Bastet invokes the installed `agent-memory` CLI automatically, encrypts snapshots for the configured Google Drive space, and invokes the official merge on receiving devices. The manual JSONL inspector is an optional advanced diagnostic.

## Setup and trust

Complete the Drive wizard, select Agent Memory OS, save, then Start. Other selected sources with no adapter are listed as skipped and do not veto this source. Discovery honors the selected memory home or `AGENT_MEMORY_HOME`. CLI lookup uses PATH, the standard memory virtual environment, common user installs and source checkouts; a native executable picker handles custom installs. Credentials, the active database and its WAL are never raw-copied into Drive.

This is full-store synchronization between the user's trusted devices holding the same space recovery key. It includes private memories, tombstones, teams/projects and permissions. The UI states this before Start. AMOS owns transactional merge and conflict/ACL semantics. Backups use its official backup command before each merge; failed backups cannot become completed backup markers. Bastet never replaces the active database. The user requested automatic operation; there is no per-cycle manual import approval.

## Execution and recovery

Only an explicit Start creates the worker. It reconnects saved Google authorization, verifies the bound account and space/key proof, then performs encrypted exchange. Manual mode waits for Sync now after a cycle; interval mode honors saved seconds, and near-real-time mode polls every 15 seconds. This version uses polling, not filesystem watching. Failures use bounded backoff in automatic modes. Cycles never overlap. Pause stops scheduling and new network operations at boundaries; an in-flight CLI merge can finish before Paused. Settings and updater installation require pause.

A persistent device stream, exported-content fingerprint and applied-object ledger avoid repeat export/import. Applied IDs are saved only after successful merge; interrupted or failed merges retry. Parent bundles apply before descendants. AMOS may normalize an unset link activation timestamp from null to empty text; the fingerprint treats these as equivalent while retaining original export bytes. Baselines are local per-device streams, not inferred from an incoming remote head.

## Limits and support boundaries

Versions 1–3 and the known record kinds are accepted. Maximum export is 8 MiB and 10,000 records; the replica remains bounded by 512 MiB / 4,096 immutable objects. Larger stores fail explicitly; chunking and history compaction are not implemented. Backups are local and currently require manual retention management. This is not a replica of embeddings, credentials or the running process. Each receiving device needs a working AMOS installation and the same recovered Drive space.

ChatGPT Work appears separately in the source list. Its local candidate may share CODEX_HOME; detection is not proof of native task migration. Other seven source adapters remain unavailable and are skipped, not marked synchronized. Official Work documentation separates local and cloud execution; no native cross-device import API was established in the reviewed pages: [Work setup](https://learn.chatgpt.com/docs/get-started-with-work), [local execution](https://learn.chatgpt.com/docs/enterprise/chatgpt-work-local-security).

## Validation

Tests cover automatic queue exchange, failed import retry, unchanged data, cancellation and all-source selection without a global veto. A separately invoked installed-CLI test uses two temporary memory homes to verify export, backup, semantic import and repeat stability. No physical second-device or Windows/Linux interactive acceptance is implied. See [validation](VALIDATION.md).
