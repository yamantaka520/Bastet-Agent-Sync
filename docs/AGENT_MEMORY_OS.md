# 🐈 Agent Memory OS transport adapter

Status: source discovery, selection, JSONL envelope inspection and byte-preserving snapshot adapter implemented. Automatic export, Drive queue orchestration and application into a running AMOS store are **not enabled**. Selecting this source does not start memory synchronization.

## Source and protocol

The seventh source is `agent-memory-os`. Discovery honors a manually selected directory, then `AGENT_MEMORY_HOME`, then `~/.agent-memory`. Discovery checks directory existence only. It never opens the live SQLite database, WAL, credentials or embedding store.

Use the installed AMOS CLI's official file-bundle interface:

```sh
agent-memory --home /absolute/path/to/memory-home sync export /absolute/path/to/export.jsonl
```

This is a user-run export command, not a command automatically executed by Bastet. The AMOS local export defaults to including private memories; review the intended recipient and keep the export outside shared folders until encrypted transport is configured. Its `--team` option filters a team export, but is not a guarantee that every metadata record is restricted to that team. Scope filtering and native import trust must be implemented and tested before automatic operation.

In Bastet, **Agent Memory OS → Choose exported JSONL** performs a read-only envelope check. Only version, record count and an authority-change notice are returned to the UI. Cancelling or failing an inspection clears the previous result. This is not AMOS semantic validation and does not authorize import.

## Preservation and limits

- Supported envelope versions: 1, 2 and 3; unknown versions or record kinds fail closed. AMOS itself tolerates unknown kinds, but this first transport adapter refuses to silently transport an unreviewed future record type.
- Record kinds: memory, link, profile; version 2 adds tombstone; version 3 adds team, project and org_tombstone.
- The original UTF-8 JSONL is preserved byte-for-byte in `agent-memory-os.jsonl`, under the dedicated `agent-memory-os` stream namespace. Owner, visibility, timestamps, pins, feedback counters, links and organization fields are never rewritten by Bastet.
- Maximum export size: 1 MiB; maximum records: 10,000. Larger exports are rejected, never truncated. Pagination/chunking remains future work.
- The adapter can capture a prepared export into the existing immutable replica protocol and restore it from a validated bundle. Fixture tests verify two-replica transfer plus authenticated encryption round trip. The GUI currently exposes inspection only, not publication or application.

## Import and concurrency gates

AMOS `sync import` is a trusted local/admin merge and may apply tombstones and organization membership changes. Do not automatically invoke it for a downloaded bundle. A future application step must show origin/trust, preview deletions and ACL changes, create an AMOS-supported backup, and use AMOS's transaction/import semantics. Retain Bastet branches when two devices export concurrently; never concatenate JSONL or silently choose the newest file. AMOS's own conflict rules need separate acceptance tests before enabling automatic merge. Do not run both AMOS peer synchronization and a Bastet transport for the same scope without a deduplication/ownership plan.

## Evidence

Contract reviewed at AMOS commit `d190e9e5f0612a2859cd93ed9a0796bded08126f`: [version registry](https://github.com/yamantaka520/Agent-Memory-OS/blob/d190e9e5f0612a2859cd93ed9a0796bded08126f/src/agent_memory_os/sync_bundles/contract.json), [export/import implementation](https://github.com/yamantaka520/Agent-Memory-OS/blob/d190e9e5f0612a2859cd93ed9a0796bded08126f/src/agent_memory_os/sync.py). The synthetic test fixture derives from that repository's v1/v3 contract fixtures. No live memory records were exported or imported during implementation.
