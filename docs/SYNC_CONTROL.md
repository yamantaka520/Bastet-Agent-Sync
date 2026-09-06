# 🐈 Sync control center — 0.5.0

[Five-language guides](../README.md#languages) · [Adapter contract](NATIVE_SESSIONS.md) · [Evidence](VALIDATION.md)

## Progress, history and devices

Each selected source shows queued/running/result state, current stage and examined/processed item counts. During a Drive request it can show current HTTP payload bytes and an estimate of seconds remaining when the payload length and a sufficient sample are known. This is **one payload**, not a whole-cycle completion estimate. Metadata, validation and cache checks can finish without a transfer. Unknown totals remain unknown.

The local control center keeps the latest 500 completed, failed or interrupted cycles across restarts. Records include source counts and diagnostic codes, not conversation contents. Device reports contain a generated installation ID, user-selected device name, OS, Bastet version, selected agent names, outcome and report time. They are encrypted with the space key. Each installation updates its own preallocated Drive object, at most once per minute after successful refresh; upload-only/download-only direction is respected. The last 128 observed devices are retained locally. Reported time belongs to the sender; observed time belongs to this receiver. Neither proves that a device is currently online. A reporting failure is shown separately and does not invalidate successful conversation transfers.

## Network and schedule

Pause before editing, choose **1–6 concurrent storage groups** (default 3), then save. Aliases and overlapping stores remain sequential within a group. Conversation/AMOS tasks use the bounded scheduler; optional portable packages follow these tasks. No fixed speedup is promised.

Set upload and download limits in KiB/s; 0 means unlimited. The aggregate limit is shared across this app's Drive body readers, including concurrent sources. It paces payload consumption, not network-interface packets or other apps. OAuth and updater downloads are excluded. Connection buffers can receive ahead of consumption. Manual pause interrupts pacing and prevents new dispatch; in-flight safe operations must finish or time out before the worker stops.

Allowed hours govern **cycle start**, using the computer's local clock. An earlier end crosses midnight; equal times allow all day. A cycle already started can finish after the end. The 15-minute pause resumes automatically while the app remains running. Resume/sync now clears that temporary pause but still respects allowed hours. Temporary pause does not survive process exit. Settings do survive exit; synchronization still requires Start after launching the app.

## Conflict review and storage

Pause, open conversation snapshots, and choose Compare. Text previews are bounded; binary files show hashes and sizes. Codex snapshots can compare against either the Codex or ChatGPT Work local path. “Keep local and mark reviewed” records the compared fingerprint. A later local change invalidates that acknowledgement. “Keep both” creates a unique new recovery folder and never replaces the live version. Review is not a merge or deletion of a branch.

Local usage measures Bastet app data, including histories/replicas and the rebuildable download cache; it is not the agent's entire source directory. Drive usage sums the three Bastet object MIME types in the configured folder, including the key proof; it is not Google account quota. Measurement is on demand. Scans are bounded and do not follow symlinks. Cache cleanup requires an idle worker and removes only hash-named JSON files from the current space's download cache. It preserves local replicas, journals, settings, recovery folders and every cloud object. Remote-history garbage collection is intentionally deferred until offline-device retention and references can be protected.

## Portable settings and skills

Both options default **off**. Select agents, opt in, then Preview using the current draft settings. Expand a file to inspect the actual candidate content and uncheck unwanted files. Save to apply the selection. Preview lists unsupported/sensitive paths as excluded. Newly discovered eligible files are candidates on subsequent cycles while the category remains enabled; review your source content before opting in.

- Supported config files: `config.toml` and `settings.json` directly inside the selected agent profile. Only these top-level scalar preferences survive: `model`, `model_reasoning_effort`, `model_reasoning_summary`, `model_verbosity`, `personality`, `theme`, `language`, `locale`, `outputStyle`, `thinkingLevel`, `defaultProvider`, `defaultModel`. Nested tables, providers, MCP servers, hooks, permissions, paths and credentials are omitted. This is a limited preference subset, not full configuration migration.
- Profile `skills/`, `commands/` and `agents/` text files are candidates. The standard Codex profile additionally includes user `.agents/skills` under `shared-skills/` in the package, keeping it separate from profile skills. Custom profiles do not implicitly read that shared directory. Project repositories, plugins, admin/system skills and binary assets are not copied.
- Allowlisted extensions: md, txt, json, yaml, yml, toml, py, js, ts, sh. No symlinks, hidden directories, dependency/build trees or credential-like filenames. Limits: 1 MiB per file, 16 MiB total text, 256 files, bounded recursion. Recognized credential patterns are rejected. This is **not general secret detection**; arbitrary text may contain private data and must be reviewed.
- AMOS uses its official backup/merge interface and has no portable-preferences package.

Portable objects use a separate encrypted MIME type. Received files never automatically install into a profile or execute. Open Received packages, Compare, then restore to a new folder. Review scripts/instructions before manually adopting them; restore does not install dependencies or preserve executable permissions. `shared-skills/` is a recovery layout, not an automatically registered Codex skill directory. Disabling sync or deselecting a file stops future exports but does not delete older remote snapshots.

The config subset and skill structure were checked against the official [Codex configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference) and [skill documentation](https://learn.chatgpt.com/docs/build-skills). These sources describe Codex; they do not establish other agents' configuration compatibility.

## Agy / Grok acceptance and continuation

Local installed-CLI acceptance, without a second computer:

- **Grok:** synthetic ACP user/assistant notifications were captured, compressed, encrypted, decrypted and restored through Bastet. The installed `grok export` read both sentinel messages from an isolated `GROK_HOME`. Recovery offers a copyable, quoted `grok --resume` command for POSIX shells or PowerShell. This command is a convenience, not evidence that a model resumed a complex session. Account configuration and the corresponding project must exist separately.
- **Agy:** a new dedicated test conversation was created with the installed logged-in CLI. A later `--conversation` invocation repeated its prior marker. Bastet's WAL-consistent snapshot, encrypted round trip and separate-folder restore passed SQLite integrity and nonempty-step checks. The installed CLI exposes no verified alternate profile switch. Its continuation check used the original test profile; restored-profile model continuation, auxiliary brain/project state and physical transfer are **not claimed**. No misleading one-click launch is provided for a separate Agy recovery folder.

Grok's source-of-truth update format was checked against its [official storage implementation](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-shell/src/session/storage/mod.rs). Complex compaction, external attachments and project remapping remain adapter limits. See [native sessions](NATIVE_SESSIONS.md).
