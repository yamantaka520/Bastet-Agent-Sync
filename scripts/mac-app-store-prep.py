#!/usr/bin/env python3
"""Create local Tauri signing inputs from a real Mac App Store profile."""

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
TEMPLATE = ROOT / "scripts/mac-app-store.entitlements.plist"
CONFIG = ROOT / "src-tauri/tauri.conf.json"


def fail(message):
    raise ValueError(message)


def generate(profile, output, decoded=None):
    identifier = json.loads(CONFIG.read_text(encoding="utf-8"))["identifier"]
    if not profile.is_file() or not profile.stat().st_size:
        fail("Missing provisioning profile")
    if decoded is None:
        if sys.platform != "darwin":
            fail("A macOS host is required to decode a provisioning profile")
        try:
            decoded = subprocess.run(
                ["security", "cms", "-D", "-i", str(profile)],
                check=True, capture_output=True,
            ).stdout
        except subprocess.CalledProcessError:
            # security cms can reject a locally unavailable Apple cert chain.
            # OpenSSL verifies the CMS signature; -noverify does not authenticate
            # the signer chain, so identity/profile binding is checked separately.
            try:
                decoded = subprocess.run(
                    ["openssl", "cms", "-verify", "-inform", "DER", "-noverify", "-in", str(profile)],
                    check=True, capture_output=True,
                ).stdout
            except subprocess.CalledProcessError:
                fail("Provisioning profile CMS signature could not be verified")
    data = plistlib.loads(decoded)
    if data.get("Platform") != ["OSX"]:
        fail("An OSX provisioning profile is required")
    team_ids = data.get("TeamIdentifier", [])
    if not isinstance(team_ids, list) or len(team_ids) != 1 or not re.fullmatch(r"[A-Z0-9]{10}", team_ids[0]):
        fail("Provisioning profile has no unique Team ID")
    team = team_ids[0]
    profile_entitlements = data.get("Entitlements", {})
    app_id = profile_entitlements.get("com.apple.application-identifier") or profile_entitlements.get("application-identifier")
    if app_id != f"{team}.{identifier}":
        fail("Provisioning profile App ID does not match the Tauri identifier and Team ID")
    if profile_entitlements.get("com.apple.developer.team-identifier") != team:
        fail("Provisioning profile entitlement Team ID differs")
    if profile_entitlements.get("get-task-allow") is True:
        fail("A development profile with get-task-allow=true is not allowed")
    if data.get("ProvisionedDevices") or data.get("ProvisionsAllDevices"):
        fail("A Mac App Store Connect distribution profile is required")
    expires = data.get("ExpirationDate")
    if not isinstance(expires, dt.datetime) or expires <= dt.datetime.now(dt.timezone.utc).replace(tzinfo=None):
        fail("Provisioning profile is expired or has no expiration date")
    certificates = data.get("DeveloperCertificates")
    if not isinstance(certificates, list) or not certificates:
        fail("Provisioning profile contains no distribution certificate")
    entitlements = plistlib.loads(TEMPLATE.read_bytes())
    for key, value in entitlements.items():
        if key in profile_entitlements and profile_entitlements[key] != value:
            fail(f"Provisioning profile forbids required entitlement: {key}")
    entitlements["com.apple.application-identifier"] = app_id
    entitlements["com.apple.developer.team-identifier"] = team
    output.mkdir(parents=True, exist_ok=True)
    os.chmod(output, 0o700)
    entitlement_path = output / "Entitlements.plist"
    entitlement_path.write_bytes(plistlib.dumps(entitlements))
    embedded_path = output / "embedded.provisionprofile"
    shutil.copyfile(profile, embedded_path)
    overlay = {
        "bundle": {
            "macOS": {
                "entitlements": str(entitlement_path),
                "files": {"embedded.provisionprofile": str(embedded_path)},
            }
        }
    }
    (output / "tauri.profile.conf.json").write_text(json.dumps(overlay, indent=2) + "\n", encoding="utf-8")
    (output / "profile-cert-sha1.txt").write_text(
        "\n".join(hashlib.sha1(cert).hexdigest().upper() for cert in certificates) + "\n", encoding="ascii"
    )
    for path in output.iterdir():
        if path.is_file():
            os.chmod(path, 0o600)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    # Fixture-only interface. The signing wrapper never passes this argument.
    parser.add_argument("--test-decoded-plist", type=Path)
    args = parser.parse_args()
    decoded = args.test_decoded_plist.read_bytes() if args.test_decoded_plist else None
    try:
        generate(args.profile.resolve(), args.output.resolve(), decoded)
    except (ValueError, OSError, subprocess.CalledProcessError, plistlib.InvalidFileException) as exc:
        print(f"Store preparation failed: {exc}", file=sys.stderr)
        return 1
    print("Store profile and entitlements validated; local build configuration prepared")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
