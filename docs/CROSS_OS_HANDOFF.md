# 🐈 Cross-OS conversation handoff — 0.6.0

Status: implemented in unreleased source. Results and remaining gates are recorded in [validation](VALIDATION.md). Published 0.5.1 does not include this behavior. Upgrade all participating computers before relying on continued-version handoff.

## Version handoff

The authenticated, encrypted Drive transport carries immutable native conversation snapshots. The receiving computer prepares each usable incoming version in a **separate managed profile**, rather than adding files to the user's default agent store. Source histories and previously prepared profiles remain unchanged.

When the user continues that profile, Bastet captures its changed conversation and publishes a child of the received snapshot. The original stream identity is retained; the snapshot records the exporting device. A local fingerprint prevents a freshly restored or path-translated profile from being uploaded merely because it was restored.

A publishes a conversation, B prepares and continues it, and A receives B's child in a new profile. A can continue that version and publish another child. If both computers edit the same base independently, both heads remain available. Bastet never concatenates histories or chooses a winner by timestamp. The snapshot library shows version depth, branch count and parent IDs.

## Configure and continue

1. Join the same Drive space using its recovery kit on every computer. Each computer needs its own Google authorization and relevant agent installation/account access.
2. Place the project locally using its normal project/Git workflow. In **Project paths**, enter the source project's absolute path and select the corresponding existing local folder. Save settings. Multiple source paths can map to one local project; more-specific mappings take precedence at directory boundaries.
3. Select the agent and start synchronization. Selection is available before its default data folder exists. An unresolved project path retains the snapshot and reports that a mapping is needed.
4. Pause synchronization, view snapshots and select **Continue restored version**. Copy the command for POSIX shells or PowerShell. It scopes the agent's data directory and working directory to that prepared version. Credentials are not transported; an isolated profile may need local sign-in or provider configuration.
5. Restart synchronization to publish subsequent edits from the registered profile. Only selected agents and the configured direction are processed.

**Restore to a new folder** also registers the restored profile for future capture. Agy is an exception: its database remains a recovery artifact and is not advertised as a working continuation profile.

Changing a mapping does not rewrite a profile already in use. That profile retains its original local project path. Restore another profile to apply a different mapping.

## Provider-specific mapping

| Provider | Location metadata | Boundaries |
| --- | --- | --- |
| Codex / local Work | Session and turn-context `cwd`; date-grouped rollout storage | Desktop sidebar/worktree parity is a separate gate. |
| Claude Code / local Claude selection | Structural `cwd` and verified project encoding, including subagent paths | Cloud Claude/Cowork is excluded. |
| Pi | Session-header `cwd` and native project encoding | External extensions and attachments are not provisioned. |
| Grok | Summary `cwd` and URL-encoded project group | Index/checkpoint and attachment completeness remain separate gates. |
| Agy | No database path rewriting | WAL-consistent recovery; explicit same-host database restore/resume was tested, but stored absolute workspace paths are not remapped. |
| Agent Memory OS | Existing official export/import adapter | Requires its compatible CLI and initialized local store. |

Mapping understands Unix, Windows drive and UNC absolute paths. Windows source matching is case-insensitive; Unix matching is case-sensitive. Whole directory components are matched, never a textual prefix such as `app` inside `apple`. Destination folders must exist locally. Immutable snapshots retain their original paths; only the prepared profile is translated.

Historical messages, tool arguments/results and recorded commands are preserved. Rewriting provider-owned location metadata does not rewrite evidence of what happened in the original conversation.

## Integrity and acceptance

Native and portable packages share Unicode-preserving filename validation. Windows-invalid characters, reserved device names, trailing spaces/dots, traversal and oversized components are rejected. NFC/case aliases and file/directory collisions are checked before writing, including differently cased directory names.

Profiles are registered after staging and atomic directory rename. Registry state advances after the immutable child exists, permitting idempotent retries. Active default stores are not import targets. Local profiles use host filesystem protections rather than a second encrypted vault.

The dedicated workflow exchanges encrypted synthetic fixtures between real macOS, Windows and Linux runners in all six directed combinations, then returns continued artifacts to their originating OS. Results must be checked in validation; defining the workflow alone is not a passing result.

The installed-reader harness reads restored synthetic profiles where a provider offers a noninteractive reader. CI pins and requires Codex 0.159.2, Claude Agent SDK 0.3.276 and Pi 0.85.1; a missing required reader fails the job. Grok is optional in CI and Agy receives only a database integrity check. A reader/export check does not prove model continuation. Full acceptance also requires native resume, a new model turn, correct local tool execution and another handoff with the intended provider versions.

Project code, Git worktrees, uncommitted changes, dependencies, credentials and external attachments are not implicitly synchronized. General multi-profile source selection remains separate from the managed continuation profiles tracked here.

Actual model/tool continuation and encrypted causal return passed for Codex, Claude Code, Pi and Grok in isolated same-host profiles. Agy separately passed explicit native database recovery/resume, but retained its original absolute workspace path. See [measured validation](VALIDATION.md) for these boundaries and the completed six-direction hosted OS exchange.
