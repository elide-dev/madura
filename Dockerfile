# syntax=docker/dockerfile:1
#
# The madura container image: a hermetic `javac` and nothing else.
#
# Build context is a directory holding the *extracted* per-arch distributions
# produced by `make-dist.sh`, one per platform being built:
#
#   <context>/madura-linux-amd64/{madura,lib/{modules,ct.sym}}
#   <context>/madura-linux-arm64/{madura,lib/{modules,ct.sym}}
#
#   docker buildx build -f Dockerfile --platform linux/amd64,linux/arm64 target/docker
#
# `TARGETARCH` is amd64/arm64 — the same spelling `make-dist.sh` derives from
# `uname -m` — so a multi-platform build is pure COPY: no emulation, no cross
# toolchain, and each arch gets the binary that was smoke-tested for it.
#
# `FROM scratch` is not minimalism theater. The per-arch binaries are statically
# linked against Cosmopolitan Libc, so there is nothing left to depend on: no
# libc, no shell, no loader. The `lib/` directory beside the binary is the only
# reason this image is 60MB rather than 6MB, and it is the platform metadata
# (`lib/modules`, `lib/ct.sym`) that makes the compiler hermetic.

# Everything scratch cannot express. `/tmp` has to exist, and has to be
# world-writable, because the image runs as an unprivileged uid: javac writes
# there for annotation processing and for `-Xprefer`-style temporary state.
FROM busybox:1.37.0-musl AS layout
RUN mkdir -p /rootfs/tmp /rootfs/work \
    && chmod 1777 /rootfs/tmp \
    && chmod 0777 /rootfs/work

FROM scratch
ARG TARGETARCH
COPY --from=layout /rootfs/ /
# Ships as a unit: madura resolves `lib/modules` binary-relative
# (`<exe>/../lib/modules`), so the binary and its `lib/` must stay siblings.
COPY madura-linux-${TARGETARCH}/ /opt/madura/

# Numeric, and deliberately not backed by an /etc/passwd entry: javac never
# resolves the uid, and a passwd file is one more thing to keep correct. 65532
# is the conventional "nonroot" uid used by distroless.
USER 65532:65532
WORKDIR /work

# No shell in the image, so ENTRYPOINT is exec-form and the arguments are
# javac's: `docker run ... madura check Foo.java` reaches the CLI verbatim.
ENTRYPOINT ["/opt/madura/madura"]

# `licenses` is an SPDX license expression covering everything in the image, per
# the OCI spec — not just the most restrictive term. `WITH` binds tighter than
# `AND`, so the parentheses are redundant to a parser and load-bearing for a
# human. The four terms are madura's own code, the Kotlin stdlib and Cosmopolitan
# Libc linked into the binary, and the OpenJDK it is compiled from, which is also
# what lib/{modules,ct.sym} is. NOTICE.md enumerates them with upstream sources.
LABEL org.opencontainers.image.title="madura" \
      org.opencontainers.image.description="Smallest possible compliant Java toolchain" \
      org.opencontainers.image.source="https://github.com/elide-dev/madura" \
      org.opencontainers.image.documentation="https://github.com/elide-dev/madura#readme" \
      org.opencontainers.image.vendor="Elide" \
      org.opencontainers.image.base.name="scratch" \
      org.opencontainers.image.licenses="0BSD AND Apache-2.0 AND ISC AND (GPL-2.0-only WITH Classpath-exception-2.0)"
