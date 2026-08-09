# madura

[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/elide-dev/madura?utm_source=badge)
[![codecov](https://codecov.io/gh/elide-dev/madura/graph/badge.svg?token=dQmhOolA5k)](https://codecov.io/gh/elide-dev/madura)

An experiment, called `madura`, which provides a hermetic, minimalist Java compiler from Rust and JDK internals only.

> [!WARNING]
> This is unstable software.

## Why

In some circumstances, you just want to check your Java code, like a typechecker would in Python or TypeScript. A good example of this is agentic engineering: code is generated often, and to keep codegen sane, many full compile cycles are needed.

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

**Compile Java**

```diff
- javac -d target ./some/java/Code.java
+ madura -d target ./some/java/Code.java
```
> Madura is designed to pass arguments to a regular `javac` invocation by default. `madura compile` is an explicit alias for this.

**Check Java**

```
# it's `madura check`
madura check ...

# `check` accepts identical arguments to `javac`
madura check -d target ./some/java/Code.java
```
> Madura's `check` step is still just `javac`, but without the codegen step.

