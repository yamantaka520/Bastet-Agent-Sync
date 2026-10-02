# Validation

## Public product/privacy pages and production OAuth — 2026-10-02

- Published [product/support](https://bastet.tw/agent-sync/) and [privacy policy](https://bastet.tw/agent-sync/privacy/) pages from `website/agent-sync/`. Both provide English HTML without JavaScript and five selectable locales. Desktop/narrow layouts, all five policy-language switches, local links, JavaScript syntax and translation-key coverage were checked. Anonymous HTTPS requests return 200 with the actual product and policy text. The header and hero use an unchanged copy of the canonical `assets/calico.png`; the deployed PNG checksum matches the source and the live browser renders it. Deployment added only the new subdirectory; the existing homepage checksum was unchanged.
- Google Search Console automatically verified the site's ownership using the domain provider. The dedicated project's homepage, privacy URL and authorized domain were saved. With explicit owner approval, its External audience was switched from Testing to Production. The verification center reports that non-sensitive `drive.file` needs no data-access verification. Brand verification is separate: the automatic check reported inaccessible/insufficient pages and name/purpose discrepancies despite the anonymous-page evidence; an evidence-based additional-review request was submitted after explicit owner authorization. Google confirmed receipt and now shows branding under review. Its progress panel estimates an initial email in 3–5 days and up to 4–6 weeks for review; approval is not yet established.
- App Store Connect saved the policy and privacy-controls URLs in all five locales. Privacy labels remain an unpublished draft, and there is still no uploaded Apple build or App Review submission.
- The App now offers a five-language policy link. Native builds open only the fixed public policy URL through a dedicated command; an ad-hoc sandbox copy opened that URL in the system browser. The complete frontend suite passes **62 tests**; production build, formatting and documentation checks pass. The universal Store candidate was rebuilt with this link and its dedicated OAuth client; Apple validation again returned **VERIFY SUCCEEDED with no errors**. Production-profile/TestFlight and provider capture/restore acceptance remain separate.

## Dedicated Google OAuth sandbox acceptance — 2026-10-02

- The rebuilt universal Store `0.6.0` build `1` candidate includes the dedicated client and latest frontend changes. Apple server-side validation returned **VERIFY SUCCEEDED with no errors**. This does not upload the build or establish TestFlight/App Review acceptance.
- The new dedicated desktop client completed the real browser consent and loopback callback flow in an isolated ad-hoc signed App Sandbox copy. The UI reported Google connected and the expected test-account identity. No agent data was uploaded and no sync folder or worker was started.
- After quitting and reopening the sandbox App, saved wizard steps and identity remained. Explicit credential preparation succeeded; reloading Drive folders then verified Google connectivity without repeating consent. This demonstrates persisted credential readback and authorized API access in the ad-hoc sandbox, not production-profile/TestFlight acceptance or token-expiry refresh.
- The owner selected the product domain. Its current site is reachable; deployment-source discovery and publication of the product/privacy pages remain open. Existing OAuth projects and the independently distributed application were not changed.

## Mac App Store package validation — 2026-10-02

- The pushed preparation commit `63ac049` passed [Desktop foundation CI](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/36977314562) and [Cross-OS conversation handoff CI](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/36977314655). These results precede the package/privacy changes below.
- Apple server-side validation initially rejected private directory/file modes (90255). The rebuilt universal Store `0.6.0` build `1` package, with normalized permissions and final signing, returned **VERIFY SUCCEEDED with no errors**. Its embedded privacy manifest matches the source. Three packaging-fixture tests, shell syntax and diff checks pass. This is validation only: no upload, TestFlight processing or App Review submission.
- The age questionnaire is saved (global 4+, regional ratings vary); the two privacy categories are saved as an unpublished draft. The public policy URL, owner content-rights confirmation and applicable encryption classification/documents remain pending. An unsaved Apple encryption walkthrough with France distribution requested CCATS and French declaration documents; no declaration was submitted.
- A new ad-hoc sandbox copy accepted a synthetic Claude folder through the native picker and saved its local selection. A confirmed process restart retained that selection and detected the granted folder. This establishes ad-hoc bookmark persistence, not production-profile or provider capture/restore acceptance. A transient snapshot `sync_busy` resolved on retry to the expected missing-Drive-setup response; no Drive sync was started.
- Follow-up frontend changes add bounded retries only for `sync_busy` on read-only snapshot/operations views and five-language setup/busy guidance. The complete frontend suite passes **62 tests**, with production build and formatting checks passing. The subsequent OAuth-enabled universal candidate includes these frontend changes; its server validation is recorded separately below.
- The user chose Bastet-provided shared Google sign-in. Authenticated Google Cloud inspection found that the existing desktop client shares another application’s consent-screen brand and broad Gmail/Calendar/Drive scope configuration. Production audience is enabled, but verification is incomplete. After explicit owner authorization, a separate project was created with the existing billing account and Google Drive API was enabled. With explicit owner authorization, Google’s User Data Policy was accepted and a dedicated desktop OAuth client was created. Credentials were stored privately outside the repository. Only `drive.file` is configured; one owner test account is allowlisted. The universal Store candidate embeds that client. Public/reviewer sign-in remains pending because the audience is still External/Testing and product-domain/privacy setup is incomplete.

## Mac App Store preparation — local acceptance, 2026-10-02

- Added an opt-in `mac-app-store` feature with direct updater/plugin commands excluded and five-language Store UI. The user's first-Store-release decision excludes Agent Memory OS in both backend and UI; the default direct-download channel retains its existing updater and AMOS behavior. Store credentials use a separate Keychain service.
- Implemented security-scoped folder-picker bookmarks, scoped reads through worker/restore paths, and same-path managed-handoff reauthorization. Native Store-profile restart persistence, moved/revoked grant behavior and real provider acceptance remain pending.
- Default Rust suite: **123 passed, 7 ignored**. Store-feature Rust suite: **132 passed, 6 ignored**. Frontend: **57 passed**, production build passed. Strict Clippy for both feature configurations and formatting passed. Two packaging-fixture tests passed; 28 Markdown documents passed the documentation check.
- Built and verified the universal Store-signed `.app` and `.pkg` using marketing version `0.6.0`, build `1`. The production Store profile cannot be locally launched by design; TestFlight or a development profile is required. A separate ad-hoc sandbox test copy launched after correcting the test copy's executable bit. Its isolated core UI check passed: published/received 2, retained branches 2, repeated transfer 0, rebuilt objects 3. This is not production-profile, Store Keychain, or Drive acceptance.
- Four frontend regression cases cover transient wizard-lock recovery, restored folder input/options, and preservation of genuine startup/action errors and unsaved edits. The native UI exposed a stale initial error; lock contention is inferred from the competing startup reads rather than a captured IPC trace.
- App Store Connect draft state: names/subtitles, five localized descriptions and keywords, support/marketing URLs, free price, 175 eligible territories, `2026 Bastet AI` copyright, private review contact, manual release and user's non-trader DSA declaration saved. Apple's page reports regulatory requirements completed. No upload, TestFlight build, App Review submission or publication has occurred. Public 0.6.0 assets were not changed.
- Still pending: production-profile/TestFlight acceptance, native bookmark restart/revoke/provider tests, privacy policy publication, privacy manifest and API reasons, encryption/age/content answers, reviewer setup and screenshots. See [App Store delivery](APP_STORE.md).

## 0.6.0 published release — 2026-10-02

- [v0.6.0](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.6.0) is public/latest with 19 assets and five-language notes. Tagged source: `21e280de0588c4a0a17cd1b57c57e05482788a37`.
- [Three-platform CI](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/36899499148), [four architecture builds](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/36899566789), and [independent installer checks](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/36901139961) passed on that exact source. Windows NSIS installation and MSI extraction each passed version, x64 PE and native launch checks; their binaries are intentionally not required to have identical hashes because Tauri embeds format-specific bundle markers. macOS arm64 DMG installation/launch and Ubuntu/Fedora package dependency checks passed.
- [Cross-OS handoff run](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/36899499226) passed all 15 jobs: three producers, all six directed receiving combinations and all six returns to the original OS. Official Codex, Claude and Pi readers ran on each receiving runner; Grok remained optional and Agy used SQLite integrity checks. One Linux prerequisite job was retried after approximately 15 minutes downloading Ubuntu mirror packages; the same-source retry and all dependent returns passed. These are hosted synthetic exchanges, separate from the actual model-turn evidence below.
- Both macOS architectures passed Developer ID signing, Apple notarization and Gatekeeper acceptance. Downloaded DMGs and updater apps independently passed signature, stapled-ticket, integrity, version and architecture checks. Windows Authenticode remains explicitly deferred; this release does not claim Windows publisher trust.
- All seven updater signatures and their trusted comments, 19 GitHub digests, 18 SHA256SUMS entries and nine updater routes matched. The six Mac package/support files exactly matched the independently verified workflow artifacts. All nine public installer/support links returned HTTP 200, and the actual public latest update feed exactly matched the verified 0.6.0 artifact.
- Physical two-computer acceptance, Intel Mac native GUI launch, Linux graphical interaction and an instrumented in-app upgrade to 0.6.0 remain outside these checks. The running user application and active agent stores were not replaced. Agy's stored absolute workspace paths remain a recovery limitation.

## 0.6.0 real Drive and model continuation — delivery verification, 2026-10-02

- An explicitly enabled test used the saved Google authorization to create a separate test folder, space key and proof. Three synthetic Claude sessions (196,932 source bytes) completed initial upload, persistent-cache population, unchanged warm sync, a one-session append and restore into a separate receiving profile. The active binding and user histories were untouched. Exact folder identity was checked before cleanup; Drive confirmed it was trashed.
- On macOS arm64 in a **debug test build**, initial upload took 12.39 s (3 objects; 337,372 estimated encrypted payload bytes), cache population 10.90 s, unchanged warm sync 4.42 s (zero uploads; one fresh proof download), append 8.19 s (one object; 112,825 estimated encrypted payload bytes), and receiver download/restore 46.42 s (4 version bundles; 3 restored sessions). Setup took 31.01 s and cleanup 1.20 s. Payload estimates exclude HTTP headers and multipart metadata; this is neither a release-app speed claim nor a physical two-device test.
- Actual installed Pi, Claude Code, Codex and Grok model turns resumed isolated restored profiles, retained their synthetic A/B history, executed a tool and wrote a provider-specific marker. A separate encrypted return test published each continued conversation as a child of the received head and restored it into a new A-side profile, checking the same session identity, A/B/live markers and tool result. These model turns ran on one Mac; they do not establish model execution on every CI operating system. Grok required correcting synthetic summary timestamps to the native ISO format; its installed session listing then discovered the restored session before the successful live turn. Agy separately passed production SQLite capture, encrypted-bundle reopen and fresh-profile restore: the official CLI resumed the same conversation, recalled its original marker and executed a file tool; repeating capture/restore of the updated database retained both markers on another resume. Its native database retained the original absolute workspace path, including a trial that wrote to the original temporary project despite a different launch cwd. This establishes explicit database recovery on one Mac, not cross-OS path remapping or managed-profile continuation. Temporary authentication files were removed after each invocation.
- The separately measured 200-session fixture includes 21,450,872 source bytes and verifies unchanged sync, append, pre-upload network failure, uncertain committed-upload retry, restart and receipt. With the persistent object cache populated, an unchanged cycle uploaded nothing and downloaded only the fresh proof; failure retries did not duplicate publication. The optimized Rust test binary completed the workload in 35.74 s: initial upload 8.07 s (200 objects; 36,557,210 encrypted bytes), cache population 3.86 s, unchanged warm sync 1.73 s (one 623-byte proof), append to the 8 MiB history 2.40 s (one 14,197,668-byte object), and receiver restore 10.42 s (203 version bundles; 200 sessions). Peak resident memory was 270,745,600 bytes. This uses an in-memory remote under the production cache wrapper; it is not Drive throughput or an installed-app benchmark. Final platform/release results are recorded above. An initial tagged build was stopped before publication after Windows Clippy found an unnecessary mutable vector in the new opt-in test; its platform-specific temporary-root list was corrected without changing production behavior.

- Integrated local verification after the new opt-in harnesses: **123 Rust tests passed, 7 explicit optional tests ignored**. A sandbox-only attempt blocked localhost fixtures; rerunning with localhost access passed. The live-return, optimized-volume and real-Drive tests above were invoked explicitly and are not inferred from ignored test counts.

## 0.6.0 sync performance — local acceptance, 2026-10-02

- Integrated Rust suite: **123 passed, 3 explicit optional tests ignored**. Strict Clippy, formatting, diff checks and 25 Markdown documents passed. The explicit synthetic produce → continue → verify handoff also passed after these changes. Frontend code was unchanged by this performance pass; its preceding 49-test result is recorded below.
- Native capture counters: Claude Code, Grok and Agy each compressed once on initial publication, zero additional times on an unchanged second cycle, and once more after a child-file, companion-file or committed WAL change. Changed content retained its causal parent. Unchanged captures left the journal unchanged; missing root or segment baselines cannot take the cache shortcut. Full stable capture still occurs; this is not an mtime-only shortcut.
- Cloud fixture counters: eight overlapping listing requests produced one remote listing. A later request refreshed the revision; a concurrent upload invalidated the stale listing and forced a retry. Uploading one object retained unrelated cached downloads. Existing revision, restart-cache and fresh proof tests passed.
- Progress fixture: 10,000 routine updates inside one reporting interval produced no additional callback after the initial stage callback; completion emitted the exact count. Scope exit flushed partial byte counts. Routine callbacks are limited to 100 ms intervals; stage/body/completion events remain immediate.
- At this initial local acceptance stage, no real-account throughput, latency or CPU percentage improvement was claimed. The user subsequently authorized the four delivery steps; the measured real-Drive and published-release evidence above supersedes the earlier unreleased status. No active agent store or installed application was replaced.

## 0.6.0 cross-OS handoff — local acceptance, 2026-10-01

- Local focused fixtures cover project mapping, preserved historical text, managed-profile continuation, causal ancestry, pristine-profile no-loop behavior and portable filenames. The frontend adds mapping, branch details and scoped continuation commands in all five locales.
- Integrated local checks passed: 117 Rust tests (3 explicit optional tests ignored), 49 frontend tests, TypeScript/Vite build, strict Clippy, formatting, actionlint, 25 Markdown documents and 3 release-asset tests. The explicit synthetic produce → continue → return fixture passed separately.
- Official installed readers passed on restored synthetic profiles: Codex 0.159.2 `thread/read`, Claude Agent SDK 0.3.276 session/message readers, Pi 0.85.1 SessionManager and Grok 1.0.13 transcript export. Agy passed SQLite integrity only. These checks did not call a model.
- Initial [cross-OS run 36893791667](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/36893791667) and [three-platform CI 36893791808](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/36893791808) were triggered for `ac835d1`; platform-specific temporary-directory and fixture assertions were subsequently corrected. The final passing runs are recorded above. Six directed source/target combinations each require a return to the original OS; Codex, Claude and Pi readers are mandatory on each receiving runner. Grok remains optional in CI. A defined matrix is not evidence that its jobs passed.
- Actual model continuation and Agy recovery results are now recorded above. Complete external dependencies and portable Agy workspace remapping remain separate gates. See [contract](CROSS_OS_HANDOFF.md). No active user agent store has been imported into or overwritten.

## macOS Developer ID signing — integration, 2026-10-01

- The user switched scope to Apple signing/notarization and deferred Windows signing. The supplied Developer ID certificate matches its private key. A local probe signed successfully with an Apple-rooted chain, secure timestamp, expected team and hardened runtime; strict codesign verification passed. Temporary keychain state was restored afterward.
- Apple notarization API authentication passed; an empty submission history is not evidence of a notarized app. Repository credentials were installed only after explicit authorization. No credential values were added to source.
- [Initial run 36878828250](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/36878828250) authenticated and signed its probes, but the runtime-flag parser incorrectly expected a separate flags line. Commit `32dcd09` reads the actual CodeDirectory line; the corrected expression was verified against the real local signed probe.
- [Run 36879098228](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/36879098228) passed signing preflight, frontend/Rust tests, documentation and release-asset checks for both targets, then compiled and submitted both apps to Apple. Both app submissions, Intel `36a46a1e-0061-4bfd-b97b-c1ddb219843f` and Apple Silicon `44f50a60-4a0a-4a0c-8170-8f3a271e2071`, are now **Accepted**. Apple Silicon DMG submission `604fbb3c` is also Accepted. The Apple Silicon job completed; Intel lost network connectivity while polling Apple and uploaded no artifact.
- Static actionlint, shell syntax, 24 Markdown files and 3 release-asset tests passed. The downloaded Apple Silicon app, DMG and updater app passed independent signature, stapled-ticket and Gatekeeper checks with macOS signing services available; the updater signature was verified against the repository public key. An initial sandbox-restricted local check failed to access those services and is not evidence of damaged bytes. Intel was subsequently rebuilt and verified in the 0.6.0 release above. Existing older public release assets were not replaced. See [macOS signing contract](MACOS_SIGNING.md).

## Windows publisher signing — signing succeeds, trust blocked, 2026-09-09

- Static workflow validation with actionlint, 23 Markdown files and 3 release-asset tests passed. The workflow requires valid Authenticode signatures and timestamps for package and installed files and rejects unsigned input.
- [First run 34318563228](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34318563228) failed Azure login with `AADSTS700213`. After correcting the exact immutable OIDC subject, [run 34319166948](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34319166948) passed Azure login, frontend/native tests and optimized Windows compilation, but failed at its signing callback. No package was uploaded.
- A small signing probe now runs before expensive compilation. [Run 34320448557](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34320448557) confirmed Artifact Signing returned `Succeeded`; Windows signature verification returned `UnknownError`. [Detailed diagnostic run 34320673359](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34320673359) identified an untrusted certificate root, with additional offline/unknown revocation diagnostics. Thus Azure authentication and signing permission work, but publicly trusted signature acceptance is not established.
- [Read-only management inspection 34322764027](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34322764027) passed after account-scoped management access was granted. Azure returned two active profiles: `PrivateTrust` and `PrivateTrustCIPolicy`; the configured signing profile uses the latter. No Public Trust profile was returned. A publicly trusted profile must pass the same check before packaging proceeds. No trust roots were installed and no verification was relaxed. Current public 0.5.1 assets are unchanged; installer signature, installed-binary and launch acceptance for this workflow remain pending.
- [Setup and packaging contract](WINDOWS_SIGNING.md). Apple signing/notarization was subsequently authorized; its separate status is recorded above.

## 0.5.1 published release — 2026-09-09

- [v0.5.1](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.5.1) is public/latest with 19 assets and five-language notes. Tagged source: `dcc40c46f9358bf2be7b5443fa17a4825db07d15`.
- [Four architecture builds, run 34308068921](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34308068921), [independent three-platform CI, run 34308065357](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34308065357), and [installer checks, run 34308625229](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34308625229) all passed.
- Installer checks cover Windows installation/native launch, macOS arm64 DMG/ad-hoc signature/native launch, Ubuntu DEB dependencies and Fedora RPM dependencies. Linux graphical interaction and Intel Mac native launch are outside these checks.
- All seven updater signatures, 19 GitHub digests, 18 SHA256SUMS entries and nine updater routes matched. Both macOS DMGs passed integrity checks; archived version and architecture matched. Nine public installer/support links returned HTTP 200, and the public latest update feed matched the verified artifact.
- Isolated mixed-space fixtures demonstrate the fix; physical two-computer acceptance and any individual device failure cause remain unverified. No user app or active agent store was replaced. Apple notarization and Windows Authenticode remain excluded.

## 0.5.1 encrypted-space fix — local acceptance, 2026-09-09

- Mixed-space encrypted fixtures now preserve own-space transfer, skip foreign objects with a partial warning, and remain idempotent on repeat. The configured proof remains mandatory, including for device-report refreshes. Malformed public envelopes, wrong keys for the configured space and unsupported versions remain fatal. Foreign ciphertext cannot be authenticated without its key and is never imported. New-space setup refuses occupied session-object folders before key/proof allocation.
- Local validation: 103 Rust tests passed, 2 optional installed-CLI tests ignored; 43 frontend tests passed. TypeScript/Vite build, Clippy with warnings denied, formatting, documentation and release-asset checks passed.
- These are isolated fixtures, including loopback HTTP tests. They do not establish the cause of any particular Windows failure or real two-computer success. Device foreign-object warnings describe the latest attempted refresh; cached refreshes are throttled for 60 seconds.
- Release and installer evidence is recorded above. No active agent store, cloud history or running user app was replaced.

## 0.5.0 published release — 2026-09-07

- [v0.5.0](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.5.0) is public/latest with 19 assets and five-language notes containing direct installer links. Tagged source: `297d8c9cbf552effb2721a3facf89ae6dbb22b9c`.
- [Four architecture builds and draft publication, run 34046845932](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34046845932) passed. [Independent three-platform CI, run 34046836761](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34046836761) also passed.
- [Installer checks, run 34047325944](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34047325944): Windows installation/native launch, macOS arm64 DMG/ad-hoc signature/native launch, minimal Ubuntu DEB dependencies and Fedora RPM dependencies all passed. Linux graphical interaction and Intel Mac native launch are not included in these checks.
- All seven updater signatures were verified using the repository public key. All 19 GitHub asset digests and 18 SHA256SUMS entries matched; all nine updater platform/format routes matched the signed artifacts. Both macOS DMGs passed hdiutil verification; archived app version and Mach-O architecture matched arm64/x64.
- Nine public installer/support links returned HTTP 200; the public latest update feed exactly matched the verified 0.5.0 artifact. Existing running user app was not replaced.
- Remaining acceptance boundaries are unchanged: browser visual QA was blocked by unavailable admin-policy verification; macOS in-app upgrade success is user-reported; installed Agy/Grok local checks do not establish physical two-device or restored-profile Agy model continuation. Apple notarization and Windows Authenticode were explicitly excluded.


## 0.5.0 control center — local acceptance, 2026-09-07

- New regression coverage includes configured concurrency, timed dispatch pause, aggregate bandwidth/cancellation, progress isolation and ETA, history retention/corruption, identity persistence, safe cache cleanup, reviewed fingerprints, path/symlink rejection, portable sanitization/selection, encrypted idempotent preferences exchange and MIME/stream guards. Local results: 97 Rust tests passed, 2 optional installed-CLI tests excluded from the default suite; the new Agy/Grok installed fixture test was separately run and passed. 39 frontend tests, TypeScript/Vite build, Clippy with warnings denied, documentation checks and 3 release-asset tests passed. CI/package evidence follows after release verification.
- Installed Grok exported both synthetic ACP sentinel messages after Bastet capture, compression, encryption/decryption and isolated-folder restore. Agy's dedicated original test conversation was created and resumed with the installed logged-in CLI; the second invocation repeated its prior marker. Bastet's restored snapshot passed SQLite integrity and nonempty-step checks. No private user conversation was used. Restored-profile Agy model continuation and physical second-device transfer are not claimed.
- macOS in-app upgrade success is a user statement. Apple notarization and Windows Authenticode are excluded by authorization.
- Browser visual QA was blocked: the tool could not verify an admin-enforced browser policy. No browser-control workaround or new visual acceptance is claimed. Existing running app was not overwritten.
- [Exact behavior and limitations](SYNC_CONTROL.md). This entry supersedes the earlier unreleased scheduling status; dated historical evidence below is retained.


## Bounded parallel synchronization — unreleased, 2026-09-06

- 79 native tests passed; one installed-AMOS opt-in test ignored. 32 frontend tests passed. Clippy with warnings denied, optimized native build and production frontend build passed.
- Scheduler fixtures demonstrate exactly three concurrent groups, dispatch stopping on pause, all active threads joining before return, and panic-to-error handling. Planner fixtures cover aliases, canonical journals and overlapping paths.
- Isolated source integration uploads Codex, Pi and AMOS concurrently with a failing source, deduplicates Codex/Work counts, restores into a second temporary device store and does not repeat unchanged uploads/restores. Pre-paused work never starts.
- Eight concurrent cache readers share one same-revision download; explicit proof requests bypass the cache. Existing revision-change and restart-cache tests still pass. Five-language queued labels and paused fallback are verified.
- Existing seven-source native format, encryption, conflict and loopback HTTP tests still pass. This is fixture/local build evidence, not a Google account throughput benchmark or native visual acceptance. Public v0.4.3 remains unchanged and the running app was not replaced.

## 0.4.3 published release — 2026-09-06

- [v0.4.3](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.4.3) is public/latest with 19 assets. Tagged source: `af213fb82fee1d5e6d299f6ae7db3e8cce961031`.
- [Release build 34039070119](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34039070119) passed all four architecture builds and draft creation, including 31 frontend tests and native tests on each build host.
- [Installer smoke 34039591327](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34039591327) passed macOS arm64 and Windows NSIS install/launch, plus Ubuntu/Fedora package dependency checks.
- Seven updater signatures cryptographically verified; 18 checksum entries and 19 GitHub digests match downloaded artifacts. Both macOS DMGs pass integrity checks. All nine updater platform/format routes match signed artifacts.
- Nine public installer/support URLs returned HTTP 200. The public latest.json exactly matches the verified 0.4.3 manifest.
- Five-language guides and release notes describe compact status and payload traffic scope. Live-account traffic display, native visual acceptance, Intel GUI launch, in-app upgrade and full physical two-device continuation remain separate checks. The running local app was not replaced. macOS remains ad-hoc signed/unnotarized and Windows lacks Authenticode.

## 0.4.2 published release — 2026-09-06

- [v0.4.2](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.4.2) is public/latest with 19 assets. Tagged source: `7865eed3bb60365f3d8be3e2e121624dcb17f9aa`.
- [Release build 34037416517](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34037416517) passed all four architecture builds and draft creation; workflow permissions were unchanged. [Language fix CI 34037163236](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34037163236) also passed.
- [Installer smoke 34037855791](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/34037855791) passed macOS arm64 install/launch, Windows NSIS install/launch, and Ubuntu/Fedora native package dependency checks.
- All seven updater signatures were cryptographically verified; all 18 checksum entries and all 19 GitHub digests matched downloaded assets. Both macOS DMGs passed integrity checks. All nine updater platform/format routes match their signed artifacts.
- Nine public installer/support links returned HTTP 200; the actual latest.json feed returned 0.4.2.
- Native interactive language/tray switching, repeated keychain prompt counts, in-app upgrade between published versions, Intel GUI launch and full physical two-device continuation remain separate acceptance checks. The running local app was not replaced. Existing macOS ad-hoc and Windows unsigned OS trust limitations remain.

## 0.4.2 documentation review — 2026-09-06

All 20 tracked Markdown documents were reviewed for this release. README, requirements, master plan, five locale guides, release notes, changelog, validation, native/AMOS adapters, cloud security, setup, snapshot protocol, update/status and the ADR were reconciled with current code. AGENTS.md and asset provenance remain applicable and retain their original instructions/provenance. Historical validation entries and dated requirements are preserved as history, not current acceptance claims.

Corrections include active Start/Pause and local adapters, separate diagnostic local-folder scope, actual macOS OAuth evidence, account identity checks, cached client readback, current retry boundaries and the language-only save exception during sync. Outstanding physical two-device, interactive OS credential, native resume, OS signing and in-app upgrade checks remain explicit.

## 0.4.2 language switching — 2026-09-06

- Root cause: the selector was disabled while the worker was running, and locale changes used the full-settings dirty/save path.
- The locale-only command validates the five supported values, atomically preserves persisted sync settings and updates tray labels. The UI waits for persistence, preserves other draft fields, and does not pause/restart synchronization.
- 71 Rust tests and 28 frontend tests passed; Clippy with warnings denied and frontend build passed. Regression tests cover all five locales during active sync, reload, failed persistence, initial incomplete setup, disconnected folders, invalid locale and corrupt-file preservation. One optional installed AMOS test was skipped.
- At the implementation check, 0.4.2 was not yet released; publication is recorded above. It was not installed over the running app. Native menu rendering and actual running-app language interaction remain to be checked.

## 0.4.1 published release — 2026-09-06

- [v0.4.1](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.4.1) was published as latest with 19 assets, built from tagged commit `1a53bef3d3d42382f32ee21038acbe3324c0fe8c`.
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

## 2026-09-06 — compact status and Drive traffic (unreleased)

- Rust: 73 tests passed, one installed-AMOS opt-in test ignored. New tests cover independent byte directions, partial reads/EOF, bounded rolling buckets, idle zero rates and retained totals. Existing encrypted Drive HTTP upload/download fixtures still pass with sized streaming request bodies.
- Frontend: 31 tests passed, including all five traffic locales, rates/totals, missing samples and runtime-row placement below Drive setup. Production frontend build, optimized native build and Rust Clippy with warnings denied passed.
- Google account traffic and desktop visual acceptance for this change have not been tested. Existing running app and public v0.4.2 installers are unchanged.
