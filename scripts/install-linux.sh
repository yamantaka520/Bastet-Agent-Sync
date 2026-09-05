#!/bin/sh
# Download this file, inspect it, then run: sh install-linux.sh
set -eu
VERSION='@VERSION@'
BASE="https://github.com/yamantaka520/Bastet-Agent-Sync/releases/download/v$VERSION"
[ "$(uname -s)" = Linux ] || { echo 'Linux required.' >&2; exit 1; }
[ "$(uname -m)" = x86_64 ] || { echo 'This release requires x86_64.' >&2; exit 1; }
if command -v apt-get >/dev/null 2>&1; then
    manager=apt-get; extension=deb
elif command -v dnf >/dev/null 2>&1; then
    manager=dnf; extension=rpm
else
    echo 'Supported package managers: apt-get or dnf. Use the AppImage on other systems.' >&2; exit 1
fi
as_root() {
    if [ "$(id -u)" = 0 ]; then "$@"; else sudo "$@"; fi
}
echo "Bastet Agent Sync $VERSION — $manager will install missing runtime dependencies."
echo '正在安裝必要元件 / 正在安装必要组件 / Installing dependencies / 必要な依存関係をインストール / 필수 구성 요소 설치'
if ! command -v curl >/dev/null 2>&1; then
    if [ "$manager" = apt-get ]; then as_root apt-get update; fi
    as_root "$manager" install -y curl ca-certificates
fi
command -v sha256sum >/dev/null 2>&1 || { echo 'sha256sum is required.' >&2; exit 1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
package="Bastet-Agent-Sync-$VERSION-linux-x64.$extension"
curl --fail --location --proto '=https' --tlsv1.2 "$BASE/SHA256SUMS" -o "$work/SHA256SUMS"
curl --fail --location --proto '=https' --tlsv1.2 "$BASE/$package" -o "$work/$package"
cd "$work"
awk -v name="$package" '$2 == name {print}' SHA256SUMS > selected.sha256
[ "$(wc -l < selected.sha256 | tr -d ' ')" = 1 ] || { echo 'Missing checksum.' >&2; exit 1; }
sha256sum -c selected.sha256
if [ "$manager" = apt-get ]; then as_root apt-get update; fi
as_root "$manager" install -y "$work/$package"
echo 'Installed. Open Bastet Agent Sync from the application menu.'
echo '安裝完成 / 安装完成 / インストール完了 / 설치 완료'
