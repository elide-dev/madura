# madura

[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/elide-dev/madura?utm_source=badge)
[![codecov](https://codecov.io/gh/elide-dev/madura/graph/badge.svg?token=dQmhOolA5k)](https://codecov.io/gh/elide-dev/madura)

An experiment, called `madura`, which provides a hermetic, minimalist Java compiler from Rust and JDK internals only.

> [!WARNING]
> This is unstable software.

- **`madura` can compile Java code identically to `javac`**, because it _is_ `javac`.
- **`madura` can quickly check Java code,** and it is never wrong, because it _is just `javac`_.
- **`madura` is very smol,** making it suitable for Git hooks, CI, agentic code-gen, and so on.
- **`madura` supports up to JDK25,**, so it works drop-in for most projects.

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

## Architecture

**(1) A minimal OpenJDK image is prepared via `jlink`.**

The minimal image is designed to (effectively) build `$JAVA_HOME/lib/modules` in a way that doesn't bloat your toolchain. Specific modules can be shipped without including e.g. Native Image, GraalVM's SDK modules, and so on, which most users don't need.

**(2) GraalVM `native-image` is used to build a shared library which implements `javac`.**

The Java compiler is built as `libmadura.so`, and then FFI'd into the Cargo build, where it can be used in Rust.

**(3) Minimalist Rust command line wrapping the shared-lib.**

The entrypoint is built via [`crates/madura`](crates/madura). The CLI entrypoint loads the shared-library, checks/prepares arguments via zero-overhead calls, and then invokes check/compile routines.

**(4) JDK APIs ship with the dist.**

To enable the packaged `javac` to understand release APIs, `lib/modules` and `ct.sym` ship in the distribution. Thus, no `JAVA_HOME` is required at all to run Madura.
