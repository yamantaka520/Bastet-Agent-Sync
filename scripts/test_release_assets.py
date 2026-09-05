"""Regression checks for publishing an incomplete or mislinked release."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import release_assets as release

class ReleaseAssetsTests(unittest.TestCase):

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.out = Path(self.temp.name)
        self.patch = patch.object(release, 'OUT', self.out)
        self.patch.start()
        self.addCleanup(self.patch.stop)

    def fixtures(self):
        for platform, suffixes in release.TARGETS.values():
            for suffix in suffixes:
                (self.out / release.asset(platform, suffix)).write_text('synthetic fixture', encoding='utf-8', newline='\n')

    def test_missing_target_prevents_manifest(self):
        self.fixtures()
        (self.out / release.asset('macos-x64', '.dmg')).unlink()
        with self.assertRaises(RuntimeError):
            release.prepare()
        self.assertFalse((self.out / 'latest.json').exists())

    def test_empty_signature_prevents_manifest(self):
        self.fixtures()
        (self.out / release.asset('windows-x64', '.exe.sig')).write_text('', encoding='utf-8', newline='\n')
        with self.assertRaises(RuntimeError):
            release.prepare()
        self.assertFalse((self.out / 'latest.json').exists())

    def test_complete_release_links_to_existing_assets(self):
        self.fixtures()
        release.prepare()
        manifest = json.loads((self.out / 'latest.json').read_text(encoding='utf-8'))
        self.assertEqual(set(manifest['platforms']), {'darwin-aarch64', 'darwin-x86_64', 'windows-x86_64', 'linux-x86_64', 'linux-x86_64-appimage', 'linux-x86_64-deb', 'linux-x86_64-rpm', 'windows-x86_64-nsis', 'windows-x86_64-msi'})
        for entry in manifest['platforms'].values():
            self.assertTrue((self.out / entry['url'].split('/')[-1]).is_file())
        self.assertTrue(manifest['platforms']['linux-x86_64-deb']['url'].endswith('.deb'))
        self.assertTrue(manifest['platforms']['linux-x86_64-rpm']['url'].endswith('.rpm'))
        self.assertTrue(manifest['platforms']['windows-x86_64-msi']['url'].endswith('.msi'))
        notes = (self.out / 'release-notes.md').read_text(encoding='utf-8')
        self.assertNotIn('@DOWNLOADS@', notes)
        self.assertNotIn('@VERSION@', notes)
        for language in ['繁體中文', '简体中文', 'English', '日本語', '한국어']:
            self.assertIn(language, notes)
        checksums = (self.out / 'SHA256SUMS').read_text(encoding='utf-8')
        release.prepare()
        for platform, suffixes in release.TARGETS.values():
            for suffix in suffixes:
                self.assertEqual(checksums.count('  ' + release.asset(platform, suffix) + '\n'), 1)
if __name__ == '__main__':
    unittest.main()
