#!/usr/bin/env bash
# Assemble the hermetic madura distribution:
#
#   target/dist/madura                 the native-image binary
#   target/dist/<os.arch>/lib/modules  platform jimage (from jlink)
#   target/dist/<os.arch>/lib/ct.sym   release-targeting signatures
#
# At runtime madura resolves its platform metadata binary-relative
# (`<exe>/../<os.arch>/lib/modules`), so the binary and the `<os.arch>` dir must
# be siblings under the distribution root. `<os.arch>` is the JVM `os.arch`
# value the binary reports — `amd64` on x86-64, `aarch64` on ARM64.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist="$repo/target/dist"
image="$repo/.dev/artifacts/native-image/madura"
jdkroot="$repo/target/jdkroot"

# Two arch tokens: `osarch` names the metadata directory the binary looks in
# (JVM `os.arch`); `pkgarch` names the release tarball (Debian-style).
case "$(uname -m)" in
    x86_64) pkgarch="amd64"; osarch="amd64" ;;
    aarch64 | arm64) pkgarch="arm64"; osarch="aarch64" ;;
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
name="madura-$os-$pkgarch"

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
mkdir -p "$dist/$osarch/lib"
cp "$image" "$dist/madura"
cp "$jdkroot/lib/modules" "$dist/$osarch/lib/"
cp "$jdkroot/lib/ct.sym" "$dist/$osarch/lib/"

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
    "$dist/$osarch/lib/modules" \
    "$dist/$osarch/lib/ct.sym" \
    "$repo/target/$name.tar.gz" \
    "$repo/target/$name.tar.xz"

echo "dist assembled at $dist"
