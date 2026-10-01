#!/usr/bin/env python3
"""Optional installed-provider reads of Bastet's isolated restored fixtures.

The caller supplies a manifest under a temporary directory. This script never
opens a default agent profile, invokes a model, or contacts a sync space. A
passing read establishes that an installed provider can parse the restored
synthetic history; it does not establish interactive or physical-device resume.

Manifest example (all paths must resolve inside --root):
{"cases": [{"agent": "codex", "profile": "restored-codex",
            "session": "019f0000-0000-7000-8000-000000000001",
            "marker": "fixture"}]}
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from queue import Empty, Queue
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from threading import Thread


def isolated_environment(sandbox: Path, profile: Path) -> dict[str, str]:
    home = sandbox / "home"
    home.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment.update(
        HOME=str(home),
        USERPROFILE=str(home),
        APPDATA=str(home / "AppData"),
        LOCALAPPDATA=str(home / "LocalAppData"),
        XDG_CONFIG_HOME=str(home / ".config"),
        XDG_DATA_HOME=str(home / ".local/share"),
        XDG_CACHE_HOME=str(home / ".cache"),
        CODEX_HOME=str(profile),
        CLAUDE_CONFIG_DIR=str(profile),
        PI_CODING_AGENT_DIR=str(profile),
        GROK_HOME=str(profile),
        DO_NOT_TRACK="1",
    )
    for key in list(environment):
        if key.endswith("_API_KEY") or key.endswith("_TOKEN") or key in {
            "AWS_PROFILE", "GOOGLE_APPLICATION_CREDENTIALS", "CLAUDE_CODE_OAUTH_TOKEN"
        }:
            environment.pop(key, None)
    return environment


def same_working_directory(actual: object, expected: str) -> bool:
    """Compare existing native directories, including Windows separators/case/prefixes."""
    if not isinstance(actual, str) or not Path(actual).is_absolute():
        return False
    try:
        return os.path.samefile(actual, expected)
    except (OSError, ValueError):
        return False


def codex_read(binary: str, profile: Path, session: str, marker: str,
               sandbox: Path, expected_cwd: str | None) -> None:
    environment = isolated_environment(sandbox, profile)
    process = subprocess.Popen(
        [binary, "app-server", "--stdio"],
        cwd=sandbox,
        env=environment,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        bufsize=1,
    )
    requests = [
        {"id": 1, "method": "initialize", "params": {
            "clientInfo": {"name": "bastet-fixture", "version": "1"}, "capabilities": {}}},
        {"method": "initialized", "params": {}},
        {"id": 2, "method": "thread/read", "params": {
            "threadId": session, "includeTurns": True}},
    ]
    try:
        assert process.stdin is not None and process.stdout is not None
        responses: Queue[str | None] = Queue()

        def read_lines() -> None:
            for line in process.stdout:
                responses.put(line)
            responses.put(None)

        Thread(target=read_lines, daemon=True).start()
        for request in requests:
            process.stdin.write(json.dumps(request) + "\n")
        process.stdin.flush()
        for _ in range(200):
            try:
                line = responses.get(timeout=0.1)
            except Empty:
                continue
            if line is None:
                raise AssertionError("Codex app server closed before thread/read")
            response = json.loads(line)
            if response.get("id") != 2:
                continue
            if "error" in response:
                raise AssertionError(f"Codex thread/read rejected fixture: {response['error']}")
            data = response.get("result", {})
            result = json.dumps(data, ensure_ascii=False)
            if marker not in result:
                raise AssertionError("Codex thread/read omitted fixture marker")
            if expected_cwd and not same_working_directory(data.get("thread", {}).get("cwd"), expected_cwd):
                raise AssertionError("Codex thread/read omitted mapped working directory")
            return
        raise AssertionError("Codex thread/read timed out")
    finally:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)


def pi_read(node: str, module: Path, profile: Path, session: str, marker: str,
            sandbox: Path, expected_cwd: str | None) -> None:
    matches = list(profile.glob(f"sessions/**/*{session}*.jsonl"))
    if len(matches) != 1:
        raise AssertionError(f"expected one Pi session file, found {len(matches)}")
    source = """import { pathToFileURL } from 'node:url';
const { SessionManager } = await import(pathToFileURL(process.argv[1]).href);
const manager = SessionManager.open(process.argv[2]);
process.stdout.write(JSON.stringify({cwd: manager.getCwd(), entries: manager.getEntries()}));
"""
    result = subprocess.run(
        [node, "--input-type=module", "-e", source, str(module), str(matches[0])],
        cwd=sandbox, env=isolated_environment(sandbox, profile),
        capture_output=True, text=True, timeout=20, check=True,
    )
    if marker not in result.stdout:
        raise AssertionError("Pi SessionManager omitted fixture marker")
    if expected_cwd and not same_working_directory(json.loads(result.stdout)["cwd"], expected_cwd):
        raise AssertionError("Pi SessionManager retained the old working directory")


def claude_read(node: str, module: Path, profile: Path, session: str, marker: str,
                sandbox: Path, expected_cwd: str | None) -> None:
    # These two official SDK functions inspect the transcript; neither sends a prompt.
    source = """import { pathToFileURL } from 'node:url';
const { listSessions, getSessionMessages } = await import(pathToFileURL(process.argv[1]).href);
const sessionId = process.argv[2], dir = process.argv[3];
const options = dir ? { dir } : {};
const sessions = await listSessions(options);
const messages = await getSessionMessages(sessionId, options);
process.stdout.write(JSON.stringify({ sessions, messages }));
"""
    result = subprocess.run(
        [node, "--input-type=module", "-e", source, str(module), session, expected_cwd or ""],
        cwd=sandbox, env=isolated_environment(sandbox, profile),
        capture_output=True, text=True, timeout=20, check=True,
    )
    data = json.loads(result.stdout)
    matching = [item for item in data["sessions"] if item["sessionId"] == session]
    if len(matching) != 1:
        raise AssertionError("Claude SDK listSessions did not find the restored session")
    if expected_cwd and not same_working_directory(matching[0].get("cwd"), expected_cwd):
        raise AssertionError("Claude SDK listSessions retained the old working directory")
    if marker not in json.dumps(data["messages"], ensure_ascii=False):
        raise AssertionError("Claude SDK getSessionMessages omitted fixture marker")


def grok_read(binary: str, profile: Path, session: str, marker: str,
              sandbox: Path, expected_cwd: str | None) -> None:
    summaries = list(profile.glob(f"sessions/**/{session}/summary.json"))
    if len(summaries) != 1:
        raise AssertionError(f"expected one Grok session summary, found {len(summaries)}")
    if expected_cwd:
        summary = json.loads(summaries[0].read_text(encoding="utf-8"))
        if not same_working_directory(summary.get("info", {}).get("cwd"), expected_cwd):
            raise AssertionError("Grok summary retained the old working directory")
    listed = subprocess.run(
        [binary, "sessions", "list"], cwd=expected_cwd or sandbox,
        env=isolated_environment(sandbox, profile),
        capture_output=True, text=True, timeout=20, check=True,
    )
    if session not in listed.stdout:
        raise AssertionError("Grok sessions list did not discover the restored session")
    result = subprocess.run(
        [binary, "export", session], cwd=sandbox,
        env=isolated_environment(sandbox, profile),
        capture_output=True, text=True, timeout=20, check=True,
    )
    if marker not in result.stdout:
        raise AssertionError("Grok export omitted fixture marker")


def agy_integrity(profile: Path, session: str) -> None:
    path = profile / "conversations" / f"{session}.db"
    if not path.is_file():
        raise AssertionError("Agy restored conversation database missing")
    # Read-only SQLite inspection is weaker than restored-profile CLI resume.
    with sqlite3.connect(f"file:{path.as_posix()}?mode=ro&immutable=1", uri=True) as database:
        if database.execute("PRAGMA integrity_check").fetchone() != ("ok",):
            raise AssertionError("Agy restored database failed integrity_check")
        if database.execute("SELECT COUNT(*) FROM steps").fetchone()[0] == 0:
            raise AssertionError("Agy restored database has no steps")


def canonical_agent(agent: str) -> str:
    return {"chatgpt-work": "codex", "claude": "claude-code"}.get(agent, agent)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path, help="isolated temporary fixture root")
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--pi-module", type=Path, help="installed Pi core/session-manager.js")
    parser.add_argument("--claude-sdk-module", type=Path,
                        help="official Claude Agent SDK sdk.mjs (optional, installed outside this repo)")
    parser.add_argument("--require-agents", default="",
                        help="comma-separated agents whose installed readers must pass")
    args = parser.parse_args()
    root = args.root.resolve(strict=True)
    temp_roots = [Path(tempfile.gettempdir()).resolve()]
    if os.name == "posix":
        temp_roots.append(Path("/tmp").resolve())
    if not any(root.is_relative_to(temp_root) for temp_root in temp_roots):
        parser.error("--root must be inside a host temporary directory")
    manifest = args.manifest.resolve(strict=True)
    if not manifest.is_relative_to(root):
        parser.error("--manifest must be inside --root")
    cases = json.loads(manifest.read_text(encoding="utf-8"))["cases"]
    required = {canonical_agent(agent.strip()) for agent in args.require_agents.split(",") if agent.strip()}
    unknown = required - {"codex", "claude-code", "pi", "grok"}
    if unknown:
        parser.error(f"unknown required agents: {', '.join(sorted(unknown))}")
    codex = shutil.which("codex")
    grok = shutil.which("grok")
    node = shutil.which("node")
    pi_module = args.pi_module
    claude_sdk_module = args.claude_sdk_module
    if pi_module is None and shutil.which("pi"):
        pi_cli = Path(shutil.which("pi") or "").resolve()
        pi_module = pi_cli.parent.parent / "core" / "session-manager.js"
    failures = 0
    passed = set()
    for index, case in enumerate(cases):
        agent = case["agent"]
        profile = (root / case["profile"]).resolve(strict=True)
        if not profile.is_relative_to(root):
            parser.error(f"case {index} profile escapes --root")
        session, marker = case["session"], case["marker"]
        expected_cwd = case.get("cwd")
        sandbox = root / f"reader-{index}"
        sandbox.mkdir(exist_ok=True)
        try:
            if agent in {"codex", "chatgpt-work"} and codex:
                codex_read(codex, profile, session, marker, sandbox, expected_cwd)
                verdict = "PASS installed Codex thread/read (no model turn)"
            elif agent == "pi" and node and pi_module and pi_module.is_file():
                pi_read(node, pi_module, profile, session, marker, sandbox, expected_cwd)
                verdict = "PASS installed Pi SessionManager read (no model turn)"
            elif agent == "grok" and grok:
                grok_read(grok, profile, session, marker, sandbox, expected_cwd)
                verdict = "PASS installed Grok transcript export (no model turn)"
            elif agent == "agy":
                agy_integrity(profile, session)
                verdict = "PASS SQLite integrity only; restored-profile CLI resume unverified"
            elif agent in {"claude", "claude-code"} and node and claude_sdk_module and claude_sdk_module.is_file():
                claude_read(node, claude_sdk_module, profile, session, marker, sandbox, expected_cwd)
                verdict = "PASS official Claude SDK session list/read (no model turn)"
            elif agent in {"claude", "claude-code"}:
                verdict = "UNVERIFIED optional official Claude SDK reader unavailable"
            else:
                verdict = "SKIP required installed reader unavailable"
            print(f"{agent}: {verdict}")
            if verdict.startswith("PASS "):
                passed.add(canonical_agent(agent))
        except (AssertionError, OSError, subprocess.SubprocessError, ValueError) as error:
            failures += 1
            print(f"{agent}: FAIL {error}", file=sys.stderr)
    for agent in sorted(required - passed):
        failures += 1
        print(f"{agent}: FAIL required installed reader did not pass", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
