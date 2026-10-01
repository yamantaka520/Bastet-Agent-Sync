#!/usr/bin/env bash
set -euo pipefail

state="${RUNNER_TEMP:?}/bastet-macos-signing"
keychain="$state/signing.keychain-db"
api_key="$state/AuthKey.p8"

require() {
  local name
  for name in "$@"; do
    if [[ -z "${!name:-}" ]]; then
      echo "Missing signing input: $name" >&2
      exit 1
    fi
  done
}

prepare() {
  require APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_API_PRIVATE_KEY APPLE_API_KEY APPLE_API_ISSUER APPLE_SIGNING_IDENTITY APPLE_TEAM_ID
  [[ "$APPLE_SIGNING_IDENTITY" == 'Developer ID Application:'* ]] || { echo 'A Developer ID Application identity is required' >&2; exit 1; }
  [[ "$APPLE_SIGNING_IDENTITY" == *"($APPLE_TEAM_ID)" ]] || { echo 'Signing identity and team do not match' >&2; exit 1; }
  umask 077
  mkdir -p "$state"
  security list-keychains -d user > "$state/original-keychains"
  security default-keychain -d user > "$state/original-default-keychain"
  printf '%s' "$APPLE_CERTIFICATE" | base64 -D > "$state/certificate.p12"
  printf '%s' "$APPLE_API_PRIVATE_KEY" > "$api_key"
  openssl rand -hex 32 > "$state/keychain-password"
  local password
  password="$(<"$state/keychain-password")"
  security create-keychain -p "$password" "$keychain"
  security unlock-keychain -p "$password" "$keychain"
  security set-keychain-settings -lut 21600 "$keychain"
  security import "$state/certificate.p12" -k "$keychain" -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign > /dev/null
  security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$password" "$keychain" > /dev/null
  local previous=() item
  while IFS= read -r item; do
    item="${item#*\"}"
    item="${item%\"*}"
    [[ -n "$item" ]] && previous+=("$item")
  done < "$state/original-keychains"
  security list-keychains -d user -s "$keychain" "${previous[@]}"
  security default-keychain -d user -s "$keychain"
  local identities
  identities="$(security find-identity -v -p codesigning "$keychain")"
  grep -Fq "\"$APPLE_SIGNING_IDENTITY\"" <<< "$identities" || { echo 'Configured Developer ID identity is not in the imported certificate' >&2; exit 1; }
  python3 - "$state/tauri-signing.json" <<'PY'
import json
import os
import sys
identity = os.environ['APPLE_SIGNING_IDENTITY']
with open(sys.argv[1], 'w', encoding='utf-8') as out:
    json.dump({'bundle': {'macOS': {'signingIdentity': identity, 'hardenedRuntime': True}}}, out)
PY
  {
    printf 'APPLE_API_KEY_PATH=%s\n' "$api_key"
    printf 'BASTET_MACOS_SIGNING_CONFIG=%s\n' "$state/tauri-signing.json"
  } >> "$GITHUB_ENV"
  rm -f "$state/certificate.p12"
}

probe() {
  require APPLE_API_KEY APPLE_API_ISSUER APPLE_SIGNING_IDENTITY
  [[ -f "$api_key" && -f "$keychain" ]] || { echo 'Signing material was not prepared' >&2; exit 1; }
  # This authenticates to Apple's service before the expensive Rust build.
  xcrun notarytool history --key "$api_key" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER" --output-format json > "$state/notary-history.json"
  python3 - "$state/notary-history.json" <<'PY'
import json
import sys
with open(sys.argv[1], encoding='utf-8') as source:
    assert isinstance(json.load(source), dict), 'Invalid notarytool history response'
PY
  printf 'int main(void) { return 0; }\n' > "$state/probe.c"
  clang "$state/probe.c" -o "$state/probe"
  codesign --force --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$state/probe" > /dev/null
  codesign --verify --strict "$state/probe"
  local info
  info="$(codesign -dv --verbose=4 "$state/probe" 2>&1)"
  grep -Fxq "TeamIdentifier=$APPLE_TEAM_ID" <<< "$info" || { echo 'Probe team mismatch' >&2; exit 1; }
  grep -Eq '^CodeDirectory .*flags=0x[[:xdigit:]]+\(([^)]*,)?runtime(,[^)]*)?\)' <<< "$info" || { echo 'Probe lacks hardened runtime' >&2; exit 1; }
  grep -Eq '^Timestamp=' <<< "$info" || { echo 'Probe lacks secure timestamp' >&2; exit 1; }
  rm -f "$state/probe" "$state/probe.c" "$state/notary-history.json"
}

verify_app() {
  local app="$1" arch="$2" info actual
  [[ -d "$app" ]] || { echo 'Application bundle missing' >&2; exit 1; }
  codesign --verify --deep --strict --verbose=2 "$app"
  info="$(codesign -dv --verbose=4 "$app" 2>&1)"
  grep -Eq '^Authority=Developer ID Application:' <<< "$info" || { echo 'Application lacks Developer ID authority' >&2; exit 1; }
  grep -Fxq "TeamIdentifier=$APPLE_TEAM_ID" <<< "$info" || { echo 'Application team mismatch' >&2; exit 1; }
  grep -Eq '^Timestamp=' <<< "$info" || { echo 'Application lacks secure timestamp' >&2; exit 1; }
  grep -Eq '^CodeDirectory .*flags=0x[[:xdigit:]]+\(([^)]*,)?runtime(,[^)]*)?\)' <<< "$info" || { echo 'Application lacks hardened runtime' >&2; exit 1; }
  actual="$(lipo -archs "$app/Contents/MacOS/bastet-agent-sync")"
  [[ "$actual" == "$arch" ]] || { echo "Application architecture mismatch: $actual" >&2; exit 1; }
  xcrun stapler validate "$app"
  spctl --assess --type execute --verbose=2 "$app"
}

finish() {
  require APPLE_API_KEY APPLE_API_ISSUER APPLE_TEAM_ID
  local target="$1" arch="$2" base app dmg archive signature mount extracted info status
  base="src-tauri/target/$target/release/bundle"
  app="$base/macos/Bastet Agent Sync.app"
  shopt -s nullglob
  local dmgs=("$base"/dmg/*.dmg)
  local archives=("$base"/macos/*.app.tar.gz)
  local signatures=("$base"/macos/*.app.tar.gz.sig)
  [[ ${#dmgs[@]} -eq 1 && ${#archives[@]} -eq 1 && ${#signatures[@]} -eq 1 ]] || { echo 'Expected one DMG, updater archive and signature' >&2; exit 1; }
  dmg="${dmgs[0]}"; archive="${archives[0]}"; signature="${signatures[0]}"
  [[ -s "$signature" ]] || { echo 'Updater signature is empty' >&2; exit 1; }
  verify_app "$app" "$arch"
  hdiutil verify "$dmg"
  codesign --verify --strict --verbose=2 "$dmg"
  info="$(codesign -dv --verbose=4 "$dmg" 2>&1)"
  grep -Eq '^Authority=Developer ID Application:' <<< "$info" || { echo 'DMG lacks Developer ID authority' >&2; exit 1; }
  grep -Fxq "TeamIdentifier=$APPLE_TEAM_ID" <<< "$info" || { echo 'DMG team mismatch' >&2; exit 1; }
  grep -Eq '^Timestamp=' <<< "$info" || { echo 'DMG lacks secure timestamp' >&2; exit 1; }
  xcrun notarytool submit "$dmg" --key "$api_key" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER" --wait --output-format json > "$state/dmg-notary-result.json"
  status="$(python3 - "$state/dmg-notary-result.json" <<'PY'
import json
import sys
with open(sys.argv[1], encoding='utf-8') as source:
    print(json.load(source).get('status', ''))
PY
)"
  [[ "$status" == Accepted ]] || { echo "DMG notarization status: $status" >&2; exit 1; }
  python3 - "$state/dmg-notary-result.json" <<'PY'
import json
import sys
with open(sys.argv[1], encoding='utf-8') as source:
    result = json.load(source)
print(f"DMG notarization accepted: {result.get('id', 'no submission id')}")
PY
  xcrun stapler staple "$dmg"
  codesign --verify --strict --verbose=2 "$dmg"
  xcrun stapler validate "$dmg"
  spctl --assess --type open --context context:primary-signature --verbose=2 "$dmg"
  mount="$state/mounted-dmg"
  mkdir -p "$mount"
  hdiutil attach -readonly -nobrowse -mountpoint "$mount" "$dmg" > /dev/null
  verify_app "$mount/Bastet Agent Sync.app" "$arch"
  hdiutil detach "$mount" > /dev/null
  mkdir -p "$state/updater"
  tar -xzf "$archive" -C "$state/updater"
  extracted="$state/updater/Bastet Agent Sync.app"
  verify_app "$extracted" "$arch"
  [[ "$(shasum -a 256 "$app/Contents/MacOS/bastet-agent-sync" | cut -d ' ' -f 1)" == "$(shasum -a 256 "$extracted/Contents/MacOS/bastet-agent-sync" | cut -d ' ' -f 1)" ]] || { echo 'Updater executable differs from notarized app' >&2; exit 1; }
  echo 'App, DMG and updater archive passed Developer ID, notarization and Gatekeeper checks'
}

cleanup() {
  hdiutil detach "$state/mounted-dmg" > /dev/null 2>&1 || true
  if [[ -f "$state/original-keychains" ]]; then
    local previous=() item
    while IFS= read -r item; do
      item="${item#*\"}"
      item="${item%\"*}"
      [[ -n "$item" ]] && previous+=("$item")
    done < "$state/original-keychains"
    if [[ ${#previous[@]} -gt 0 ]]; then security list-keychains -d user -s "${previous[@]}" || true; fi
  fi
  if [[ -f "$state/original-default-keychain" ]]; then
    local default
    default="$(<"$state/original-default-keychain")"
    default="${default#*\"}"
    default="${default%\"*}"
    if [[ -n "$default" ]]; then security default-keychain -d user -s "$default" || true; fi
  fi
  security delete-keychain "$keychain" 2>/dev/null || true
  rm -rf "$state"
}

case "${1:-}" in
  prepare) prepare ;;
  probe) probe ;;
  finish) [[ $# -eq 3 ]] || exit 2; finish "$2" "$3" ;;
  cleanup) cleanup ;;
  *) echo 'Usage: macos-signing.sh prepare|probe|finish TARGET ARCH|cleanup' >&2; exit 2 ;;
esac
