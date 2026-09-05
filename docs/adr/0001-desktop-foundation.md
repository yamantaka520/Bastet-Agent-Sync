# ADR 0001 — Tauri desktop and Rust-owned state

Status: adopted for M1, 2026-09-05.

Use Tauri 2, React and TypeScript, following the existing Bastet desktop family. Keep file discovery, settings validation and persistence in Rust. Expose typed commands to the UI. The first milestone never reads conversation contents or uploads agent data. GUI settings are separate from agent profiles.

Use the original generated calico-cat avatar for the app, tray and documentation. Derive platform icon sizes from the same asset. Five locales share a checked key set.

Tray presence is platform-dependent. Hiding on close is an explicit setting; a missing tray must not make the application inaccessible. OAuth and the sync scheduler are separate later milestones, so setting a folder is not represented as a successful cloud connection.
