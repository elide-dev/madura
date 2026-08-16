#!/usr/bin/env bash
# Usage: setup-cosmocc.sh <install-dir>
set -euo pipefail

dest="${1:?usage: setup-cosmocc.sh <install-dir>}"

checksum="7b3c11802791037aa5d7d1c02c9f54b21b25ba6369e1f1cc222aca9698641a97"
url="https://static.elideusercontent.com/cosmogvm/25.2i-r2/cosmocc.zip"
archive="$(mktemp -t cosmocc-XXXXXX.zip)"
trap 'rm -f "$archive"' EXIT

echo "setup-cosmocc: fetching $url" >&2
curl -fsSL --retry 3 --retry-delay 5 -o "$archive" "$url"

# `sha256sum` is coreutils; macOS ships BSD `shasum` instead.
if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$archive" | cut -d' ' -f1)"
else
    actual="$(shasum -a 256 "$archive" | cut -d' ' -f1)"
fi
if [ "$actual" != "$checksum" ]; then
    echo "setup-cosmocc: checksum mismatch" >&2
    echo "  expected $checksum" >&2
    echo "  actual   $actual" >&2
    exit 1
fi

rm -rf "$dest"
mkdir -p "$dest"
unzip "$archive" -d "$dest"

home="$dest/cosmocc"
echo "setup-cosmocc: at $home" >&2
echo "$home"
