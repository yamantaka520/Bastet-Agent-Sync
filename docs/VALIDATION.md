# Validation

## 0.4.2 language switching — 2026-09-06

- Root cause: the selector was disabled while the worker was running, and locale changes used the full-settings dirty/save path.
- The locale-only command validates the five supported values, atomically preserves persisted sync settings and updates tray labels. The UI waits for persistence, preserves other draft fields, and does not pause/restart synchronization.
- 71 Rust tests and 28 frontend tests passed; Clippy with warnings denied and frontend build passed. Regression tests cover all five locales during active sync, reload, failed persistence, initial incomplete setup, disconnected folders, invalid locale and corrupt-file preservation. One optional installed AMOS test was skipped.
- 0.4.2 is not released or installed over the running app. Native menu rendering and actual running-app language interaction remain to be checked.

## 0.4.1 published release — 2026-09-06

- [v0.4.1](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.4.1) is public/latest with 19 assets, built from tagged commit `1a53bef3d3d42382f32ee21038acbe3324c0fe8c`.
- [Release build 34035454608](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34035454608) passed all four architecture jobs and draft creation. The tag was pushed before dispatch; workflow permissions were unchanged.
- [Installer smoke 34035906240](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34035906240) passed Windows NSIS install/launch, macOS arm64 install/launch and Ubuntu/Fedora dependency installation checks. Intel macOS DMG integrity was checked locally; interactive Intel launch remains untested.
- All seven updater signatures were cryptographically verified; all 18 checksum entries and all 19 GitHub asset digests matched downloaded artifacts. Both macOS DMG integrity checks passed. Nine updater platform/format routes match their signed packages.
- All nine public installer/support links returned HTTP 200. The public latest update feed returned 0.4.1. Existing runtime provisioning and OS signing limitations from 0.4.0 still apply.
- These checks do not claim real repeated keychain prompt counts, an in-app upgrade between published versions, or complete two-device agent continuation. The existing running local app was not replaced.

## 0.4.1 sync display follow-up — 2026-09-06

- 69 Rust and 26 frontend tests passed (one optional installed AMOS test skipped). UI checks default collapsed groups, individual/all expansion, newest-first ordering, absent/invalid timestamp fallback, selected restoration, paused and unknown states and five-language completeness.
- Snapshot timestamps come from the local immutable object modification time, labelled as local save time; no original conversation timestamp is invented and no bundle schema/hash changes are introduced.
- Human-readable issue summaries retain raw codes inside technical details. Unknown source states are waiting, never implicitly successful.
- Native visual acceptance and replacement of the running older app remain outstanding; 0.4.1 was unreleased at this implementation check; publication is recorded above.

## 0.4.1 credential cache — 2026-09-06

- Local macOS: 69 Rust tests passed, one optional installed AMOS test skipped; 24 frontend tests passed; Clippy with warnings denied and frontend build passed.
- New cache checks cover 12 concurrent readers with one backend read, external change after clearing, missing/locked retry, account isolation, rotation, failed mutation eviction and deletion. UI checks explicit preparation without sync and removal of stale success after failure; all five locale key sets match.
- The initial sandbox run blocked five loopback HTTP/OAuth tests; rerunning with local listener permission passed all default tests.
- Optimized macOS executable and a separate staged 0.4.1 app were built successfully.
- Existing running 0.3.1 app was observed and not overwritten. Real keychain prompt counts across repeated sync, exit/relaunch and upgrades remain unverified. Developer ID signing is unchanged; 0.4.1 was not yet published at this implementation check.

## 0.4.0 release packaging — 2026-09-06

- [v0.4.0](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.4.0) is published with 19 assets. All 19 GitHub asset SHA256 digests matched local files; all seven updater signatures, 18 checksum entries and nine platform/format URLs were verified. All nine public installer/support download links returned HTTP 200, and the latest update feed returned version 0.4.0.

- All four architecture build jobs succeeded in [33982550173](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33982550173), from `e012d26`: macOS arm64/Intel DMG and signed app archives; Windows x64 NSIS/MSI with offline WebView2; Linux x64 DEB/RPM/AppImage. The overall run failed only when its token tried to create the release reference; an authorized Git push created the exact tag instead. No binary rebuild was needed for that permission issue.
- The tagged implementation passed [three-platform CI 33982550294](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33982550294). Three additional release checks reject a missing architecture or empty signature and verify direct download links, locales, checksum entries and installer-specific update routes.
- [Installer smoke 33983803827](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33983803827) passed: Windows NSIS install and native process launch; macOS arm64 DMG copy, ad-hoc signature verification and native launch; clean Ubuntu 22.04 and Fedora 43 containers install native packages through apt/dnf with no missing linked libraries. Intel macOS DMG integrity, package version and update signature were checked locally; Intel interactive launch was not tested.
- Windows installation ran on a hosted runner with its existing system components; the missing-WebView2 branch uses Tauri's embedded offline installer but was not separately exercised on a machine with WebView2 removed. NSIS supports all five installer languages. The Windows CRT is statically linked by target configuration. Linux package managers installed the missing dependencies in clean containers; desktop keyring unlocking and GUI/tray interaction remain separate checks.
- [Metadata review 33984074961](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33984074961) generated native DEB/RPM update signatures and nine platform/format entries so native packages do not receive an AppImage update. macOS remains ad-hoc signed, without Apple notarization; Windows has no Authenticode certificate. Updater signatures and checksums do not claim those OS trust identities.
- Initial Ubuntu Python lacked `tomllib`, so build runners now provision Python 3.12; Windows locale defaults exposed a cp1252 decoding error, fixed with explicit UTF-8. Source, updater metadata and installer integrity checks are distinct from full physical two-device agent continuation. Existing sync limitations below remain applicable.

## 0.3.2 capacity follow-up — 2026-09-05

- Optimized macOS build `e9e3268` and a separate staged `.app` bundle passed; bundle version and executable equality were checked. The running 0.3.1 bundle was not replaced. [0.3.2 CI 33971964621](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33971964621) is in progress at capture.
- Local validation: 66 default Rust tests, 23 frontend tests and one opt-in installed AMOS integration passed (90 total). Clippy with warnings denied, formatting, document/link checks and version parity passed.

- Optimized 0.3.1 passed the credential wait and reported **25 Claude conversations captured, 16 additional bundles uploaded, no size errors**. Claude Code reported the same 25 through the shared profile with zero duplicate uploads. This supersedes the pending observation below and resolves both original large Claude histories.
- Codex's first capture reached approximately 1,023 MiB: 363 of 373 discovered files had completed capture. The worker was asked to pause, preserving allocated upload IDs. This motivated a per-agent 2 GiB union and eliminating duplicate payload retention during exchange; the mixed-agent filtering regression passes alongside the existing encrypted retry/branch tests.
- The Mac locked during native follow-up. The UI tool explicitly requires a manual unlock, so no claim is made that the replacement build or all remaining sources completed their real Drive cycle. The user has been asked to unlock; independent build/test/documentation work continues.

## 0.3.1 segmented histories — 2026-09-05

- Final implementation [CI 33969430955](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33969430955) passed macOS, Windows and Linux. Local checks include 65 default Rust tests, 23 frontend tests and the opt-in installed AMOS integration.
- Optimized macOS 0.3.1 (`b335f2c`) is open with all eight saved selections. Start visibly reports the system-credential phase; its new large-history transfer has not yet been observed past that wait. The earlier 23 uploads belong to 0.3.0. No physical second-device acceptance or new large-session success is claimed.
- Segmented-history fixtures pass: missing parts prevent restore without creating its destination; complete parts reconstruct the original native files; repeated publishing reuses all IDs; each part survives authenticated encryption.
- 0.3.0 real macOS follow-up: system credential access resumed, and the Claude source reported **23 uploaded bundles** and **two size-limit failures**. It was safely paused before the remaining sources. The top-level counter initially remained zero when interrupted mid-cycle; 0.3.1 fixes that accounting and adds a separate credential-wait phase.
- Profiling located the slow debug pass in repeated snapshot hashing/serialization, not another credential wait. An optimized build is used for continued native testing. Earlier credential-wait and single-packet limits are retained below as historical observations, not the final state.

## 0.3.0 local conversation adapters — 2026-09-05

- 64 default Rust tests and 23 frontend tests pass. Tests cover all seven local selections, two isolated homes, automatic add-only receive, repeat stability, safe separate-folder recovery, conflict protection, malformed records, credential-path rejection, symlinks, SQLite committed WAL and revision-aware cache invalidation.
- The opt-in installed Agent Memory OS CLI test also passed against temporary homes (88 Rust/frontend checks including that integration).
- Installed Codex app-server `thread/read` discovered a synthetic rollout in an isolated CODEX_HOME and reconstructed its user message without a model request. This checks native read compatibility, not UI resume on a physical second computer.
- Installed Pi 0.85.0 SessionManager also read a temporary native v3 session and reconstructed its synthetic user message without a model request.
- Implementation [CI 33968308115](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33968308115) was running at capture; cross-platform success is not yet claimed.
- Clippy with warnings denied, formatting and TypeScript/Vite production build pass. Native macOS debug build succeeds. Final native launch/Drive observation is recorded below when available.
- Native macOS 0.3.0 (`944b29b`) was opened after pausing and fully quitting 0.2.1. Eight selections and the completed Drive wizard persisted. Start entered the worker, but a sampled stack stopped in `SecKeychainFindGenericPassword`; zero 0.3.0 transfer counters were observed at capture. This is an OS credential-access wait, not a successful multi-agent Drive validation. The app remains open for the local system prompt.
- Limits changed for compressed conversations: 32 MiB content, 64 MiB wire, 512 MiB replica/union. Tests use small synthetic histories; large-history performance and cross-platform interactive recovery remain separate gates.
- Scope is additive native conversation transport, not complete settings/skills, external attachments, project path mapping, Claude cloud/Cowork or cloud Work. Agy/Grok native continuation remains unverified. See [adapter boundaries](NATIVE_SESSIONS.md).

## 0.2.1 automatic AMOS worker — 2026-09-05

- 58 default Rust tests plus 20 frontend tests pass. One installed-CLI integration test is opt-in and separately passed against two isolated temporary memory homes (79 total executed checks). It verifies official export, backup, merge and repeat stability.
- Worker tests verify unsupported selections do not veto AMOS, failed apply retries, no repeat publication, and cancellation before transfer. This is not physical two-device acceptance.
- Installed CLI discovery was corrected to include the standard memory virtual environment. The source-checkout CLI on the development host was unusable; the active installed environment passed. AMOS normalizes unset link activation timestamps; fingerprint normalization prevents that from becoming a new edit.
- Native macOS 0.2.1 (`9d657c5`) launched with seven saved selections and a separate ChatGPT Work card. Start switched to a live worker and Pause; unsupported sources were skipped. First exchange was observed waiting inside macOS `SecKeychainFindGenericPassword`, with zero completed transfer counters. User-side keychain authorization is pending; successful real upload or second-device merge is **not claimed**. Previous 0.2.0 CI passed all three platforms; new run 33965425045 was in progress at capture.

- Follow-up after the user completed macOS keychain authorization: the native worker reported Google connected and a completed cycle with **1 uploaded encrypted AMOS bundle, 0 received and 0 merged**. The completion timestamp advanced from 20:22:56 to 20:24:03 (Asia/Taipei) while the upload count remained 1, confirming no duplicate publication across the observed cycles. The previous keychain blocker is resolved on this host. A physical second-device receive/merge is still unverified. [Implementation CI 33965425045](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33965425045) passed.

## 0.2.0 status and update interface — 2026-09-05

- **55 Rust + 20 frontend = 75 local tests passed**. Formatting, Clippy with warnings denied and TypeScript/Vite passed; version consistency is now checked with documentation.
- Native preflight prevents Drive completion from bypassing unavailable adapters. Frontend tests show blocking reasons, version/build identity and explicit update checks/install; an unpublished feed is not reported as up-to-date. Update artifact origin is restricted to this repository.
- Update installation/restart tests use frontend fixtures, not actual installer replacement. The signed draft pipeline and Windows/Linux/macOS self-update remain unverified until real artifacts run. No Agent sync worker was added or simulated.

- Native macOS 0.2.0 smoke: saved seven source selections survived restart; package version/build revision appeared below the logo. Start returned all seven unsupported adapter names. The update panel displayed no valid published feed. No transfer or installer replacement occurred. Blocking feedback is also shown beside Start so page position cannot hide it.

## Google browser authorization fix — 2026-09-05

- **53 Rust + 17 frontend = 70 local tests passed**; Clippy, formatting, TypeScript/Vite and macOS build/bundle passed. New tests require import readback, bypass stale refresh on first authorization, preserve reconnect, and reject credential errors without bypassing the store.
- Native macOS smoke: the old UI showed a credential-store error before browser launch. The fixed app was built, launched, and its authorization button invoked. Browser callback success was observed, then the app showed a connected account and all five setup steps complete after the user's interaction. No account IDs, folder IDs, authorization codes or keys are reproduced in this record.
- This confirms browser authorization and completed setup on one macOS host, not Windows/Linux keychain or two-device sync. The original OS-level credential error was not captured; an old running development image is a plausible contributing cause, not a proven root cause. Native tests were not repeated by rebuilding the running app.

## Agent Memory OS adapter — 2026-09-05

- **51 Rust + 17 frontend = 68 local tests passed**. Clippy with warnings denied, formatting, TypeScript/Vite and documentation checks passed. Fixture tests cover discovery/selection persistence, two-replica transfer, encryption and byte-preserving restore, unsupported formats/versions/records and size limits. UI tests cover metadata-only results, cancellation/failure cleanup and browser isolation.
- Only public synthetic AMOS fixtures were used; no live memory database, credentials or real memory export/import were accessed. Envelope inspection does not establish semantic import validity.
- Previous OAuth cancellation CI [33959106850](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33959106850) passed all three OS targets, closing the earlier macOS socket failure. This adapter's CI runs after push.
- Automatic Drive transport orchestration and trusted AMOS application remain unimplemented. [Adapter contract](AGENT_MEMORY_OS.md).

## Browser authorization cancellation — 2026-09-05

- 48 Rust + 15 frontend = **63 local tests passed** on the final code. Clippy with warnings denied, TypeScript/Vite build, formatting and documentation checks passed.
- Cancellation fixtures cover active/inactive waits, cancellation winning before code consumption, fresh retry state, listener closure and partial callback requests. Frontend verifies preserved setup and re-enabled Connect. No Google or native credential-store operations were used.
- Account-check CI [33958142867](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33958142867) failed on macOS: the test server immediately read an inherited nonblocking socket and received WouldBlock. Accepted HTTP fixture and OAuth sockets now explicitly switch to blocking mode; this is a code fix, not a retry-only workaround.
- Real Google login, three-platform native credential interaction and physical-device restore remain open.

## Account-aware setup — 2026-09-05

- **46 Rust + 14 frontend = 60 local tests passed**. Clippy with warnings denied, TypeScript/Vite build and formatting passed.
- New fixtures exercise the bounded Drive about endpoint, missing identity rejection, persisted account binding, wrong-account rejection without progress changes, display-name updates, legacy wizard loading and frontend status refresh after mismatch. No real Google account or native credential entry was used.
- Integrated ChatGPT/Codex scope was checked against current official documentation; local task adapters and cloud chat integrations remain unimplemented/unverified. Documentation scope is not a native restoration test.
- Previous wizard CI passed all three platforms in [run 33957403053](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33957403053). New account-check CI will run after push. macOS debug application built; no new native visual smoke is claimed.

## Resumable Drive setup — 2026-09-05

- **43 Rust + 13 frontend = 56 local tests passed**. Formatting, Clippy with warnings denied, TypeScript/Vite build and documentation checks passed.
- New backend tests reload each completed step, preserve manual mode, retry the same key/proof ID after interruption, require a recovery backup, validate a second replica's recovery kit before saving the key, reject invalid manual input and archive malformed progress on explicit restart.
- UI tests cover resumed steps, mode switching, restart confirmation, cancelled export, failed proof retry, manual prerequisite gates and stale diagnostic status.
- macOS debug App built and launched. Native smoke: OAuth file chooser opened and cancelled without advancing; manual mode exposed all settings, survived quit/reopen, and explicit restart returned to guided step 1. Traditional Chinese wizard layout visually checked. No OAuth configuration was imported, no keychain entry was created and no real Google account was contacted by this smoke.
- The previous M3 preview passed all three OS targets in [run 33955824629](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33955824629). This wizard revision's [run 33957403053](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33957403053) passed on macOS, Windows and Ubuntu (verified in the next account-check milestone).
- Real Google consent, native credential-store key/recovery operations on each OS, and physical two-computer setup remain unverified because no product OAuth client/account is configured. Step completion and interruption tests use isolated fixtures. [Setup contract](SETUP_WIZARD.md).

## M3 cloud preview — 2026-09-05

- Rust: **37 tests passed** (36 on Windows because the existing Unix symlink test is excluded); frontend: **11 tests passed**. HTTP fixtures bind only to loopback and need a test environment that permits local sockets.
- Crypto tests cover randomized authenticated encryption, recovery, wrong key/space, modified nonce/ciphertext and truncation. OAuth tests cover callback state/parameter rejection and the RFC S256 vector.
- Loopback HTTP fixtures verify encrypted multipart upload/download, pagination, incomplete listing, wrong parent, duplicate-ID responses and 401/403/429/503 errors. No production Google endpoint is contacted by these tests.
- Durable journals preserve allocated IDs after ambiguous folder creation and snapshot upload. Two encrypted fixture replicas preserve branches, avoid repeated publication, and refuse a wrong key or missing space proof before exchange.
- Frontend tests cover all five cloud dictionaries, browser isolation, unconfigured login, explicit connection/creation actions and clearing stale diagnostic success.
- TypeScript/Vite build, Rust formatting and Clippy with warnings denied were checked. macOS debug app built and launched; the native encryption/recovery button returned success with synthetic data, and Google login was visibly disabled because no product client is configured.
- This is a **partial M3 preview**, not the M3 acceptance gate. Native keychain round trips, actual browser consent/token exchange, real Drive transfer, Picker/shared-folder grants and two-physical-computer recovery are unverified. Space/key wizard and GUI queue orchestration remain open. [Precise contract](CLOUD_SECURITY.md).
- M3 [CI run 33955824629](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33955824629) is in progress at capture. The preceding M2 baseline-preservation patch passed all three OS targets in [run 33953930166](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33953930166).

## M2 snapshot core — 2026-09-05

- Local Rust: **24 tests passed** (6 foundation + 18 snapshot/transport tests; the Unix symlink case is not run on Windows).
- Frontend: **7 tests passed**, including invoking only the isolated native diagnostic and clearing stale success on a failed retry.
- TypeScript/Vite build, rustfmt, Clippy with warnings denied, and documentation checks passed.
- macOS debug `.app` built and launched. Pressing the native **Run isolated check** produced: 2 publication/reception operations, 2 preserved branches, 0 additional transfers on repeat, and 3 objects recovered on reopen. The UI explicitly labels this as a core check, not cloud sync.
- Test coverage includes child-before-parent arrival, partial/corrupt bundles and retries, checkpoint loss, stale conflict resolution, opposite-direction traffic prevention, Unicode text, oversized files, invalid paths, hash mismatch, wrong space, cross-stream parents, symlinks, no deletion propagation and exclusive replica locks.
- M1 Windows CI failed while decoding five-language Markdown with the host's default cp1252 encoding. `check_docs.py` now reads UTF-8 explicitly. The M2 baseline `387ab35` passed macOS, Windows and Ubuntu in [run 33953575895](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33953575895). An additional explicit-baseline test verifies that receiving a newer remote head does not silently rebase already-staged local edits; its updated [run 33953930166](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33953930166) was still in progress when this record was captured.
- No native agent profile, selected Drive folder, login or real conversation was used by the diagnostic. No power-loss or separate-physical-computer test is claimed. [Protocol and remaining gates](SNAPSHOT_PROTOCOL.md).

## Historical M1 desktop foundation

Verified locally on macOS, 2026-09-05. This is a development build, not a synchronization release.

## Automated checks

- `npm test`: 5 tests passed. Locale key parity, regional locale selection, browser preview isolation, native discovery/selection/save, corrupt-settings blocking, and visible save failure.
- `npm run build`: TypeScript and production Vite bundle passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --locked`: 6 tests passed. Atomic replacement/reload, corrupt-file preservation, invalid-save non-overwrite, candidate discovery, environment overrides and nested destination rejection.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings`: passed after simplifying the close handler.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: formatting enforced.
- `python3 scripts/check_docs.py`: localized guides, local Markdown links and public home-path checks passed.
- `npm run tauri build -- --debug --bundles app`: macOS `.app` built successfully with the generated calico icon.

## Native UI smoke

- Launched the bundled application; real native bootstrap discovered candidate folders for all six agents on the test computer.
- Switched through Traditional Chinese, Simplified Chinese, English, Japanese and Korean. Traditional Chinese and Japanese layouts visually inspected.
- Selected all detected agents, entered a device name and saved. Success state appeared.
- Opened the native folder chooser and cancelled; destination remained unset.
- Enabled close-to-tray and closed the window. The UI automation tool timed out while querying the hidden window; tray-menu reopen is **not verified**.
- Quit and reopened the app; saved Traditional Chinese locale, device name, six selections and tray preference were restored.
- The Start sync action remains disabled with an explicit explanation. No cloud login, upload, session import or real agent-store modification occurred.

## Pending gates and limitations

- Windows/Linux native CI is configured; its result is separate from local macOS evidence. Interactive tray visibility and reopen must be verified on each OS/desktop environment.
- Current discovery identifies candidate folders only. Executable/version detection, multiple profiles per agent and session counts are future adapter work.
- Local-folder selection does not prove Google Drive is connected or has transmitted data. The M2 local snapshot transport is implemented; OAuth, encryption, scheduling and native session restoration remain M3–M5.
- Native OS menus and the folder chooser follow OS localization; application labels and custom tray items use the selected language. Native menu localization and assistive-technology coverage need further QA.
- No signed release or auto-start installer has been published.
