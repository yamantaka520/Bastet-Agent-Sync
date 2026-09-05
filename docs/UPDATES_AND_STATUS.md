# 🐈 Runtime status and signed updates

Historical 0.2.0 behavior (superseded by [automatic AMOS sync in 0.2.1](AGENT_MEMORY_OS.md)): version 0.2.0 replaces the hard-coded disconnected label and permanently disabled Start button. Drive setup completion is reported from the native wizard; saved completion is distinct from a token currently held in this process. An idle, read-only status refresh checks token expiry every 15 seconds without contacting Google. It cannot detect remote revocation until a request is made.

## Historical 0.2.0 Start and status

Start validates saved settings and Drive wizard progress, then runs selected local conversation adapters and the AMOS merge independently. Per-source results distinguish empty, syncing, partial and failed work. Credential access has its own visible phase. Missing sessions are added; existing different versions remain conflicts. Pause keeps completed source counters. Settings and updates are blocked while the worker runs. “App running” still indicates this desktop instance, not proof of a completed transfer. See [current adapter scope](NATIVE_SESSIONS.md).

## Current version identity

The version below the logo comes from the native package, with a build Git revision; browser preview uses the package version. Use `python scripts/set_version.py 0.2.1` for the next release. Documentation checks enforce agreement between npm, lockfiles, Cargo and Tauri versions. A commit identifies a clean build; rebuild after committing so local development changes are included in the displayed revision.

## Online update

The sidebar provides Check for updates, Download and install, and Restart. Checking is explicit; installation requires another click, is blocked while main settings have unsaved changes, and reports download bytes. The Rust updater retains the checked update internally; the renderer cannot submit a URL or installer path. It accepts artifact URLs only under this repository's HTTPS GitHub release downloads. Tauri verifies the pinned update signature before installation. Invalid signatures, unavailable feeds, unsupported platforms and network failures never become success. The application does not automatically restart on macOS/Linux; the Windows installer may exit the old application during installation.

The fixed feed is `https://github.com/yamantaka520/Bastet-Agent-Sync/releases/latest/download/latest.json`. Without a valid published release, the UI reports that no valid feed is available, not that the installed app is necessarily current. No release package has yet been installed through this new updater; install/restart UI tests use fixtures. Cross-platform self-update acceptance remains open.

## Release operations

[Release notes template](RELEASE_NOTES.md) documents installation and support boundaries. `.github/workflows/release.yml` is a manual, main-branch-only signed draft workflow. It builds macOS arm64/x64, Windows x64 and Linux x64 independently, then validates all packages in one final job. It uses `tauri.release.conf.json` to produce updater archives/signatures and a merged `latest.json`, then leaves the release **draft** for artifact review. Normal pushes do not publish or install anything. Before dispatch, push the version tag at the exact main commit through an authorized Git identity; draft creation verifies that tag instead of asking the Actions token to create a workflow-bearing reference. If the platform denies release API access, the signed workflow artifacts remain available for authorized CLI publication. After verifying all platform packages and signatures, publish that draft to make the feed available. Do not publish a partially completed matrix.

The updater public key is checked in. The private key is excluded from Git and stored in the repository's encrypted `TAURI_SIGNING_PRIVATE_KEY` Actions secret; keep its local backup protected. Never put it into Markdown, logs or the notebook. Update signatures are separate from Apple notarization and Windows Authenticode. The draft pipeline currently uses macOS ad-hoc signing; production OS signing/notarization is not claimed.

Official implementation references: [Tauri updater](https://v2.tauri.app/plugin/updater/), [GitHub release pipeline](https://v2.tauri.app/distribute/pipelines/github/).
