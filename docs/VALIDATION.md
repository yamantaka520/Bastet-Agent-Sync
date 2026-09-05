# Validation

## 0.2.1 automatic AMOS worker — 2026-09-05

- 58 default Rust tests plus 20 frontend tests pass. One installed-CLI integration test is opt-in and separately passed against two isolated temporary memory homes (79 total executed checks). It verifies official export, backup, merge and repeat stability.
- Worker tests verify unsupported selections do not veto AMOS, failed apply retries, no repeat publication, and cancellation before transfer. This is not physical two-device acceptance.
- Installed CLI discovery was corrected to include the standard memory virtual environment. The source-checkout CLI on the development host was unusable; the active installed environment passed. AMOS normalizes unset link activation timestamps; fingerprint normalization prevents that from becoming a new edit.
- Native Google upload and interactive cross-platform behavior are recorded separately after smoke testing.

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
