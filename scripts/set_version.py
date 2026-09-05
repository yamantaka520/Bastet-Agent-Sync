"""Set the release version in every package source and lockfile."""
from pathlib import Path
import json, re, sys
root = Path(__file__).resolve().parents[1]
version = sys.argv[1] if len(sys.argv) == 2 else ''
if not re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', version):
    raise SystemExit('Usage: python scripts/set_version.py MAJOR.MINOR.PATCH')
for name in ['package.json', 'package-lock.json', 'src-tauri/tauri.conf.json']:
    p=root/name; data=json.loads(p.read_text(encoding='utf-8')); data['version']=version
    if 'packages' in data: data['packages']['']['version']=version
    p.write_text(json.dumps(data,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
for name in ['src-tauri/Cargo.toml','src-tauri/Cargo.lock']:
    p=root/name; text=p.read_text(encoding='utf-8')
    text,count=re.subn(r'(name = "bastet-agent-sync"\nversion = ")[^"]+',lambda m:m[1]+version,text,count=1)
    if count!=1: raise SystemExit('Missing Rust package version')
    p.write_text(text,encoding='utf-8')
print('Updated package versions to '+version)
