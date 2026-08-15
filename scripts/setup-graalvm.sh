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
expect_version="25.2.4-dev"

# Where the JDK home sits inside the extracted tree. Linux unpacks it at the
# root; macOS ships a framework bundle, so the home is nested under Contents.
home_suffix=""

platform="linux-x64"
checksum="294c31d8998fc1d672bd038a6276614abd17ef88e1c40cec793574c7dd3af144"

url="https://static.elideusercontent.com/cosmogvm/25.2i/graalvm-ce-25.2.4-dev-linux-x86_64.tar.gz"
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
