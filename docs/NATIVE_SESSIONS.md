# 🐈 Native conversation sync — 0.3.2

Selecting a source now starts its real local conversation adapter independently of Agent Memory OS. The worker captures compressed native snapshots, exchanges authenticated encrypted Drive objects and adds missing sessions on the receiving computer. Different existing bytes produce `session_conflict`; the existing file is never replaced. Pause, open the snapshot library and restore a version into a new folder to retain both. This is additive synchronization, not automatic reconciliation of an active conversation.

| Selection | Local data | Boundary |
| --- | --- | --- |
| Claude / Claude Code | Claude Code project JSONL and subagent JSONL | Shared local coding profile; Claude cloud chats and Cowork VM state are excluded. Selecting Claude does not export its cloud chat history. |
| Codex / ChatGPT Work | `sessions` and `archived_sessions` rollout JSONL | Shared local profile, including local desktop tasks. Cloud tasks and general ChatGPT chat history remain account-managed. |
| Google Agy CLI | One SQLite online backup per conversation database | WAL-aware snapshot; schema checks for trajectory metadata/steps. Native CLI resume and auxiliary brain/project state remain unverified. |
| Grok Build CLI | Summary, update stream, raw chat history, plan, signals and rewind points | Search-index databases are excluded. Compaction checkpoint and external attachment completeness remain unverified. |
| Pi Agent | Native session JSONL, including inline content | Native format versions 1–3. External attachments and extension state are not bundled. |
| Agent Memory OS | Official CLI sync export/import with backup | Automatic official merge remains unchanged; see [AMOS contract](AGENT_MEMORY_OS.md). |

Claude and Claude Code, or Codex and Work, share one scan/exchange when their effective paths agree. Each selected card still receives a result; shared sources do not duplicate upload counters. Custom Codex/Work paths are separate scans. Claude's coding data comes from the Claude Code path setting, not its Electron cache directory.

## Operation

1. Complete the resumable five-step Drive wizard on each computer, using the same trusted space and recovery key.
2. Select sources, save, then Start. Manual mode waits for **Sync now** after a cycle; near-real-time mode checks every 15 seconds after the previous cycle finishes. Intervals never overlap.
3. Read per-source results: empty, syncing, complete, partial or failed. A missing source, unsupported record or changing file is reported independently. No source selection is silently treated as a successful transfer.
4. Missing local sessions are added automatically in download/bidirectional mode. Existing different sessions stay untouched. Deletions are not propagated by these file adapters.
5. Pause before using the snapshot library. **Restore to a new folder** uses a native folder picker and creates a unique child profile. Configure the agent's supported profile environment variable and resume by session ID where supported. Agy's separate-folder result is a database recovery artifact, not a verified standalone CLI profile. Project files, dependencies, absolute path remapping and account sign-in remain the receiving user's responsibility.

## Safety and capacity

Only the documented session paths are accepted on receive. Root credentials, browser stores, locks, sockets, hooks and general configuration are not copied. No received content is executed. Safe directory checks and create-only file writes prevent replacing an active history. Multi-file additions are retryable; a failure can leave already-added files, which are compared before retry. Conflicts retain both the immutable incoming bundle and the unchanged local file.

SQLite snapshots use the online backup API with a bounded busy retry rather than copying a live database and WAL separately. Text captures check size/mtime before and after reading; incomplete JSONL is retried. Main Claude histories retain their subagent files as one snapshot. Compressed packs are bounded to 384 MiB and split into 23 MiB encoded parts when needed; unpacked JSON is bounded to 512 MiB. A hashed manifest refers to immutable parts, and receive waits for every validated part before restoring any file. Unchanged parts reuse their IDs. Oversized conversations are explicit partial failures, not silently truncated. Protocol limits are now 32 MiB content, 64 MiB wire and 2 GiB per selected-agent replica/transfer union, with 4,096 objects. Upgrade both peers to 0.3.2 for larger snapshots. Automatic history retention and complete external attachment transport remain open.

Initial scans compress the data; unchanged text files use metadata hints. Batch capture avoids repeatedly loading the full replica for every conversation. Drive object revisions permit a persistent validated download cache; missing/changed revisions fetch again, and each cycle still validates the space-key proof. Local replicas, cache and recovery folders contain private plaintext and use the host's access protections; they are not a second encrypted-at-rest vault.

## Evidence and remaining gates

Fixture tests exercise all seven local selections, two isolated homes, additive restoration, repeated transfer, conflicts, malformed data, symlinks and WAL snapshots. Installed Codex app-server `thread/read` successfully reconstructed the synthetic conversation in an isolated profile without a model request. Installed Pi SessionManager also reconstructed a synthetic v3 user message from an isolated file. This is not physical two-computer acceptance or verification of every provider's native resume UX. Agy/Grok native continuation, Claude Desktop cloud/Cowork, cloud Work, external attachments, path remapping and three-platform interactive restoration remain open. See [validation](VALIDATION.md).

Primary format references: [Codex app-server](https://learn.chatgpt.com/docs/app-server), [Pi session format](https://github.com/earendil-works/pi-mono/blob/main/packages/coding-agent/src/core/session-manager.ts), [Grok session storage](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/17-sessions.md), [Claude Code workflows](https://code.claude.com/docs/en/common-workflows). Agy schema handling is based on local read-only schema inspection, not a published portability contract.
