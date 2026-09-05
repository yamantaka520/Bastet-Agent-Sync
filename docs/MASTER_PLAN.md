# 🐈 Bastet Agent Sync — Master plan

Status: M2 snapshot core implemented; M3 OAuth/encryption preview and resumable guided/manual setup implemented with offline tests. Real-account and two-device gates remain open; native agent transport remains gated. This is the implementation roadmap; REQUIREMENTS.md preserves the detailed product requirements. User-facing plan and guide: [English](manual/en/guide.md) · [繁體中文](manual/zh-Hant/guide.md) · [简体中文](manual/zh-Hans/guide.md) · [日本語](manual/ja/guide.md) · [한국어](manual/ko/guide.md).

## Product

A local-first desktop companion that synchronizes selected agent settings, skills, memory and conversations across computers through a shared Google Drive folder. macOS, Windows and Linux are target platforms. Native session restoration is a required adapter goal; context-based continuation must be labelled as a separate fallback.

## Architecture

Tauri 2 + React/TypeScript for the desktop UI; Rust owns local discovery, configuration and later the synchronization engine. The renderer has a narrow command interface, not arbitrary filesystem or shell access. One desktop process owns the lifecycle; closing to tray is opt-in and only allowed with an available tray. No daemon or localhost web server is needed for the first foundation.

Use immutable, content-verified snapshots and a persistent local journal in the sync milestone. Preserve concurrent conversation branches and conflicting settings. Google Drive API with desktop OAuth is the primary planned transport (Linux has no official Drive desktop client); an existing local Drive folder is an optional transport. Do not use cloud lock files as distributed mutexes.

## Milestones and gates

| Milestone | Deliverable | Gate |
| --- | --- | --- |
| M0 | Repository, five-language entry points, requirements, plan, brand asset | Links, public-data review, planning commit pushed, BastetMind source and index |
| M1 | Tauri desktop, five-language GUI/tray, agent-path discovery, native folder picker, settings persistence | Rust + frontend tests, build, macOS visual smoke; Windows/Linux CI tracked separately |
| M2 | Snapshot schema, journal, stable exports, validation, retries, conflicts, local-folder transport | Two isolated fixtures, interrupted transfer, no loops or overwrite, malicious bundle tests |
| M3 | Google Drive OAuth, folder wizard, API transport, credential storage, encryption | Real two-device transfer, reconnect, quota/backoff, key recovery; requires configured OAuth client |
| M4 | Versioned adapters and native conversation restore | Each agent tested independently across computers; attachments/project paths verified |
| M5 | Scheduler, near-real-time, bidirectional UX, packaging and autostart | Three-platform install/tray/restart tests, signed distribution plan, locale QA |

M1 saves setup choices. M2 adds a real filesystem diagnostic using isolated synthetic data; it never uses selected agent paths or the selected Drive folder. Neither enables a pretend agent sync button. Start remains explicitly unavailable until a transport and snapshot adapter have passed their gates. CLI executables are not run automatically during discovery; presence does not establish version or compatibility.

## Data boundaries

Exclude credentials, sockets, locks, caches and active databases from raw copying. Export external memory stores through their supported interfaces. Preserve original histories; model context windows can limit what is loaded. Worktree state is separate from conversation state. Never erase files just because a source directory is temporarily missing.

## Documentation and knowledge

Root README follows Bastet's cat identity, badges, honest status and language navigation. Five localized guides carry the same scope, milestones, commands and limitations. Technical contracts may have a canonical source with localized summaries. Every milestone records its documents and evidence in BastetMind with an immutable source snapshot and updated topic/index/log; operational details stay there.

## Open questions

Exact Claude desktop import API; Codex desktop versus CLI session parity; Agy persistence; agent version compatibility; supported OS minimums; Google OAuth distribution credentials and shared-folder scopes; encryption recovery and release signing. These are validation work, not assumed capabilities.

M2 protocol and precise limitations: [SNAPSHOT_PROTOCOL.md](SNAPSHOT_PROTOCOL.md). The journal is a rebuildable checkpoint backed by immutable objects. No deletion propagation, native import, encryption or background scheduling is included in the M2 core.

M3 implementation boundaries and release gates: [Cloud transport and encryption](CLOUD_SECURITY.md).

Setup wizard, resume/restart and manual configuration: [Setup contract](SETUP_WIZARD.md).
