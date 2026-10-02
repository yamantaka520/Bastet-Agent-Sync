#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"

require() {
  local name
  for name in "$@"; do
    if [[ -z "${!name:-}" ]]; then
      echo "Missing Store signing input: $name" >&2
      exit 1
    fi
  done
}

require BASTET_STORE_PROFILE BASTET_STORE_APP_IDENTITY BASTET_STORE_INSTALLER_IDENTITY
[[ "$(uname -s)" == Darwin ]] || { echo 'A macOS build host is required' >&2; exit 1; }
[[ "$BASTET_STORE_APP_IDENTITY" == 'Apple Distribution:'* || "$BASTET_STORE_APP_IDENTITY" == '3rd Party Mac Developer Application:'* ]] || {
  echo 'An App Store application distribution identity is required' >&2; exit 1;
}
[[ "$BASTET_STORE_INSTALLER_IDENTITY" == 'Mac Installer Distribution:'* || "$BASTET_STORE_INSTALLER_IDENTITY" == '3rd Party Mac Developer Installer:'* ]] || {
  echo 'A Mac Installer Distribution identity is required' >&2; exit 1;
}

prior_keychains=()
if [[ -n "${BASTET_STORE_KEYCHAIN:-}" ]]; then
  [[ "$BASTET_STORE_KEYCHAIN" = /* && -f "$BASTET_STORE_KEYCHAIN" ]] || { echo 'BASTET_STORE_KEYCHAIN must name an existing absolute keychain path' >&2; exit 1; }
  while IFS= read -r item; do
    item="${item#*\"}"
    item="${item%\"*}"
    [[ -n "$item" ]] && prior_keychains+=("$item")
  done < <(security list-keychains -d user)
  restore_keychains() {
    if [[ ${#prior_keychains[@]} -gt 0 ]]; then
      security list-keychains -d user -s "${prior_keychains[@]}" >/dev/null || true
    fi
  }
  trap restore_keychains EXIT
  security list-keychains -d user -s "$BASTET_STORE_KEYCHAIN" "${prior_keychains[@]}" >/dev/null
fi

umask 077
state="${BASTET_STORE_OUTPUT_DIR:-$repo/.signing/app-store}"
mkdir -p "$state"
state="$(cd "$state" && pwd)"
[[ "$state" != "$repo" && "$state" != "$repo/src-tauri" ]] || { echo 'Unsafe Store output location' >&2; exit 1; }
[[ ! -L "$state" ]] || { echo 'Store output directory cannot be a symlink' >&2; exit 1; }
export CARGO_TARGET_DIR="$state/cargo-target"
python3 scripts/mac-app-store-prep.py --profile "$BASTET_STORE_PROFILE" --output "$state/prepared"

# Bind the requested signing identity to a certificate inside the decoded profile.
python3 - "$state/prepared/profile-cert-sha1.txt" <<'PY'
import os, re, subprocess, sys
allowed = set(open(sys.argv[1], encoding='ascii').read().split())
identity = os.environ['BASTET_STORE_APP_IDENTITY']
command = ['security', 'find-identity', '-v', '-p', 'codesigning']
if os.environ.get('BASTET_STORE_KEYCHAIN'):
    command.append(os.environ['BASTET_STORE_KEYCHAIN'])
listing = subprocess.run(command, check=True, capture_output=True, text=True).stdout
matches = [(fingerprint, name) for fingerprint, name in re.findall(r'\b([A-F0-9]{40})\s+"([^"]+)"', listing)]
if not any(fingerprint in allowed and name == identity for fingerprint, name in matches):
    raise SystemExit('Requested application identity is unavailable or absent from the profile')
PY

python3 - "$state/prepared/tauri.profile.conf.json" <<'PY'
import json, os, sys
path = sys.argv[1]
with open(path, encoding='utf-8') as source:
    config = json.load(source)
config['bundle']['macOS']['signingIdentity'] = os.environ['BASTET_STORE_APP_IDENTITY']
with open(path, 'w', encoding='utf-8') as output:
    json.dump(config, output, indent=2)
PY

# Store artifacts never share Cargo output with Developer ID releases.
unset TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD
# App Store packages are processed by Apple after upload, not Developer ID notarization.
unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH
unset APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD
export APPLE_SIGNING_IDENTITY="$BASTET_STORE_APP_IDENTITY"
npm run tauri build -- --ci --bundles app --target universal-apple-darwin --features mac-app-store \
  --config src-tauri/tauri.appstore.conf.json --config "$state/prepared/tauri.profile.conf.json"

app="$CARGO_TARGET_DIR/universal-apple-darwin/release/bundle/macos/Bastet Agent Sync.app"
[[ -d "$app" && -s "$app/Contents/embedded.provisionprofile" ]] || { echo 'Signed app or embedded profile is missing' >&2; exit 1; }
cmp -s "$BASTET_STORE_PROFILE" "$app/Contents/embedded.provisionprofile" || { echo 'Embedded profile differs from input' >&2; exit 1; }
codesign --verify --deep --strict "$app"
python3 - "$app" "$state/prepared/Entitlements.plist" <<'PY'
import os, plistlib, subprocess, sys
from pathlib import Path
app, required = sys.argv[1:]
info = plistlib.loads((Path(app) / 'Contents/Info.plist').read_bytes())
if info.get('CFBundleIdentifier') != 'tw.bastet.agent-sync':
    raise SystemExit('Signed app bundle identifier mismatch')
signature = subprocess.run(['codesign', '-dv', '--verbose=4', app], check=True, capture_output=True, text=True).stderr
if f"Authority={os.environ['BASTET_STORE_APP_IDENTITY']}" not in signature:
    raise SystemExit('Signed app authority mismatch')
expected = plistlib.loads(Path(required).read_bytes())
if f"TeamIdentifier={expected['com.apple.developer.team-identifier']}" not in signature:
    raise SystemExit('Signed app Team ID mismatch')
actual = plistlib.loads(subprocess.run(['codesign', '-d', '--entitlements', '-', '--xml', app], check=True, capture_output=True).stdout)
for key, value in expected.items():
    if actual.get(key) != value:
        raise SystemExit(f'Signed app entitlement mismatch: {key}')
binary = Path(app) / 'Contents/MacOS' / info['CFBundleExecutable']
archs = set(subprocess.run(['lipo', '-archs', str(binary)], check=True, capture_output=True, text=True).stdout.split())
if archs != {'arm64', 'x86_64'}:
    raise SystemExit('Signed app is not universal arm64/x86_64')
PY

pkg="$state/Bastet Agent Sync.pkg"
keychain_args=()
if [[ -n "${BASTET_STORE_KEYCHAIN:-}" ]]; then keychain_args=(--keychain "$BASTET_STORE_KEYCHAIN"); fi
xcrun productbuild --sign "$BASTET_STORE_INSTALLER_IDENTITY" "${keychain_args[@]}" --component "$app" /Applications "$pkg"
pkgutil --check-signature "$pkg" > "$state/pkg-signature.txt"
python3 - "$state/pkg-signature.txt" <<'PY'
from pathlib import Path
import os, sys
signature = Path(sys.argv[1]).read_text(encoding='utf-8')
# Store distribution certificates are reported as Apple-issued (Development)
# by pkgutil; direct-install Gatekeeper acceptance is a different channel.
statuses = (
    'Status: signed by a certificate trusted by macOS',
    'Status: signed by a certificate trusted by Mac OS X',
    'Status: signed by a developer certificate issued by Apple (Development)',
)
if not any(status in signature for status in statuses) or 'Apple Root CA' not in signature:
    raise SystemExit('Installer signature does not have an accepted Apple chain')
if os.environ['BASTET_STORE_INSTALLER_IDENTITY'] not in signature:
    raise SystemExit('Installer signing identity mismatch')
PY
echo "Store candidate built and locally checked: $pkg"
