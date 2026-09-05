# M2 snapshot protocol and local-folder transport

Status: implemented core, exercised with isolated fixtures. Native agent import and Google Drive API are not implemented. [Five-language guide](manual/en/guide.md).

## Scope and interfaces

Rust `sync::Replica` owns a separate local replica; `sync::LocalTransport` addresses a shared folder. Neither writes to an agent profile. `export_from` accepts explicitly prepared UTF-8 text artifacts with their original base snapshot ID, `sync` exchanges complete bundles and `resolve` records an explicit reconciliation. The GUI exposes only `run_sync_diagnostic`, with synthetic data in disposable directories. It does not pass saved agent paths or the selected Drive folder to this command.

`capture` reads an explicit staging-file list twice and rejects changed content. Both scans must match. Only `.md`, `.txt` and `.jsonl` are allowed; hidden/auth/credential/secret paths and symlinks are rejected. This is a **quiescent staging export**, not a transactionally consistent backup of live files. Content may itself contain secrets: filename exclusions do not sanitize text. Real agent export requires a versioned adapter and the M3 protection model.

## Bundle v1

A single UTF-8 JSON file contains `id` and `snapshot`. Snapshot fields are `schema` (1), `space`, `device`, `stream`, `parents`, and `files`. Stream fields are `agent`, `profile`, `conversation`. Each file maps a portable logical path to `sha256` and `content`.

- `id`: lowercase SHA-256 of `serde_json::to_vec(snapshot)`. Struct fields use the declaration order above; file maps use lexicographic key order. Parent IDs must already be sorted and unique. This is the v1 Rust codec contract, not a claim of RFC 8785 canonical JSON.
- Each entry has a SHA-256 of its UTF-8 content bytes. Full bundle validation is mandatory even when a complete filename is present.
- Physical filenames are only `<64 lowercase hex>.json`. Incoming logical paths are never used to write files. Snapshot files stay in an isolated inbox until future native import.
- Portable logical paths are ASCII letters/digits/underscore/dash/dot with `/` separators. Reject absolute paths, `..`, empty segments, backslashes, Windows device names, trailing dots, case-insensitive aliases and file/directory collisions. Unicode **content** is supported; Unicode filenames need adapter mapping.
- Limits: 8 MiB content/file, 256 files, 8 MiB aggregate content/bundle, 16 MiB serialized input, 4,096 objects and 64 MiB serialized valid objects per replica/transfer union. Counts and bytes are bounded during directory reads. This development ceiling must be revisited for large histories.
- Unknown schema or struct fields, malformed JSON, bad hashes, wrong space and invalid paths are rejected. Input limits are enforced before reading unbounded files; structural limits are enforced after bounded JSON decoding.

Hashes detect corruption, **not sender authenticity or confidentiality**. Drive members can create new valid bundles. M3 must add authenticated encryption and device trust before real conversation transport.

## Store and recovery

Shared folder: `space.json` plus `objects/`. Initialize on the first device; other devices connect to that same completed space. Creation uses an exclusive immutable write; folder identity is rechecked on every transfer. Two offline computers independently creating different spaces must not be treated as the same space.

Local replica: `identity.json`, `replica.lock`, `objects/` and `journal.json`. The lock is an OS file lock, released when the owning handle/process exits; it is never a distributed lock on Drive. Immutable objects are the recovery source of truth. The journal is a replaceable checkpoint of IDs, pending histories and stream heads, **not an append-only audit log**. If interrupted after writing an object but before checkpointing, the next open reconstructs the checkpoint. Old valid snapshots are retained as recovery versions.

Write to a temporary file, flush, publish without clobbering, then checkpoint. Unix directory entries are flushed; Windows power-loss guarantees depend on the filesystem. Temporary files are not complete objects. Partial, corrupt and unexpected remote files produce issue codes and stay available for a later retry. `sync` is a bounded one-shot operation; retry by invoking it again. Automatic scheduling/backoff remains M5.

Static symlink redirections in managed directories/files are rejected. This does not claim confinement against a malicious native process concurrently replacing path components under the same OS account. Such a process is outside the M2 trust boundary.

## Lineage, conflicts and directions

A verified child with absent parents is stored pending, not advertised as a usable head. Later receipt of parents promotes it. Parents must belong to the same stream. Quarantined ancestry is not re-published. Heads form a DAG frontier per agent/profile/conversation.

An adapter records the base ID when local work begins and passes it to `export_from`. Remote reception between local editing and export cannot silently change that base: exporting from the older base creates a preserved branch. Unknown or wrong-stream bases fail. Exporting content identical to its base returns that same ID, avoiding echo loops. The current-head convenience exporter is crate-private and used only by synthetic fixtures. Concurrent heads are preserved. An ordinary export refuses to silently merge them. Explicit `resolve` must supply all current observed head IDs; stale or incomplete choices fail. The resulting snapshot references both branches; originals remain intact.

Upload-only does not receive objects; download-only does not publish. Transport results distinguish published-to-folder, received-into-inbox, pending ancestry, conflicts and rejected inputs. Published-to-folder never means cloud delivery or native agent restoration.

There is no automatic deletion or garbage collection in v1. A missing source or removed remote object never deletes local history. Bidirectional re-publication may restore an absent remote object; deletion intent/tombstones require a future explicit protocol revision. Selected-agent/profile filtering must be enforced by the adapter layer before real transport is exposed.

## Tests

See [validation](VALIDATION.md) for automated and native UI evidence. The diagnostic uses two replicas, a shared folder, divergent edits, repeated synchronization, restart reconstruction and explicit resolution. Its displayed counts are derived from actual filesystem operations, not fabricated UI data.
