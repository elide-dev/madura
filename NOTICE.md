# Notices

`madura`'s own source code — everything in this repository — is licensed under the
[BSD Zero Clause License](./LICENSE) (SPDX: `0BSD`). 0BSD is public-domain-equivalent: copy it,
ship it, sell it, no attribution required, no notice to retain.

**The binaries `madura` ships are a different matter, and this file is the reason.** A `madura`
distribution is `javac` — OpenJDK code — compiled ahead-of-time and packaged beside OpenJDK
platform metadata. Those parts stay under their own licenses no matter what license this
repository carries, and the terms below travel with every tarball, container image, and bottle.

## What is in a distribution

A release archive contains exactly two things:

| Path                        | What it is                                          | License                                        |
| --------------------------- | --------------------------------------------------- | ---------------------------------------------- |
| `madura` (or `madura.com`)  | native image of `jdk.compiler` + this repo's entrypoint | `GPL-2.0-only WITH Classpath-exception-2.0 AND 0BSD AND Apache-2.0 AND ISC` (see below) |
| `lib/modules`, `lib/ct.sym` | `jlink`'d OpenJDK platform metadata (the jimage and release signatures) | GPL-2.0-only WITH Classpath-exception-2.0      |

## Third-party components

### OpenJDK

`madura` **is** `javac`: the shipped binary is an ahead-of-time compilation of `jdk.compiler`
and `java.base` from OpenJDK, and `lib/modules` / `lib/ct.sym` are produced by `jlink` from an
OpenJDK 25 build. These portions are licensed under the **GNU General Public License, version 2,
with the Classpath Exception** (SPDX: `GPL-2.0-only WITH Classpath-exception-2.0`).

- Upstream source: <https://github.com/openjdk/jdk> (JDK 25)
- The class libraries linked into the image come from GraalVM's JDK
  ([`oracle/graal` labs-openjdk](https://github.com/graalvm/labs-openjdk)), also
  GPL-2.0 with the Classpath Exception.

The Classpath Exception is what makes this project possible: it permits linking independent
modules — this repository's entrypoint — with the OpenJDK code and distributing the combination
under terms of your choosing, provided the OpenJDK portions themselves remain under GPL-2.0.

**Written offer.** Complete corresponding source for the OpenJDK portions of any `madura`
distribution is publicly available at the upstream repositories above, at the tag matching the
`javac` version the binary reports (`madura --version`). If you would rather receive it on
physical media, email <engineering@elide.dev> and we will supply it for no more than the cost of
distribution.

### GraalVM Community Edition

The binary is built by GraalVM Native Image, which links parts of the Substrate VM runtime into
the executable. GraalVM CE is licensed under **GPL-2.0 with the Classpath Exception**.

- Upstream source: <https://github.com/oracle/graal>
- Cosmopolitan-targeting patches: <https://github.com/elide-dev/graalcosmo-patches>

### Kotlin standard library

The entrypoint is Kotlin, and portions of the Kotlin standard library are linked into the image.
Licensed under the **Apache License 2.0** (SPDX: `Apache-2.0`).

- Upstream source: <https://github.com/JetBrains/kotlin>
- License text: <https://www.apache.org/licenses/LICENSE-2.0>

### Cosmopolitan Libc

**Every** distribution is linked against Cosmopolitan Libc, not only the universal one: the
native image is built with `--libc=cosmo` for every target, so the per-arch ELF binaries are
statically linked against it too. The `cosmo-universal` distribution additionally carries the
Actually Portable Executable loader, which is what lets one file run on macOS, Linux and Windows.
Cosmopolitan is licensed under the **ISC license**; some vendored components within it carry
their own permissive licenses, enumerated upstream.

- Upstream source and licenses: <https://github.com/jart/cosmopolitan>

## Trademarks

Java and OpenJDK are trademarks or registered trademarks of Oracle and/or its affiliates.
`madura` is not affiliated with, endorsed by, or sponsored by Oracle.
