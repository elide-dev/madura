#!/usr/bin/env bash
# Assemble the hermetic madura distribution:
#
#   target/dist/madura          the native-image binary
#   target/dist/lib/modules     platform jimage (from jlink)
#   target/dist/lib/ct.sym      release-targeting signatures
#
# At runtime madura resolves its platform metadata binary-relative
# (`<exe>/../lib/modules`), so the binary and the `lib` dir are siblings under
# the distribution root.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist="$repo/target/dist"
image="$repo/.dev/artifacts/native-image/madura"
jdkroot="$repo/target/jdkroot"

# The distribution is a native binary plus platform metadata, so the tarball is
# named for the machine that produced it (Debian-style arch).
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
for f in modules ct.sym; do
    if [ ! -f "$jdkroot/lib/$f" ]; then
        echo "make-dist: missing $jdkroot/lib/$f (run \`make jdkroot\`)" >&2
        exit 1
    fi
done

rm -rf "$dist"
mkdir -p "$dist/lib"
cp "$image" "$dist/madura"
cp "$jdkroot/lib/modules" "$dist/lib/"
cp "$jdkroot/lib/ct.sym" "$dist/lib/"

rm -rf "$repo/target/${name:?}"
cp -fr "$dist" "$repo/target/$name"
pushd "$repo/target"
tar -cf "$name.tar" "$name"
gzip --best -k -v -f "$name.tar"
xz --best --extreme -k -v -f "$name.tar"
popd

echo "----------"
du -h \
    "$dist/madura" \
    "$dist/lib/modules" \
    "$dist/lib/ct.sym" \
    "$repo/target/$name.tar.gz" \
    "$repo/target/$name.tar.xz"

echo "dist assembled at $dist"
