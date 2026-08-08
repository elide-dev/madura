#!/usr/bin/env bash
# Assemble the hermetic madura distribution: <root>/{bin,lib}.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
profile="${1:-release}"
dist="$repo/target/dist"

cargo build \
    --profile "$profile" \
    -p madura

rm -rf "$dist"
mkdir -p "$dist/bin" "$dist/lib"
cp "$repo/target/$profile/madura" "$dist/bin/madura"
cp "$repo/target/lib/libmadura-javac.so" "$dist/lib/"
cp "$repo/target/lib/modules" "$dist/lib/"
cp "$repo/target/lib/ct.sym" "$dist/lib/"

cp -fr "$repo/target/dist" "$repo/target/madura-linux-amd64"
pushd "$repo/target"
tar -cf madura-linux-amd64.tar madura-linux-amd64
gzip --best -k -v madura-linux-amd64.tar
xz --best --extreme -k -v madura-linux-amd64.tar
popd

echo "----------"
du -h \
    ./target/dist/lib/modules \
    ./target/dist/lib/*.so \
    ./target/dist/bin/madura \
    ./target/madura-linux-amd64.tar.gz \
    ./target/madura-linux-amd64.tar.xz

echo "dist assembled at $dist"
