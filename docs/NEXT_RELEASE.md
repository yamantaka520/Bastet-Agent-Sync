# 🐈 0.7.0 completed release checklist

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
