# Contributing to madura

Thanks for looking. `madura` is small on purpose — a hermetic native `javac` and nothing else —
so the highest-value contributions are usually bug reports with a reproducer, and patches that
delete code.

By participating you agree to the [Code of Conduct](./CODE_OF_CONDUCT.md). Contributions are
accepted under the repository's [0BSD license](./LICENSE); see [NOTICE.md](./NOTICE.md) for the
licensing of the binaries the build produces.

## Prerequisites

| Tool                     | Why                                                       | How                                      |
| ------------------------ | --------------------------------------------------------- | ---------------------------------------- |
| [mise](https://mise.jdx.dev) | Pins every other toolchain (`elide`, `bun`, `hk`, JDK 25) | `mise install`                           |
| GraalVM 25.2.x           | `native-image` builds the binary (`driverMode = "external"`) | `./scripts/setup-graalvm.sh <dir>`       |
| `cosmocc`                | Only for the universal (APE) distribution                  | `./scripts/setup-cosmocc.sh <dir>`       |

`JAVA_HOME` must point at mise's `openjdk-25` — a jimage-bearing JDK — because `jlink` from that
JDK produces the `lib/{modules,ct.sym}` the distribution ships. The GraalVM above contributes
`native-image` and nothing else; keep it out of `JAVA_HOME`.

The stock GraalVM 25.0.x line will not work. It forces `jdk.internal.jrtfs` to build-time
initialization, freezing the build machine's `lib/modules` path into the image — see the comment
at the top of `scripts/setup-graalvm.sh`.

## Build and test

```bash
make deps          # maven + npm dependencies
make build         # native image -> jlink -> target/dist
make test          # elide test dev (JVM unit tests) + bun test (CLI/smoke)
make help          # every target
```

`make build` produces `target/dist/madura` beside `target/dist/lib/`. That pairing is the whole
distribution contract: `madura` resolves platform metadata binary-relative (`<exe>/../lib/modules`,
with the executable path resolved through symlinks), then falls back to `$JAVA_HOME`, then to an
explicit `--java-home <dir>`. Anything that moves `lib/` away from the binary breaks hermeticity,
so run the smoke tests before assuming otherwise.

The testing regime is described in [README.md](./README.md#testing-regime). The bar for a change
that touches codegen is unchanged: emitted bytecode must stay **byte-identical** to what stock
`javac` produces.

## Commits and releases

Commits follow [Conventional Commits](https://www.conventionalcommits.org/); release-please reads
them to compute the next version and assemble `CHANGELOG.md`. The prefixes that produce visible
changelog entries are `feat`, `fix`, `perf`, and `build`; `ci`, `chore`, `docs`, `refactor`, and
`test` are hidden but still parsed. A `!` suffix or a `BREAKING CHANGE:` footer forces a major
bump.

Do not hand-edit `CHANGELOG.md`, `.release-please-manifest.json`, or the `version` fields in
`package.json` / `elide.pkl` — the release PR owns those.

Releases are cut by merging the release PR on `main`. That creates a *draft* GitHub release; the
same workflow run attaches the distribution artifacts it already built and smoke-tested, then
publishes. Downstream installers (Homebrew, container images) are updated from that same run.

## Pull requests

- One logical change per PR; a green `Gate` check is required.
- CI is path-filtered by `.github/triage.yml`. If you add a new build input, add it there too, or
  your change will not trigger a build.
- Comments explain *why*, not *what*. This repository's comments carry the reasoning behind
  non-obvious choices (see `job.release.yml` for the house style); keep that bar.
- Pin new GitHub Actions by commit SHA with a trailing `# vX.Y.Z` comment.

## Reporting bugs

A `madura` bug is almost always one of two things: a difference from stock `javac`, or a failure
to locate platform metadata. Either way, include:

- `madura --version` output, your OS and architecture, and how you installed it;
- the exact command line, and the same command run with `javac` if you have one;
- for miscompiles, the two class files (or `javap -c -p` of each).
