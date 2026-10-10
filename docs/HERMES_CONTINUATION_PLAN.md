# Hermes cross-host and cross-OS continuation — implementation contract

Status: prepared for **0.8.0**, implemented in source and locally fixture-tested; **external acceptance not yet earned**. Published 0.7.0 installers are unchanged and this release candidate is not yet published, committed or installed. This contract supersedes the earlier decision to treat a single restored conversation database as sufficient Hermes support. API keys and login credentials remain recipient-managed by explicit user agreement.

## Required result

A receiver must be able to continue the selected work, not merely open its transcript. Every required dependency is either restored, verified as already available, or reported as an explicit blocker. Readiness is never inferred from a packet transfer or a passing schema fixture.

## Work packages — implemented (source + local fixtures)

1. **Native graph and attachments.** Required native parent/compression relationships and all associated messages are retained, with unambiguous root session identity; explicit edges distinguish compression/delegation/tool children, parent cycles are bounded, and a dangling parent is reported and cleared. A finished async delegation result travels as an inert sidecar (no runtime row, no lease, no replay); an in-flight job is an explicit blocker. Referenced `@file:`/`@image:` attachments are captured from mapping-authorized roots, bounded, content-hashed and remapped; missing, symlinked, credential-like and out-of-root files are reported. Owner: `native_sessions` and its tests.
2. **Portable workspace.** Selected skills (with scripts/references/assets), project rules (`AGENTS.md`/`SOUL.md`), built-in memory (`memories/MEMORY.md`/`USER.md`) and an allowlisted, sanitized `config.yaml` are captured into the restored home. Received code is never executed; disabled executable dependencies are reported pending recipient approval. API: `hermes_workspace::capture(home)` returns a base64 file map; `validate(files)`; `allowed(path)`. The portable path no longer returns empty for Hermes.
3. **External memory.** The actual provider binding is inspected. Local memory uses the official export/import with the original identity and an isolated recipient store; unknown providers, a missing identity and an unbound home remain explicit blockers. No raw active-DB copy and no implicit broad private-memory export — the broad no-scope export runs only under the explicit binding. Store build withholds the external channel; built-in memory stays available.
4. **Project continuity and prerequisites.** Repository identity, commit/branch and the selected dirty/untracked/deleted working dependencies are included without overwriting receiver work; a restore writes into a new isolated directory. OS-specific commands, path mappings, model/provider setup, memory binding, attachments and skill dependencies are validated on the receiver by presence/PATH/OS probes that never execute received code, with opaque items acknowledged explicitly. Recovery UI distinguishes blockers, warnings and verified checks in all five locales.
5. **Integration acceptance (local).** Synthetic independent source and target homes with different path layouts; source bytes preserved, graph and file content retained, unsafe paths rejected, store read via the real Hermes storage API, next work step exercised with restored dependencies, causal return captured and idempotent rerun — all covered by fixtures. Windows/POSIX path semantics are unit-checked.

## Remaining external acceptance (not yet earned)

- Physical two-device delivery and physical cross-host resume.
- Live provider/model continuation and a captured causal return on a real model.
- Windows/Linux **runtime** acceptance (fixtures are not runtime acceptance).
- Cross-version Hermes compatibility beyond schema 31.
- Attachment completeness beyond the documented bounds.
- Mac App Store submission/approval.

Only executed platforms and real devices count as runtime acceptance; report these gates honestly while they are unavailable.

## Safety and scope

- No credential transfer, live agent-store replacement, automatic execution of received skills/hooks/MCP commands, deployment, commit, push, release or installation replacement.
- No silent truncation or dropping dependencies to obtain green tests. Explicit bounds and missing/unsupported dependency states are required.
- Existing uncommitted work and legacy agent adapter behavior are preserved.
- Fixtures are based on the installed source contract and independently checked against native reader APIs.
- Actual model continuation, physical-device delivery and signed sandbox acceptance remain separate gates; report them honestly when unavailable.

## Verification and closure

Each work package follows failing test → implementation → passing regression, with spec review before security/quality review. Final executed-test evidence goes into [VALIDATION.md](VALIDATION.md) (owned by the final verification pass, not this document), with matching five-language guidance, CHANGELOG, BastetMind source/topic/index/append-only log and AgentMemoryOS routing record at closure. The conversation database opening is not closure.
