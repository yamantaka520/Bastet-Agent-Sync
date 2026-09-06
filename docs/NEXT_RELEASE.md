# 🐈 0.5.0 release checklist

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
