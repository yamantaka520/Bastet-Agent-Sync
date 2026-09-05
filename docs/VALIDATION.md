# Validation

## M2 snapshot core — 2026-09-05

- Local Rust: **24 tests passed** (6 foundation + 18 snapshot/transport tests; the Unix symlink case is not run on Windows).
- Frontend: **7 tests passed**, including invoking only the isolated native diagnostic and clearing stale success on a failed retry.
- TypeScript/Vite build, rustfmt, Clippy with warnings denied, and documentation checks passed.
- macOS debug `.app` built and launched. Pressing the native **Run isolated check** produced: 2 publication/reception operations, 2 preserved branches, 0 additional transfers on repeat, and 3 objects recovered on reopen. The UI explicitly labels this as a core check, not cloud sync.
- Test coverage includes child-before-parent arrival, partial/corrupt bundles and retries, checkpoint loss, stale conflict resolution, opposite-direction traffic prevention, Unicode text, oversized files, invalid paths, hash mismatch, wrong space, cross-stream parents, symlinks, no deletion propagation and exclusive replica locks.
- M1 Windows CI failed while decoding five-language Markdown with the host's default cp1252 encoding. `check_docs.py` now reads UTF-8 explicitly. The M2 baseline `387ab35` passed macOS, Windows and Ubuntu in [run 33953575895](https://github.com/yamantaka520/Bastet-Agent-Sync/actions/runs/33953575895). An additional explicit-baseline test verifies that receiving a newer remote head does not silently rebase already-staged local edits; its updated run is tracked separately.
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
- Local-folder selection does not prove Google Drive is connected or has transmitted data. OAuth, encryption, snapshot transport, scheduling, bidirectional merge and native session restoration remain M2–M5.
- Native OS menus and the folder chooser follow OS localization; application labels and custom tray items use the selected language. Native menu localization and assistive-technology coverage need further QA.
- No signed release or auto-start installer has been published.
