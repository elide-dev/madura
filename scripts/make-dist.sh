#!/usr/bin/env bash
# Assemble the madura distribution: a single native-image binary at
# <root>/bin/madura. Platform metadata (lib/modules, lib/ct.sym) is read from
# the caller's $JAVA_HOME at runtime, so nothing else ships.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist="$repo/target/dist"
image="$repo/.dev/artifacts/native-image/madura"

# The distribution is a native binary, so the tarball is named for the machine
# that produced it rather than assumed.
case "$(uname -m)" in
    x86_64) arch="amd64" ;;
    aarch64 | arm64) arch="arm64" ;;
    *)
        echo "make-dist: unsupported architecture $(uname -m)" >&2
        exit 1
        ;;
esac
case "$(uname -s)" in
    Linux) os="linux" ;;
    Darwin) os="darwin" ;;
    *)
        echo "make-dist: unsupported operating system $(uname -s)" >&2
        exit 1
        ;;
esac
name="madura-$os-$arch"

if [ ! -x "$image" ]; then
    echo "make-dist: native image not found at $image (run \`make image\`)" >&2
    exit 1
fi

rm -rf "$dist"
mkdir -p "$dist/bin"
cp "$image" "$dist/bin/madura"

rm -rf "$repo/target/${name:?}"
cp -fr "$dist" "$repo/target/$name"
pushd "$repo/target"
tar -cf "$name.tar" "$name"
gzip --best -k -v -f "$name.tar"
xz --best --extreme -k -v -f "$name.tar"
popd

echo "----------"
du -h \
    "$dist/bin/madura" \
    "$repo/target/$name.tar.gz" \
    "$repo/target/$name.tar.xz"

echo "dist assembled at $dist"
