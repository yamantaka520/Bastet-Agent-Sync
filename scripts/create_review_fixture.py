#!/usr/bin/env python3
"""Create a portable, synthetic Claude Code sample for App Review.

The source cwd is deliberately virtual. On a receiving Mac, map that recorded
path to the extracted ReceivingProject through Bastet's project-path UI.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo


SOURCE_CWD = "/BastetReviewSourceProject"
SESSION_ID = "192160fa-35ee-4079-9266-b53e6d6fad73"
RECORD_ID = "b5acfb6f-4279-42cf-ad58-dd270886d767"
SAMPLE_NAME = "BastetReviewSample"


def claude_group(cwd: str) -> str:
    """Mirror the ASCII part of project_mapping.rs::claude_group for this fixture."""
    if not cwd.isascii() or len(cwd) > 200:
        raise ValueError("Fixture cwd must be short ASCII")
    return "".join(char if char.isalnum() else "-" for char in cwd)


def build(base: Path) -> Path:
    base = base.expanduser().resolve()
    base.mkdir(parents=True, exist_ok=True)
    sample = base / SAMPLE_NAME
    archive = base / f"{SAMPLE_NAME}.zip"
    if sample.exists() or archive.exists():
        raise FileExistsError("Review fixture already exists; choose an empty output base")

    source = sample / "SampleClaudeConfig" / "projects" / claude_group(SOURCE_CWD)
    source.mkdir(parents=True)
    record = {
        "type": "user",
        "sessionId": SESSION_ID,
        "cwd": SOURCE_CWD,
        "uuid": RECORD_ID,
        "parentUuid": None,
        "isSidechain": False,
        "timestamp": "2026-10-02T00:00:00Z",
        "message": {
            "role": "user",
            "content": "Synthetic Bastet review sample. No personal conversations or credentials.",
        },
    }
    (source / f"{SESSION_ID}.jsonl").write_text(
        json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )

    receiving = sample / "ReceivingProject"
    receiving.mkdir()
    (receiving / "README.txt").write_text(
        "Synthetic receiving project; no code or private data.\n", encoding="utf-8"
    )
    restore_parent = sample / "RestoredProfilesParent"
    restore_parent.mkdir()
    (restore_parent / "README.txt").write_text(
        "Choose this as the parent for a separate restored agent profile.\n",
        encoding="utf-8",
    )
    (sample / "README.md").write_text(
        """# Bastet AI Sync review sample

This is a fictional, single-message Claude Code-format source. It contains no
personal conversation, credential, project code, or third-party content. It is
for testing Bastet AI Sync's folder selection and, optionally, its Google Drive
snapshot flow. It does not demonstrate model continuation.

## Inspect the local source

1. Extract this ZIP anywhere on the Mac. Keep the folders together. No terminal,
   Claude account, or manual move to a system directory is required.
2. Launch Bastet AI Sync 0.6.0 (1). In the **Claude Code** source card, click
   **Choose path** and select the extracted **SampleClaudeConfig** folder (the
   parent of `projects`). Approve the macOS folder picker.
3. Check **Claude Code**, click **Save setup**, and click **Scan again** if needed.
   The sample has exactly one synthetic user record. Setup and scanning do not
   transfer it.

## Optional Google Drive upload and separate restore

1. In Google Drive setup, click **Use built-in configuration**, then
   **Authorize Google Drive**. Sign in with your own Google account in the
   browser. Create or select a Drive folder, create the space key, save the
   recovery kit outside Drive, verify the encrypted space, and finish setup.
2. In **Project paths**, click **Add mapping**. Enter
   `/BastetReviewSourceProject` as **Source project path**. Click
   **Choose local project** and select this ZIP's extracted **ReceivingProject**
   folder. Click **Save setup**. The source path is deliberately virtual; do
   not create it. This maps a source computer's recorded path to this Mac's
   real folder, without copying project files.
3. With only the sample Claude Code source selected, click **Start sync**.
   Inspect the actual status. If you test a restore, use **View conversation
   snapshots**, expand Claude Code, and choose **Restore to a new folder**.
   Select this ZIP's **RestoredProfilesParent** in the macOS picker. The restored
   profile is separate from the selected source and receiving project.

Google authorization is required for cloud sync. A compatible installed Claude
Code and its own account are required to continue a restored conversation in
that tool; this single-message fixture does not establish such continuation.
Never select an active agent store or working project as a restore destination.
""",
        encoding="utf-8",
    )

    with ZipFile(archive, "w", ZIP_DEFLATED) as zipped:
        for path in sorted(sample.rglob("*")):
            if path.is_file():
                info = ZipInfo(str(path.relative_to(base)), (2026, 10, 2, 0, 0, 0))
                info.compress_type = ZIP_DEFLATED
                info.external_attr = 0o100644 << 16
                zipped.writestr(info, path.read_bytes())
    return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", type=Path, required=True, help="Empty output directory")
    args = parser.parse_args()
    print(build(args.base))
