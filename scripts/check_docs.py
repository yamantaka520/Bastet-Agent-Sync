"""Check public documentation links and accidental machine-specific paths."""
from pathlib import Path
import re
root = Path(__file__).resolve().parents[1]
files = [p for p in root.rglob('*.md') if not any(x in p.parts for x in ('node_modules', 'target', '.git'))]
errors=[]
for path in files:
    text=path.read_text(encoding="utf-8")
    if re.search(r'/Users/[^/\s`]+/|/home/(?!user[/\s])[^/\s`]+/',text): errors.append(f'{path.relative_to(root)}: personal path')
    for link in re.findall(r'\]\(([^)]+)\)',text):
        if '://' in link or link.startswith('#'): continue
        target=(path.parent/link.split('#')[0]).resolve()
        if not target.exists(): errors.append(f'{path.relative_to(root)}: missing {link}')
for lang in ['en','zh-Hant','zh-Hans','ja','ko']:
    if not (root/f'docs/manual/{lang}/guide.md').exists(): errors.append(f'missing guide: {lang}')
if errors: raise SystemExit('\n'.join(errors))
print(f'{len(files)} Markdown files: local links and public paths checked')
