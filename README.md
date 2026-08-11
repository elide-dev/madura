# madura

[![codecov](https://codecov.io/gh/elide-dev/madura/graph/badge.svg?token=dQmhOolA5k)](https://codecov.io/gh/elide-dev/madura)

An experiment, called `madura`, which provides a hermetic, minimalist Java compiler from JDK internals only.

> [!WARNING]
> This is unstable software.

- **`madura` can compile Java code identically to `javac`**, because it _is_ `javac`.
- **`madura` can quickly check Java code,** and it is never wrong, because it _is just `javac`_.
- **`madura` is very smol,** making it suitable for Git hooks, CI, agentic code-gen, and so on.
- **`madura` supports up to JDK25,** so it works drop-in for most projects.

See **_Architecture_** below for full details.

## Why

**In some circumstances, you just want to check your Java code, like a typechecker would in Python or TypeScript.** A good example of this is agentic engineering: code is generated often, and to keep codegen sane, many full compile cycles are needed.

This tool takes a different approach: it's just native `javac`. Optionally, one can omit the codegen step by running `madura check` instead of `madura compile`, which allows hyper-fast checking of your Java code without extra fuss.

The result is a very fast check cycle, no daemon needed:

![madura check bench](./docs/check.gif)

## Getting Started

Install Madura using any of the following methods:
```
tbd
```

### Usage

`madura` is a hermetic native command line tool, and so requires no JDK or `JAVA_HOME` to use. It is self-contained and supports up to **JDK 25** at this time.

#### Check Java

```
# it's `madura check`
madura check ...

# `check` accepts identical arguments to `javac`
madura check -d target ./some/java/Code.java
```
> Madura's `check` step is still just `javac`, but without the codegen step.

#### Compile Java

```diff
- javac -d target ./some/java/Code.java
+ madura -d target ./some/java/Code.java
```
> Madura is designed to pass arguments to a regular `javac` invocation by default. `madura compile` is an explicit alias for this.

## Testing Regime

To make absolutely sure that `madura` behaves as expected, there are several layers of testing:

- **Unit Testing**
  - JVM-side tests (`elide test`) guarantee the compiler entrypoint's contract
- **CLI Testing**
  - `bun test` drives the assembled binary end-to-end: subcommands, passthrough, `--version`, and platform-metadata resolution
- **Smoke**
  - For each smoke test, a reference JVM bytecode class is built using normal `javac` from OpenJDK
  - Then, `madura check` is run, and expected outputs are checked
  - Then, `madura compile` is run, expected outputs are checked, and bytecode is compared
  - Tests only pass if bytecode is _byte-identical_ to what `javac` would produce
- **TCK**
  - `madura` then runs against relevant upstream tests via [`testsuite`](https://github.com/elide-dev/testsuite).

## Architecture

**(1) Platform metadata is prepared via `jlink`.**

A minimal image (`java.base`, `java.compiler`, `jdk.compiler`) supplies `lib/modules` (the platform jimage) and `lib/ct.sym` (for `--release N` targeting), without bloating your toolchain with Native Image or GraalVM SDK modules most users don't need.

**(2) GraalVM `native-image` builds `madura` as a single binary.**

The compiler entrypoint is Kotlin (`dev.elide.jvm.JavacInvoker`), built directly into a native executable — no Rust, no shared library, no FFI. Its `main` is the whole CLI.

**(3) The CLI mirrors `javac`.**

`madura check` runs `javac` without codegen; `madura compile` (or no subcommand at all) is a drop-in `javac`. Platform metadata is located binary-relative — `<exe>/../<arch>/lib/modules` — with `$JAVA_HOME` and an explicit `--java-home <dir>` as fallbacks.

**(4) The distribution is hermetic.**

The dist ships the binary beside its `<arch>/lib/{modules,ct.sym}`, so no JDK or `JAVA_HOME` is required at runtime.
