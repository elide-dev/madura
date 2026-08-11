#!/usr/bin/env bash
# Install the GraalVM that builds the javac image, printing its home on stdout.
#
# Not `graalvm/setup-graalvm`: that action serves the 25.0.x line, whose
# `JDKInitializationFeature` forces `jdk.internal.jrtfs` to build-time
# initialization. `SystemImage.RUNTIME_HOME` is then resolved from the builder's
# `java.home` and frozen into the image, so javac reads the *build machine's*
# lib/modules — and the resulting binary only works on hosts where that path
# happens to exist. The 25.2.x line does not, and it is what `elide.pkl` targets
# (`truffleTarget`). Oracle publishes it on GDS but not through the action.
#
# Usage: setup-graalvm.sh <install-dir>
set -euo pipefail

dest="${1:?usage: setup-graalvm.sh <install-dir>}"

# GDS serves feature lines rather than point releases — there is no
# version-pinned URL — so the checksum is the pin. A new upstream build in this
# line changes it and fails here rather than silently swapping the toolchain;
# refresh the affected hash when that happens.
line="25i2"
expect_version="25.2.4"

# Where the JDK home sits inside the extracted tree. Linux unpacks it at the
# root; macOS ships a framework bundle, so the home is nested under Contents.
home_suffix=""

case "$(uname -s)/$(uname -m)" in
    Linux/x86_64)
        platform="linux-x64"
        checksum="7100d99cbfec68b03b669cc60c7e8592bbcda1732e8eaebc460fe0b75849a894"
        ;;
    Linux/aarch64 | Linux/arm64)
        platform="linux-aarch64"
        checksum="0bc65f9c36ae77bd83aad46a2b4de4b0ec97da1b4ac83fedb59e19f868873dee"
        ;;
    Darwin/arm64)
        platform="macos-aarch64"
        checksum="1b5937aa3076707459cfc815a1699761f943d2d1c9cbe03388e36d5e47eb27c3"
        home_suffix="/Contents/Home"
        ;;
    *)
        echo "setup-graalvm: unsupported platform $(uname -s)/$(uname -m)" >&2
        exit 1
        ;;
esac

url="https://gds.oracle.com/download/graal/$line/latest/graalvm-jdk-$line-25_${platform}_bin.tar.gz"
archive="$(mktemp -t graalvm-XXXXXX.tar.gz)"
trap 'rm -f "$archive"' EXIT

echo "setup-graalvm: fetching $url" >&2
curl -fsSL --retry 3 --retry-delay 5 -o "$archive" "$url"

# `sha256sum` is coreutils; macOS ships BSD `shasum` instead.
if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$archive" | cut -d' ' -f1)"
else
    actual="$(shasum -a 256 "$archive" | cut -d' ' -f1)"
fi
if [ "$actual" != "$checksum" ]; then
    echo "setup-graalvm: checksum mismatch for $platform" >&2
    echo "  expected $checksum" >&2
    echo "  actual   $actual" >&2
    echo "  (a new $line build was published; verify it and update this script)" >&2
    exit 1
fi

rm -rf "$dest"
mkdir -p "$dest"
tar -xzf "$archive" -C "$dest" --strip-components=1

# The install dir is what callers hide away to prove hermeticity; the home is
# what goes on PATH. They differ only on macOS.
home="$dest$home_suffix"

# Guard against the line silently moving: the whole point is *which* GraalVM
# builds the image, so a mismatch here has to be loud.
version="$(sed -n 's/^GRAALVM_VERSION="\(.*\)"$/\1/p' "$home/release")"
if [ "$version" != "$expect_version" ]; then
    echo "setup-graalvm: expected GraalVM $expect_version, got ${version:-<unknown>}" >&2
    exit 1
fi

echo "setup-graalvm: GraalVM $version at $home" >&2
echo "$home"
