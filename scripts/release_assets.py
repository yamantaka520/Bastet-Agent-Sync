"""Collect deterministic release names; require four targets before drafting."""
import argparse
import datetime
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
from urllib.parse import quote
ROOT = Path(__file__).resolve().parent.parent
VERSION = json.loads((ROOT / 'package.json').read_text(encoding='utf-8'))['version']
TAG = 'v' + VERSION
REPO = 'yamantaka520/Bastet-Agent-Sync'
OUT = ROOT / 'release-assets'
TARGETS = {'aarch64-apple-darwin': ('macos-arm64', ['.dmg', '.app.tar.gz', '.app.tar.gz.sig']), 'x86_64-apple-darwin': ('macos-x64', ['.dmg', '.app.tar.gz', '.app.tar.gz.sig']), 'x86_64-pc-windows-msvc': ('windows-x64', ['.exe', '.exe.sig', '.msi', '.msi.sig']), 'x86_64-unknown-linux-gnu': ('linux-x64', ['.deb', '.rpm', '.AppImage', '.AppImage.sig'])}

def asset(platform, suffix):
    return f'Bastet-Agent-Sync-{VERSION}-{platform}{suffix}'

def collect(target):
    platform, suffixes = TARGETS[target]
    base = ROOT / 'src-tauri' / 'target' / target / 'release' / 'bundle'
    OUT.mkdir(exist_ok=True)
    for suffix in suffixes:
        matches = [p for p in base.rglob('*' + suffix) if p.is_file()]
        if len(matches) != 1:
            raise RuntimeError(f'Expected one {suffix} for {target}; found {len(matches)}')
        shutil.copyfile(matches[0], OUT / asset(platform, suffix))

def prepare():
    for platform, suffixes in TARGETS.values():
        for suffix in suffixes:
            p = OUT / asset(platform, suffix)
            if not p.is_file() or p.stat().st_size == 0:
                raise RuntimeError(f'Missing or empty package: {p.name}')
    base = f'https://github.com/{REPO}/releases/download/{TAG}/'
    platforms = {}
    for key, platform, suffix in [('darwin-aarch64', 'macos-arm64', '.app.tar.gz'), ('darwin-x86_64', 'macos-x64', '.app.tar.gz'), ('windows-x86_64', 'windows-x64', '.exe'), ('linux-x86_64', 'linux-x64', '.AppImage')]:
        name = asset(platform, suffix)
        signature = (OUT / (name + '.sig')).read_text(encoding='utf-8').strip()
        if not signature:
            raise RuntimeError('Empty updater signature')
        platforms[key] = {'signature': signature, 'url': base + quote(name)}
    manifest = {'version': VERSION, 'notes': 'Bastet Agent Sync — see the five-language release notes.', 'pub_date': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'platforms': platforms}
    (OUT / 'latest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8', newline='\n')
    installer = (ROOT / 'scripts/install-linux.sh').read_text(encoding='utf-8').replace('@VERSION@', VERSION)
    (OUT / 'install-linux.sh').write_text(installer, encoding='utf-8', newline='\n')
    hashes = [f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}' for p in sorted(OUT.iterdir()) if p.is_file() and p.name not in ('SHA256SUMS', 'release-notes.md')]
    (OUT / 'SHA256SUMS').write_text('\n'.join(hashes) + '\n', encoding='utf-8', newline='\n')
    rows = ['| Platform / 平台 | Download / 下載 |', '| --- | --- |']
    for label, platform, suffix in [('macOS 12+ Apple Silicon', 'macos-arm64', '.dmg'), ('macOS 12+ Intel', 'macos-x64', '.dmg'), ('Windows 10/11 x64 (recommended)', 'windows-x64', '.exe'), ('Windows x64 MSI', 'windows-x64', '.msi'), ('Ubuntu 22.04+ / Debian 12+ x64', 'linux-x64', '.deb'), ('Fedora-family x64', 'linux-x64', '.rpm'), ('Linux x64 portable', 'linux-x64', '.AppImage')]:
        name = asset(platform, suffix)
        rows.append(f'| {label} | [{suffix[1:]}]({base}{quote(name)}) |')
    rows.append(f'\n[Linux guided install / 自動補齊相依套件]({base}install-linux.sh) · [SHA256SUMS]({base}SHA256SUMS)')
    notes = (ROOT / 'docs/RELEASE_NOTES.md').read_text(encoding='utf-8').replace('@VERSION@', VERSION).replace('@DOWNLOADS@', '\n'.join(rows))
    (OUT / 'release-notes.md').write_text(notes, encoding='utf-8', newline='\n')
    print(f'Validated {len(hashes)} assets and four updater targets')

def draft():
    prepare()
    existing = subprocess.run(['gh', 'release', 'view', TAG, '--repo', REPO], capture_output=True)
    if existing.returncode == 0:
        raise RuntimeError('Release already exists; inspect it instead of overwriting')
    sha = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    subprocess.run(['gh', 'release', 'create', TAG, '--repo', REPO, '--target', sha, '--title', f'🐈 Bastet Agent Sync {TAG}', '--draft', '--notes-file', str(OUT / 'release-notes.md'), *[str(p) for p in sorted(OUT.iterdir()) if p.name != 'release-notes.md']], check=True)
if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('command', choices=['collect', 'prepare', 'draft'])
    parser.add_argument('--target', choices=TARGETS)
    args = parser.parse_args()
    if args.command == 'collect':
        if not args.target:
            parser.error('--target is required')
        collect(args.target)
    elif args.command == 'prepare':
        prepare()
    else:
        draft()
