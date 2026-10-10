# 🐈 Hermes Agent synchronization

Status: the Hermes Agent source shipped in **0.8.0**; **0.8.1** is a maintenance patch that makes multipart readiness honest (present + index consistency, labelled "parts received — integrity will be verified before restore", never "ready"/"verified"). Published 0.7.0/0.8.0 installers are unchanged by 0.8.1 until its own publication completes. Final executed-test evidence belongs in [VALIDATION.md](VALIDATION.md); fixture success is not physical-device or live-model acceptance.

**Superseded scope.** The first Hermes change restored one conversation database only and treated skills, rules, built-in/external memory, sanitized config, project working state, attachments and native cross-session lineage as out of scope. That scope is superseded by the user's requirement to continue actual work across hosts and operating systems; API keys and login remain recipient-managed. The continuation behavior below is implemented and locally fixture-tested. The external acceptance gates at the end stay open — this document does not claim them.

## Workflow

1. Select **Hermes Agent** (`hermes`) and save. Discovery uses `HERMES_HOME`, falling back to `~/.hermes`; a custom path takes precedence.
2. A named profile is selected by setting its home directory as the custom source. Only one Hermes home is selected at a time; this is not automatic multi-profile enumeration.
3. Configure the encrypted destination and explicitly start synchronization. The adapter uses the native session worker, not an external agent or a raw cloud-folder database mirror.
4. Authorize project continuity in **Project mappings** by mapping each source working directory — the session `cwd` recorded in the native store — to a local folder. Project files, referenced attachments and the resume workdir are read only for a `cwd` covered by an explicit mapping. An unmapped `cwd` is never read and yields a `project_mapping_required` blocker; a receiver that has not mapped the source gets the same blocking sidecar instead of an implicitly "ready" version.
5. Pause before inspecting received snapshots, restore into a new managed home, then continue. The supplied POSIX or PowerShell command is scoped to that restored `HERMES_HOME` and to the restored project directory, and is displayed, never executed by sync.
6. Configure the local Hermes installation, model provider and login separately. Recipient credentials are never transferred.

## Native conversations and lineage

- The source is the native `state.db`, read through SQLite's online backup interface — never a raw live database/WAL copy. One consistent whole-store backup is taken per cycle (512 MiB); each selected conversation is rebuilt into a fresh allowlisted schema-31 database.
- Message ordering, tool calls/results and available reasoning metadata are preserved; persisted session `cwd` is remapped along with the manifest.
- Required native lineage is retained: a continuation/compression parent's link and end reason travel with the child, an explicit delegation edge is distinguished from a compression continuation, and a `tool`-source session child is not required lineage. Parent cycles are bounded and preserved; a dangling parent reference is reported (`hermes_parent_missing`) and cleared rather than left broken.
- A finished asynchronous delegation result travels as an **inert sidecar** — its records, task and result metadata only. The rebuilt store carries no runtime row and no `session_turn_leases`/`compression_locks` entry, so nothing is restarted, resumed or replayed. An in-flight job (`running`/`finalizing`) is an explicit `hermes_async_delegation_incomplete` blocker with no records; the live source store is never modified.
- A restored multi-session lineage re-captures from the managed home as an unambiguous causal child of the restored head.

## Portable workspace (skills, rules, built-in memory, config)

Captured as inert base64 bytes and never executed; materialized only through an explicit restore into a new isolated home:

- **Skills** with their `scripts`/`references`/assets, bounded by file count, per-file size, total size and depth.
- **Project rules / identity** (`AGENTS.md`, `SOUL.md`) and the **built-in** curated memory (`memories/MEMORY.md`, `memories/USER.md`).
- **`config.yaml`**, strictly allowlisted and sanitized. No secret — key, token, credential, `.env`, `auth.json` or runtime value — is included; MCP `command`, `args` and `env` **values** are dropped and only the server *name* and its env-variable *names* survive as inert requirements; a received MCP `command`/`args` is refused outright.
- A referenced support file that was not captured, an unparseable or non-text `config.yaml`, an excluded sensitive/unsupported path, and the size bounds are reported as explicit blockers/warnings, never silent drops. Skill `platforms:` is retained as metadata and re-evaluated on the **receiving** host.

Filename-heuristic secret exclusion proves a path is never captured; it cannot redact a secret a *non-secret-named* file happens to contain. Content-level secret scanning remains a human-review requirement and is not claimed here.

## Referenced attachments

- Bounded, explicitly referenced attachments are captured from **authorized roots only** (`@file:` and `@image:` references; a URL is never read as a local file). A recorded `cwd` is a read root only when an explicit mapping covers it.
- Each captured blob is content-hashed and its reference remapped into the restored home; a Windows reference is re-rooted for the receiving platform.
- Missing files, symlinks, credential-like paths and out-of-root references are reported (`hermes_attachment_missing`/`_symlink`/`_credential`/`_outside_roots`), never followed or silently dropped. Attachment completeness is bounded by the documented limits, not guaranteed complete.

## External memory (Agent Memory OS)

- The external store travels only when the profile's `config.yaml` selects `agent-memory-os` **and** an explicit, consistent memory home is bound (Bastet's custom path, the profile's own binding file, or the `AGENT_MEMORY_HOME` fallback). Every other case — including a missing/unreadable `config.yaml` — fails closed with an explicit blocker, never a silent built-in assumption.
- The export is the official `agent-memory sync export` with **no team/project scope**, which by the provider's own contract includes the profile's private memories and its team/project/ACL "authority" records: this is same-identity continuation, not a raw database copy, but it **is broad**. It is gated entirely on the explicit binding above; the manifest records the record count and whether authority changes are present. No content-level secret guarantee is made — secrets that happen to live inside memory content are not redacted.
- Restore requires the recipient to have opted into the source **and** the provider plugin to be discoverable from the isolated home; the official CLI imports into a **new** nested store this adapter reserves atomically, then re-exports it and requires the recalled records, authority flags and normalized fingerprint to match the capture. A `memories.db` that merely exists is not recall, and the provider's availability flag is not trusted.
- An existing, recipient-modified store is never overwritten: a matching snapshot imports once and repeat sync is idempotent; a store that has diverged is reported (`memory_store_diverged`), never force-merged.
- **Mac App Store build:** the external channel is withheld up front — capture reports `memory_unavailable_store`, carries no export and never invokes a CLI, and no store is placed. The **built-in** provider stays fully available in the Store build.

## Receiver-side readiness, explicit actions and acknowledgment

A restored version is never marked ready merely because it was copied; a version with no readiness report is treated as unverified, not ready. Its per-version continuation report (blockers, warnings, verified facts), localized in all five languages, is authoritative. Receiver-side probes run **without executing received code**:

- `env` — the variable's **presence only** (`var_os`), never its value.
- `runtime` — the command name is looked up in the trusted absolute local `PATH` and must be an executable regular file; the command is never run. A package-spec-like name (`pip:foo`) is opaque instead.
- `os` — the declared `platforms:` list is re-evaluated against the receiver OS.
- `app`/`credential`/`mcp`/`model`/`memory` — opaque; they need the recipient's explicit acknowledgment and are **never** reported as verified.

A verifiable probe that fails is `missing` and stays blocking, never downgraded to a warning. An opaque requirement stays blocking until acknowledged **for the exact received manifest**; a changed manifest carries a different fingerprint and the old acknowledgment no longer matches. Explicit, user-initiated actions re-evaluate a version against its **already registered** isolated managed home — the backend resolves the target from its own registry by version id, never accepts a caller-supplied path, refuses while sync is running, and never creates another managed profile:

- **Recheck continuation** re-runs dependency and memory readiness and re-applies the official memory import when the provider plugin is now discoverable from that home.
- **Set up memory provider** runs the trusted, configured local CLI's fixed `agent-memory hermes install --hermes-home <EXACT HOME>` operation into that home only — no received code, no download, a single bounded command — then re-runs the official import. Installation is proven by reading the written provider shim back; readiness is proven by re-exporting the restored store and matching the captured fingerprint.
- **I configured these prerequisites; recheck** records an explicit, hash-bound acknowledgment of the opaque requirements (MCP servers, custom model provider, host apps, credential files). This is an acknowledgment, **not a validation**: acknowledged items stay as warnings and are never reported as verified, the acknowledgment is voided when the requirement set changes, and verifiable prerequisites (environment variables, commands, operating system) still block.

The external channel and **Set up memory provider** action are unavailable in the sandboxed Mac App Store build.

## Data boundary

- Restore only into a separate managed home; the active source is never overwritten and continued versions use the existing causal-parent and branch-preservation mechanism.
- Credentials, `.env`, `auth.json`, API keys, login material, cron jobs, gateway routing and running execution state are **not** migrated. Recipient credentials and model/provider setup are the recipient's own.
- Content embedded in messages is still private and may contain secrets; excluding credential-named files is not content redaction.
- Received data is treated as untrusted: schema/data isolation, exact column-set checks, integrity checks and rejection of executable schema objects (foreign tables, triggers, views) are mandatory, not best-effort cleanup.

## Compatibility and remaining acceptance gates

The implementation supports schema **31** with exact column-set checks; older/newer versions and column drift are refused (`session_format_unsupported`) rather than silently dropping fields. Other tables and FTS content are not copied; runtime/gateway tables start empty.

Not established by local fixture tests, and requiring separate real-world evidence:

- physical two-device delivery and physical cross-host resume;
- live provider/model continuation and a captured causal return;
- Windows/Linux **runtime** acceptance (path semantics are unit-checked, but only executed platforms count as runtime acceptance);
- cross-version Hermes compatibility;
- attachment completeness beyond the documented bounds;
- App Store submission or approval.

No current installation is replaced as part of this source change; publication is pending and this preparation makes no commit, push or release by itself.

Related: [implementation contract](HERMES_CONTINUATION_PLAN.md), [native adapters](NATIVE_SESSIONS.md), [managed handoff](CROSS_OS_HANDOFF.md), [security](CLOUD_SECURITY.md), [requirements](../REQUIREMENTS.md).
