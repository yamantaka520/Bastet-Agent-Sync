# Changelog

## Mac App Store — information supplied and resubmitted 2026-10-03

- Record Apple’s Guideline 2.1 information request and `REJECTED` status. Prepare an internal TestFlight group containing the submitted build, with automatic distribution disabled. Prepare a six-part response/notes draft, capture checklist and synthetic sample. After the owner updated the Mac to `27.0.1` and authorized the invitation, the invitation was redeemed and the exact TestFlight build installed and launched in its production sandbox; Google setup, one synthetic bundle upload and isolated restore with verified project-path remapping passed on one physical Mac; a privacy-edited 7:26 physical-Mac recording and synthetic ZIP were attached, six-part Notes saved, and Apple confirms WAITING_FOR_REVIEW after resubmission; no specific runtime defect was identified in the review message.

## Mac App Store — submitted 2026-10-02

- Formally submit macOS `0.6.0` build `1`; Apple website and API confirm `WAITING_FOR_REVIEW`. Release remains manual with France excluded (174 territories). Complete the published non-OS encryption questionnaire, publish privacy disclosures and save the owner-confirmed content-rights answer. Review notes explicitly describe Google OAuth access. This is not Store approval or publication.

Earlier preparation milestones below record their state at the time; the submission above supersedes their pending-form status.

- Exclude France from the first Store release by explicit owner instruction; App Store Connect confirms 174 available territories. Encryption compliance remains open.

- Fix Store recovery-kit export to write the exact Save-dialog-selected file with exclusive creation and private `0600` mode; preserve identical-content retry and refuse replacement of different content. An isolated signed sandbox exported and verified the kit, completed an encrypted Drive check and uploaded a synthetic Claude snapshot. A malformed synthetic restore was rejected; a corrected fixture restored into a separate profile with translated project metadata and retained session ID. Actual model continuation, return capture and two-device acceptance remain open.
- Replace the Store wizard's Agent Memory OS completion prompt with supported-agent guidance in all five locales, while keeping the direct-download message. Upload five real native 2560 × 1600 screenshots; Apple reports all five `COMPLETE`. The latest universal Store package passed validation at 17:54 and upload at 17:59 Asia/Taipei on 2026-10-02. App Store Connect reports version `0.6.0` build `1` as `VALID` and `APP_STORE_ELIGIBLE`; export declaration, TestFlight testing and App Review remain open. The candidate includes working-tree Store fixes beyond its displayed base revision.
- Prepare a private French encryption technical draft without submitting a declaration. The expired XChaCha20 draft alone does not establish proprietary status; the applicable classification and France declaration remain unresolved. Privacy labels remain unpublished.

- Publish five-language product and privacy pages using the canonical calico logo, add the in-app policy link, save all five Store policy URLs, and enable the dedicated Google OAuth production audience after owner approval. Website ownership passes; the owner-authorized Google brand-review request is submitted and under review, not yet approved. The later Store package upload is recorded above; no App Review submission is implied.

- Create a dedicated Google Cloud project and desktop OAuth client with Drive API and only `drive.file`. Embed the client in the Store candidate and verify real consent, callback and persisted credential reconnection in an isolated ad-hoc sandbox. Public/reviewer login and production-profile acceptance remain pending.

- Normalize Store package permissions before final signing, check embedded privacy-manifest contents, and pass Apple server-side package validation. Add the candidate privacy manifest and source-based assessment; save age-rating and unpublished privacy drafts. Upload succeeded later; processing and App Review remain pending.

- Retry transient setup-state lock contention at startup without leaving a stale error; preserve genuine startup and action failures. Read-only snapshot and operations views also retry short file-lock contention and show localized busy/setup guidance in all five locales.

- Add an opt-in `mac-app-store` build that omits direct updater/plugin commands, uses a separate Keychain service, and presents Store update guidance in all five locales. The first Store release excludes Agent Memory OS in backend and UI by explicit product decision; the default direct-download channel retains existing updater and AMOS behavior.
- Implement security-scoped picker bookmarks, scoped reads for worker and restore flows, and same-path handoff reauthorization. Build and verify the universal Store-signed `.app` and `.pkg` at version `0.6.0` build `1`; this is a local candidate only.
- Save five-language Store descriptions and keywords, support and marketing URLs, and account draft selections. App Review notes explain the login and synthetic demonstration; its login checkbox and blank credential fields still need resolution. Local validation and limits are in [validation](docs/VALIDATION.md); production-profile, moved/revoked grants, model continuation/return, TestFlight, final privacy/review materials, App Review and publication remain pending. Public 0.6.0 assets are unchanged.

## 0.6.0 — cross-OS conversation handoff and sync performance

Published 2026-10-02: [installers and five-language notes](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.6.0). Four architecture builds, three-platform CI and installer checks, six directed cross-OS returns, seven updater signatures and all release checksums passed. Actual continuation and performance boundaries are recorded in [validation](docs/VALIDATION.md).

- Avoid repeated compression of unchanged native captures, coalesce overlapping cloud listings and throttle routine progress notifications while preserving final counters. See [sync controls](docs/SYNC_CONTROL.md) for cache and validation boundaries.
- Verify isolated real-Drive upload, cache reuse, append and receive/restore with synthetic sessions, plus actual Pi, Claude Code, Codex and Grok model/tool continuation and encrypted return. Agy explicit database recovery/resume also passes in isolated same-host profiles, while workspace remapping remains unsupported. Measurements and remaining platform gates are recorded in [validation](docs/VALIDATION.md).
- Prepare received versions in separate managed profiles, track continued edits as descendants of the received snapshot, and preserve concurrent branches. Default agent stores are no longer import targets.
- Add saved project mappings, provider-specific path translation, version/branch details and scoped POSIX/PowerShell continuation commands for Codex, Claude Code, Pi and Grok. New receiving computers can select an agent before its default data folder exists.
- Reject cross-platform filename and case/Unicode/path collisions in native and portable payloads before restoration.
- Add OS-to-OS fixture exchange and return checks for macOS, Windows and Linux, plus optional installed-provider read checks. See [handoff contract](docs/CROSS_OS_HANDOFF.md) and [validation](docs/VALIDATION.md) for measured results and remaining gates. Agy remains database recovery; full model continuation is not implied.

### macOS Developer ID signing

- Add isolated Apple Silicon and Intel Developer ID signing and notarization workflows, reused by release packaging. Require signed, notarized and Gatekeeper-accepted app, DMG and updater contents before upload.
- Both 0.6.0 Mac architectures passed hosted and independent signature, notarization-ticket, Gatekeeper, version, architecture and updater verification. A previous Intel runner lost network access while polling; its replacement release build passed. Earlier public assets were not replaced. Windows publisher signing remains deferred.
- Make Windows publisher-signing deferral an explicit release input. Required signing remains the default and never automatically falls back; the deferred path preserves updater signatures and installer smoke requirements and discloses unsigned Windows publisher status.

## Pending — Windows publisher signing

- Add a reusable Windows build workflow with GitHub OIDC and Microsoft Artifact Signing. Tauri signs application and installer binaries before updater signatures are generated. Signing failures block package upload and release drafting.
- Validate installer signatures, timestamps and the application inside both NSIS and MSI; launch the NSIS-installed application on a hosted runner. Azure identifiers remain repository variables. Azure OIDC and signing requests now succeed after the federated-subject correction. Read-only Azure management inspection confirms the configured profile uses Private Trust CI Policy. Windows rejects the issued certificate chain as untrusted; public-trust profile acceptance remains pending and no signed installer has been published.

## 0.5.1 — encrypted-space diagnostics

- Distinguish `foreign_space`, `unsupported_encryption_version` and `encrypted_space_mismatch` after strict proof verification; skip only non-proof foreign-space objects and report `foreign_space_objects` as a partial result.
- Apply the encrypted-space classification to native sessions, Agent Memory OS, portable packages and device reports. Never import foreign objects, auto-select keys, or suppress decryption, corruption or version failures.
- Reject a non-empty new-space session-object folder with `folder_has_sync_objects` before key/proof allocation. Joining requires the original recovery kit; selecting the same folder or observing some downloads is not proof of a common space key.
- Published 2026-09-09: [v0.5.1 installers and five-language notes](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.5.1). Four architecture builds, independent three-platform CI and installer checks passed. Seven updater signatures, 19 digests, 18 checksums and nine public links were verified. Physical two-computer sync remains unverified.

## 0.5.0 — sync control center

Published 2026-09-07: [three-platform installers and five-language notes](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.5.0). Four architecture builds, independent CI and all installer checks passed; seven signatures, 19 digests, 18 checksums and nine public links were verified.

- Run 1–6 independent storage groups concurrently (default 3); aliases sharing a canonical agent and path execute once. Shared journals and overlapping source paths remain sequential.
- Show all selected sources as queued before dispatch, update each result independently, and count shared work once. Pause stops new dispatch and joins active work before another cycle.
- Make the Drive revision cache thread-safe and coalesce same-object reads with bounded locks. Always fetch space-key proof from Drive. Existing per-source ordering, retry cadence and conflict rules remain.

- Add per-source stages/counts/current HTTP bytes and sampled payload ETA; persist the latest 500 cycles and encrypted last-reported device metadata.
- Add aggregate upload/download limits, local allowed hours and 15-minute pause/resume; expose app/cache/Drive object usage and safe idle cache cleanup.
- Compare incoming/local text or hashes, acknowledge an unchanged fingerprint, and preserve both versions with separate-folder recovery.
- Add opt-in sanitized preferences and text skills, draft preview, per-file exclusions and inert reviewed receiving packages. Standard Codex shared user skills remain separate. New MIME types keep older clients from treating reports/preferences as conversations.
- Validate installed Grok export after an encrypted isolated restore; validate dedicated original-profile Agy continuation and restored SQLite integrity. Add safely quoted Grok continuation commands. Full physical-device or restored-profile Agy model acceptance is not claimed.
- Preserve five-language UI/docs and all installer targets. User-reported macOS in-app upgrade success is recorded. Apple notarization and Windows Authenticode remain excluded.

## 0.4.3 — compact runtime status and Drive traffic

Published 2026-09-06: [three-platform installers and five-language notes](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.4.3). Four architecture builds, all three installer smoke jobs, seven update signatures, 18 checksums and nine public download links passed.

- Move app, Google and sync states into one responsive row immediately below Drive setup status.
- Show separate upload/download rates and session totals in all five locales, refreshed each second. Rates use approximately three seconds of Drive HTTP payload consumption; no OS-wide traffic or transfer-success inference.

## 0.4.2 — language switching fix

Published 2026-09-06: [three-platform installers and five-language release notes](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.4.2). Four builds and all installer smoke jobs passed. All 19 assets, seven update signatures, 18 checksums and nine public download links were verified. The full documentation set was reviewed and reconciled with current behavior.

- Keep the language selector available during synchronization. Persist locale immediately through a locale-only native command and refresh tray labels without pausing the worker.
- Preserve other saved settings and unsaved UI edits; failed writes retain the displayed language. Locale changes work before setup completion and with disconnected source paths.

## 0.4.1 — credential access and clearer sync results

Published 2026-09-06: [installers and five-language release notes](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.4.1). All four builds, three installer checks, seven update signatures and nine public download links passed.

- Group snapshots by agent with collapse/expand controls, local save timestamps and newest-first ordering.
- Show per-agent result cards, labelled transfer counts and actionable five-language issue summaries; retain technical codes in expandable details.
- Add five-language credential preparation guidance and an explicit read-only preparation button.
- Cache successful native credential reads in zeroizing process memory; serialize concurrent misses and keep failed/missing reads retryable.
- Rotate cache only after persisted writes; evict on failed writes/removals. Clear on logout, setup restart, client replacement and normal exit.
- No OS authorization bypass or Developer ID signing change.

## 0.4.0 — first public installers

Published 2026-09-06: [installers and five-language release notes](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.4.0). Nineteen assets, native installer smoke checks, seven update signatures and public download links verified.

- Four architecture targets across macOS, Windows and Linux; release notes link directly to every installer.
- Embedded offline WebView2, static Windows CRT, declared Linux dependencies and checksum-verified apt/dnf installer.
- Complete-platform draft gate, signed updater manifest and SHA256SUMS; five-language installation guidance.


## 0.3.2 — Independent large-source capacity

- Apply the 2 GiB union limit per selected agent, so an unrelated large conversation source does not consume AMOS/Pi/Agy's budget.
- Avoid retaining duplicate local/remote bundles in the exchange union; move validated transport objects instead of cloning their payloads and reuse the loaded graph when checkpointing receives.
- Fetch up to 1,000 Drive metadata entries per page while preserving complete-list and revision checks.

## 0.3.1 — Large-history recovery

- Split large compressed conversations into immutable parts with a hashed manifest. Receive waits for every verified part before adding any native file; unchanged parts keep their object IDs.
- Keep transferred counters when a later source is paused; show system-credential access separately from conversation processing.
- Detect a session already stored under a different project/archive path instead of introducing a duplicate native identity.
- Increase the bounded replica/transfer union to 1 GiB and use an optimized macOS build for real histories.

## 0.3.0 — Local conversation sync preview

- Connect selected local conversation sources to compressed native snapshots and encrypted Drive exchange, independently of Agent Memory OS.
- Automatically add missing receiving sessions; preserve existing different versions and offer separate-folder recovery in a five-language snapshot library.
- Show per-source empty, syncing, complete, partial and failure results. Deduplicate shared Claude/Claude Code and Codex/Work profiles.
- Use SQLite online backup for Agy, retain Claude subagent files, batch large captures and cache Drive downloads by provider revision.
- Keep cloud chats, full settings/skills, external attachments and unverified native continuation explicitly outside this milestone; see the native adapter contract.

## 0.2.1

- Automatic Agent Memory OS export, encrypted Drive exchange and backed-up official merge; manual/interval/15-second scheduling, pause and per-cycle counters.
- ChatGPT Work listed separately; unsupported sources no longer veto ready memory synchronization.
- Manual JSONL inspection moved under Advanced. Native conversation adapters remain unavailable.

## 0.2.0 — Runtime status and signed updater

- Replace permanently disabled Start with explicit native preflight and visible reasons; connect Drive status to wizard state. No automatic Agent worker is claimed.
- Add emoji status, native version/build below the logo, and a version-setting/checking workflow.
- Add signed online update checks, download/install progress and restart, with a protected signing key and manual four-platform draft release workflow. Published-feed and self-update acceptance remain pending.


## Imported Google authorization recovery — 2026-09-05

- Verify imported OAuth configuration by reading it back before marking setup complete.
- First authorization opens browser consent directly; reconnect retains the saved-login refresh route. Distinguish client, login-store and browser-launch failures in five languages.
- macOS real-account smoke reached the Google callback and all five wizard steps completed; cross-platform and two-device acceptance remain open.


## Agent Memory OS adapter preview — 2026-09-05

- Add Agent Memory OS as a seventh selectable source with environment/custom-path discovery.
- Add bounded official JSONL v1–3 inspection and lossless snapshot capture/restore; test replica transfer plus encryption without touching a live memory store.
- Add five-language inspection UI and document private-export, deletion/ACL and trusted-import boundaries. Automatic export, Drive orchestration and live import remain gated.


## Browser authorization cancellation — 2026-09-05

- Cancel browser OAuth waiting without resetting setup; retry uses fresh state/PKCE. Five-language UI explains phases that cannot be cancelled.
- Bound callback header reads and explicitly configure accepted sockets, also fixing a macOS CI fixture race caused by inherited nonblocking mode.


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
