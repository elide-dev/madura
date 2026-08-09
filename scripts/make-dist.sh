#!/usr/bin/env bash
# Assemble the hermetic madura distribution: <root>/{bin,lib}.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
profile="${1:-release}"
dist="$repo/target/dist"

# The distribution carries a platform image and a native shared library, so the
# tarball is named for the machine that produced it rather than assumed.
case "$(uname -m)" in
    x86_64) arch="amd64" ;;
    aarch64 | arm64) arch="arm64" ;;
    *)
        echo "make-dist: unsupported architecture $(uname -m)" >&2
        exit 1
        ;;
esac
case "$(uname -s)" in
    Linux) os="linux" libext="so" ;;
    Darwin) os="darwin" libext="dylib" ;;
    *)
        echo "make-dist: unsupported operating system $(uname -s)" >&2
        exit 1
        ;;
esac
name="madura-$os-$arch"

cargo build \
    --profile "$profile" \
    -p madura

rm -rf "$dist"
mkdir -p "$dist/bin" "$dist/lib"
cp "$repo/target/$profile/madura" "$dist/bin/madura"
cp "$repo/target/lib/libmadura-javac.$libext" "$dist/lib/"
cp "$repo/target/lib/modules" "$dist/lib/"
cp "$repo/target/lib/ct.sym" "$dist/lib/"

rm -rf "$repo/target/${name:?}"
cp -fr "$repo/target/dist" "$repo/target/$name"
pushd "$repo/target"
tar -cf "$name.tar" "$name"
gzip --best -k -v -f "$name.tar"
xz --best --extreme -k -v -f "$name.tar"
popd

echo "----------"
du -h \
    "$dist/lib/modules" \
    "$dist"/lib/*."$libext" \
    "$dist/bin/madura" \
    "$repo/target/$name.tar.gz" \
    "$repo/target/$name.tar.xz"

echo "dist assembled at $dist"
