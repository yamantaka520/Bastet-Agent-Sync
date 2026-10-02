#!/usr/bin/env python3
"""Fixture checks for profile-derived Store signing inputs."""

import datetime as dt
import importlib.util
from pathlib import Path
import plistlib
import tempfile
import unittest

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


if __name__ == "__main__":
    unittest.main()
