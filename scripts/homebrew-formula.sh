#!/usr/bin/env bash
# Render the Homebrew formula for a madura release to stdout.
#
#   scripts/homebrew-formula.sh <version> <sha256> [url]
#
# The formula is generated rather than checked in so it cannot drift from the
# release it describes: the version, the URL and the checksum all come from the
# same release job that uploaded the asset. `job.release.yml` renders it and
# pushes the result to the tap at elide-dev/homebrew-elide.
#
# The third argument overrides the download URL, which is only useful for
# testing the formula against a local artifact before a release exists:
#
#   scripts/homebrew-formula.sh 1.2.0 "$(shasum -a 256 dist.zip | cut -d' ' -f1)" \
#       "file://$PWD/dist.zip" > madura.rb
#   brew install --formula ./madura.rb && brew test ./madura.rb
#
# One formula covers every platform because there is one asset: the release ships
# an Actually Portable Executable, so the same `madura.com` runs on macOS and
# Linux, x86_64 and arm64. That is also why the formula has no `on_macos` /
# `on_linux` split and no per-arch checksums.
set -euo pipefail

version="${1:?usage: homebrew-formula.sh <version> <sha256> [url]}"
sha256="${2:?usage: homebrew-formula.sh <version> <sha256> [url]}"
url="${3:-https://github.com/elide-dev/madura/releases/download/v${version}/madura-${version}-cosmo-universal.zip}"

if ! printf '%s' "$sha256" | grep -Eq '^[0-9a-f]{64}$'; then
    echo "homebrew-formula: not a sha256 digest: $sha256" >&2
    exit 1
fi

cat <<FORMULA
class Madura < Formula
  desc "Smallest possible compliant Java toolchain"
  homepage "https://github.com/elide-dev/madura"
  url "${url}"
  version "${version}"
  sha256 "${sha256}"
  # madura's own code is 0BSD; the binary is an ahead-of-time compilation of
  # OpenJDK's javac, which stays GPL-2.0 with the Classpath Exception. See
  # NOTICE.md in the repository.
  license all_of: [
    "0BSD",
    { "GPL-2.0-only" => { with: "Classpath-exception-2.0" } },
  ]

  livecheck do
    url :stable
    strategy :github_latest
  end

  def install
    # The distribution is a unit, not a loose binary: madura resolves its
    # platform metadata binary-relative (<exe>/../lib/modules, with the
    # executable path resolved through symlinks), so madura.com and lib/ have to
    # stay siblings. Hence libexec plus a symlink, rather than bin.install.
    libexec.install Dir["*"]
    bin.install_symlink libexec/"madura.com" => "madura"
  end

  test do
    (testpath/"Hello.java").write <<~JAVA
      public class Hello {
        public static void main(String[] args) {
          System.out.println("hello from madura");
        }
      }
    JAVA

    # check analyzes and writes nothing; compile emits bytecode.
    system bin/"madura", "check", testpath/"Hello.java"
    refute_path_exists testpath/"Hello.class"

    system bin/"madura", "compile", testpath/"Hello.java", "-d", testpath/"out"
    assert_path_exists testpath/"out/Hello.class"

    assert_match "javac", shell_output("#{bin}/madura --version 2>&1")
  end
end
FORMULA
