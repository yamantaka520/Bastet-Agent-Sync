#!/usr/bin/env python3
"""Fixture checks for profile-derived Store signing inputs."""

import datetime as dt
import importlib.util
from pathlib import Path
import plistlib
import tempfile
import unittest
import os

spec = importlib.util.spec_from_file_location("store_prep", Path(__file__).with_name("mac-app-store-prep.py"))
prep = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prep)


class StorePrepTests(unittest.TestCase):
    def profile(self):
        team = "ABCDEFGHIJ"
        entitlements = plistlib.loads(prep.TEMPLATE.read_bytes())
        entitlements.update({
            "com.apple.application-identifier": team + ".tw.bastet.agent-sync",
            "com.apple.developer.team-identifier": team,
        })
        return {
            "Platform": ["OSX"],
            "TeamIdentifier": [team],
            "Entitlements": entitlements,
            "ExpirationDate": dt.datetime(2099, 1, 1),
            "DeveloperCertificates": [b"fixture certificate"],
        }

    def test_generates_local_profile_bound_config(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            profile = base / "fixture.provisionprofile"
            profile.write_bytes(b"fixture cms")
            output = base / "output"
            prep.generate(profile, output, plistlib.dumps(self.profile()))
            generated = plistlib.loads((output / "Entitlements.plist").read_bytes())
            self.assertEqual(generated["com.apple.application-identifier"], "ABCDEFGHIJ.tw.bastet.agent-sync")
            self.assertEqual(generated["com.apple.security.files.bookmarks.app-scope"], True)
            self.assertEqual((output / "embedded.provisionprofile").read_bytes(), profile.read_bytes())
            self.assertIn("embedded.provisionprofile", (output / "tauri.profile.conf.json").read_text())

    def test_rejects_wrong_app_id_and_missing_distribution_certificate(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            profile = base / "fixture.provisionprofile"
            profile.write_bytes(b"fixture cms")
            data = self.profile()
            data["Entitlements"]["com.apple.application-identifier"] = "ABCDEFGHIJ.other.app"
            with self.assertRaisesRegex(ValueError, "App ID"):
                prep.generate(profile, base / "output", plistlib.dumps(data))
            data = self.profile()
            data["DeveloperCertificates"] = []
            with self.assertRaisesRegex(ValueError, "certificate"):
                prep.generate(profile, base / "output", plistlib.dumps(data))
            data = self.profile()
            data["Entitlements"]["get-task-allow"] = True
            with self.assertRaisesRegex(ValueError, "development profile"):
                prep.generate(profile, base / "output", plistlib.dumps(data))

    @unittest.skipUnless(os.name == "posix", "macOS bundle permissions require POSIX file modes")
    def test_app_permissions_and_embedded_manifest(self):
        with tempfile.TemporaryDirectory() as temp:
            app = Path(temp) / "Fixture.app"
            macos = app / "Contents/MacOS"
            resources = app / "Contents/Resources"
            macos.mkdir(parents=True)
            resources.mkdir()
            (app / "Contents/Info.plist").write_bytes(plistlib.dumps({"CFBundleExecutable": "fixture"}))
            binary = macos / "fixture"
            binary.write_bytes(b"binary fixture")
            os.chmod(binary, 0o700)
            manifest = resources / "PrivacyInfo.xcprivacy"
            manifest.write_bytes(prep.PRIVACY_MANIFEST.read_bytes())
            os.chmod(app, 0o700)
            os.chmod(app / "Contents", 0o700)
            os.chmod(app / "Contents/Info.plist", 0o600)
            with self.assertRaisesRegex(ValueError, "inaccessible"):
                prep.check_app_payload(app)
            prep.normalize_app_permissions(app)
            prep.check_app_payload(app)
            self.assertEqual(app.stat().st_mode & 0o777, 0o755)
            self.assertEqual(binary.stat().st_mode & 0o777, 0o755)
            self.assertEqual((app / "Contents/Info.plist").stat().st_mode & 0o777, 0o644)
            manifest.write_bytes(plistlib.dumps({"NSPrivacyTracking": True}))
            with self.assertRaisesRegex(ValueError, "differs from source"):
                prep.check_app_payload(app)

if __name__ == "__main__":
    unittest.main()
