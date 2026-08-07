#!/usr/bin/env bash
# Assemble the hermetic madura distribution: <root>/{bin,lib}.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
profile="${1:-release}"
dist="$repo/target/dist"

cargo build --profile "$profile" -p madura

rm -rf "$dist"
mkdir -p "$dist/bin" "$dist/lib"
cp "$repo/target/$profile/madura" "$dist/bin/madura"
cp "$repo/target/lib/libmadura-javac.so" "$dist/lib/"
cp "$repo/target/lib/modules" "$dist/lib/"
cp "$repo/target/lib/ct.sym" "$dist/lib/"

echo "dist assembled at $dist"
