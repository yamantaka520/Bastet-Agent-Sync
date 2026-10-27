# 🐈 Release scope and completed checklists

## 0.8.2 Google Drive cache security patch

Authorized and published 2026-10-27: [v0.8.2](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.8.2) is public/latest, release ID `425077750`, with 19 assets at source `064dec54350c8fef52bb7cb73c7882ff9135b0f0`. Checksums, API digests, all seven updater signatures and nine routes passed; see [validation](VALIDATION.md). The installed app and App Store were not changed.

- [x] Bound cache bytes and reference count before writes across concurrent cache instances.
- [x] Deduplicate validated bundle content and clean expired, orphan and legacy cache data.
- [x] Complete exploit regression and legitimate restart/revision/proof behavior tests.
- [x] Complete default/Store Rust checks, frontend checks and independent candidate review.
- [x] Build all four architecture targets, verify signatures and macOS notarization, then publish immutable assets and read back the update feed.

## 0.8.1 multipart readiness honesty repair

Patch release on top of 0.8.0, published 2026-10-11 as [v0.8.1](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.8.1) (public/latest, 19 assets, tagged source `a8d40873662ddf44d07a2dfc658d1764bb08ad49`). The running installed application is not replaced.

- [x] Replace presence-only multipart `ready` with `complete` (present + index consistency) and label it "parts received — integrity will be verified before restore" in five locales; never "ready"/"verified".
- [x] Accept part presence only from a well-formed `PortablePart`, so a forged/truncated bundle embedding a transfer id as a substring cannot inflate presence.
- [x] Align the native crash-window profile enumerator to `MAX_MANIFEST_FILES` instead of a fixed 2048.
- [x] Regression tests (RED then GREEN), frontend honesty test, both feature sets and five-language notes.
- [x] Publish v0.8.1 (four installer targets, macOS Developer ID signing/notarization, updater signatures, public readback) and record immutable evidence.

A buffered, not streaming, transport and an in-memory mock relay mean fixture coverage is not real cloud delivery. The pre-existing Store-parallel `sync_busy` flake is unchanged. Physical two-device delivery, live model/provider continuation, Windows/Linux runtime acceptance and signed Store sandbox acceptance remain open.

## 0.8.0 completed release checklist

Authorized and published 2026-10-11: [v0.8.0](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.8.0) is public/latest with 19 assets and five-language notes; tagged source `6d8bf1f0072a1d79773ed44dd2ee0e27b3491021`. Four installer targets and macOS Developer ID signing/notarization passed; all 7 updater signatures were cryptographically verified; Windows publisher signing remains explicitly deferred. Two real Windows-only Hermes test defects found by CI were fixed before publication. Exact evidence is recorded in [validation](VALIDATION.md).

- Include the [Hermes continuation implementation](HERMES_AGENT.md) together with **three mutually exclusive sync relays**: Google Drive API (default), iCloud Drive folder, or OneDrive folder.
- OneDrive remains a selectable, supported folder transport, not a legacy-only option. The user's final three-choice decision supersedes the briefly considered two-choice picker.
- iCloud and OneDrive use the existing encrypted folder worker and installed provider sync clients; direct Apple/Microsoft OAuth is not implied. Keep existing provider configuration and recovery kits; pause before switching, save setup, then explicitly Start. Switching does not migrate old history or enable simultaneous mirroring.
- Local folder handoff tests do not prove physical-device cloud delivery. Follow [validation](VALIDATION.md) for exact executed checks and remaining gates. This scope record does not claim publication, installation replacement or completed release packaging.

## 0.7.0 completed release checklist

Authorized 2026-10-03: offer one selected destination from Google Drive API, iCloud Drive folder, and OneDrive folder, plus an actionable credentials/recovery entry. The folder modes use installed provider desktop clients and the encrypted worker. The historical synthetic folder diagnostic remains separate.

- [x] Implement destination selection, provider-specific create/join, recovery-kit export, encrypted local object exchange and five-language UI/guides.
- [x] Integration tests and three-platform CI passed; exact results are recorded in [validation](VALIDATION.md).
- [x] Installer artifacts, macOS Developer ID signing/notarization and updater signatures verified.
- [x] Publish v0.7.0 and record immutable release evidence and notebook records.

A completed folder cycle proves only local handoff. iCloud/OneDrive cloud delivery and a physical two-device run have not been verified. Repeated macOS Keychain prompts after “Always Allow” remain unresolved. Direct Microsoft Graph sign-in is outside this release.

## 0.6.0 completed checklist

Authorized 2026-10-02: complete cross-platform CI and six directed round trips, actual model/tool continuation and return, measured performance/recovery, then verified installer publication. macOS requires Developer ID signing and notarization. Windows publisher signing remains explicitly deferred; updater signatures remain required.

- [x] Isolated managed profiles, project mappings, preserved branches and five locales.
- [x] Actual Codex, Claude Code, Pi and Grok model/tool continuation with encrypted return to a fresh receiving profile.
- [x] Optimized 200-session fixture, restart/recovery checks and isolated real-Drive measurement with cleanup.
- [x] Complete three-platform CI and all six OS-to-OS return checks.
- [x] Complete the Agy restored-profile investigation: explicit native database recovery/resume passes; stored absolute workspace paths are not remapped.
- [x] Verify four installer targets, signatures, macOS notarization and installer smoke checks.
- [x] Publish and record the final delivery evidence in BastetMind.

Published 2026-10-02: [v0.6.0](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.6.0). Measured results and limitations are recorded in [validation](VALIDATION.md). Agy is currently database recovery only. The installed user application and active agent stores are not acceptance-test targets.

## 0.5.0 completed checklist

Authorized 2026-09-06: finish the following features, update five-language documentation and BastetMind, then publish all installer targets. Apple notarization and Windows Authenticode are explicitly excluded. The user reports macOS in-app upgrade succeeded. Agy/Grok acceptance will use installed CLIs with isolated local profiles; a second physical computer is not required for this milestone.

- [x] Per-source stages, item counts and current payload-byte progress.
- [x] Persistent cycle history and encrypted device reports with observed timestamps.
- [x] Conflict comparison and explicit keep-both recovery.
- [x] Configurable concurrency, upload/download limits, allowed hours and timed pause.
- [x] Local/cache/Drive usage and safe cache cleanup.
- [x] Opt-in portable settings and skills, preview, credential exclusion and receiving-side review.
- [x] Installed Agy/Grok local-profile compatibility checks and truthful continuation actions.
- [x] Five-language UI/docs and regression tests.
- [x] Three-platform packages, installer checks and public release.

Published: [v0.5.0](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/tag/v0.5.0). Exact acceptance and exclusions: [validation](VALIDATION.md).

A device's last report is not proof it is online. Payload bytes are not system-wide traffic. Existing agent files are never silently overwritten. Remote history deletion is excluded until references and offline-device retention can be proven safe; this milestone delivers local cache cleanup.
