# Design: `madura` as an end-to-end `javac`

**Date:** 2026-08-07
**Status:** Approved

## Goal

`madura Foo.java -d out` behaves exactly like `javac Foo.java -d out` — same flag
surface, diagnostics, and exit codes — with compilation performed by the system Java
compiler baked into `madura-javac.so`, a GraalVM native-image shared library built by
Elide from Kotlin source (`JavacInvoker.kt`). Kotlin stdlib and reflection are not
available; the Kotlin source uses JDK core libraries only.

## Decisions

| Decision | Choice |
| --- | --- |
| Build driver | `cargo build` drives everything; `madura_javac/build.rs` invokes `elide build` |
| Linking | Link-time dylib dependency; rpath `$ORIGIN/../lib` (dist layout) + absolute staging dir fallback |
| CLI scope | Pure argv passthrough; no Rust-side arg parsing; drop `getargs` |
| FFI surface | `run_main(argc, argv)`; isolate/JNI APIs unused |
| Distribution | Fully hermetic: `<root>/{bin,lib}` layout; `lib/` ships `libmadura-javac.so`, `modules` (the JDK jimage), and `ct.sym` (for `--release N` compiles) |
| Platform metadata | Image built with `-H:+AllowJRTFileSystem`; the Kotlin entry resolves the binary's absolute path in-process (`ProcessHandle.current().info().command()`, symlinks resolved — mirroring WHIPLASH `Entry.kt`'s `resolveBinpath`), derives `<root>` as `bin/..`, and sets `java.home` before javac starts, so javac reads `<root>/lib/modules`. No JDK, no env vars, no argv injection at runtime. (`-D` argv injection is not viable anyway: the hardened image sets `-H:-ParseRuntimeOptions`.) |
| Reachability metadata | The image builds under `--exact-reachability-metadata`; javac's precise resource/service/reflection needs are traced with the native-image agent on a JVM run of the app jar, curated into repo-tracked config wired via `-H:ConfigurationFileDirectories`, then closed empirically against `MissingRegistration` errors |

### Amendment (2026-08-07, during implementation)

The known risk materialized in a second form: the compiler is present in the image,
but real compilation failed because (1) `java.home` is unset in a native image —
javac's `Locations` class NPEs in its static initializer — and (2) reading
`lib/modules` requires the jrt filesystem, which native-image only provides under
`-H:+AllowJRTFileSystem` (which itself "requires java.home to be set at runtime").
Verified empirically: the image parses `-Djava.home=<path>` from argv at runtime
(native-image `JavaMainWrapper` consumes `-D` args before `main`).

Per the user's direction, madura is **fully hermetic**: the distribution carries its
own `lib/modules` and `lib/ct.sym` (copied from the build-time JDK, `$JAVA_HOME` —
`ct.sym` serves `--release N` compiles against older platform APIs), laid out as
`<root>/bin/<binary>` + `<root>/lib/{libmadura-javac.so,modules,ct.sym}`. The binary
computes `<root>` from its own (canonicalized) executable path and injects
`-Djava.home=<root>` ahead of user args. In the cargo dev tree the same shape is
staged as `target/<profile>/madura` + `target/lib/…` (root = `target/`), so
`cargo run`/`cargo test` work with no environment setup and no JDK at runtime.

## Components

### 1. Kotlin entrypoint — `crates/madura_javac/src/JavacInvoker.kt`

**Amended (2026-08-07, e2e finding):** the entrypoint is `com.sun.tools.javac.Main.compile(args)`
— the same entry the real `javac` launcher uses — because
`ToolProvider.getSystemJavaCompiler().run(...)` ignores its stdout parameter and
routes everything (version banner, usage) to stderr, breaking stream parity with
`javac`. `Main.compile` splits notices→stdout / diagnostics→stderr and returns the
exit code:

```kotlin
package dev.elide.jvm

import com.sun.tools.javac.Main
import java.nio.file.Files
import java.nio.file.Paths

object JavacInvoker {
  // In the native image `java.home` is unset: resolve the dist root from the
  // binary's own absolute path (<root>/bin/madura, or target/<profile>/madura
  // in the dev tree), mirroring Elide Entry.kt's binpath resolution. On a
  // plain JVM java.home is already valid and this block is skipped.
  @JvmStatic fun main(args: Array<String>) {
    if (System.getProperty("java.home") == null) {
      val cmd = ProcessHandle.current().info().command().orElse(null)
      if (cmd == null) {
        System.err.println("madura: cannot resolve own binary path")
        System.exit(2)
        return
      }
      var bin = Paths.get(cmd)
      if (Files.isSymbolicLink(bin)) bin = bin.toRealPath()
      val root = bin.parent?.parent
      if (root == null || !Files.isRegularFile(root.resolve("lib").resolve("modules"))) {
        System.err.println(
          "madura: missing platform image at <root>/lib/modules (binary must live in <root>/bin or target/<profile>)")
        System.exit(2)
        return
      }
      System.setProperty("java.home", root.toString())
    }
    System.exit(Main.compile(args))
  }
}
```

Binary-path resolution happens in-process via JDK-core `ProcessHandle` (proven to
work inside native images by WHIPLASH `Entry.kt`), so no environment variables and
no argv rewriting are involved; the CLI remains pure passthrough end to end.

The former missing-compiler null-check is moot: with `Main` referenced directly, a
missing `jdk.compiler` fails the native-image build rather than appearing at runtime.
All calls compile to plain JVM bytecode with no Kotlin stdlib dependency.

`elide.pkl` additionally gains the native-image flag `-H:+AllowJRTFileSystem` so the
image can read `lib/modules` (jimage) at runtime, plus javac's resource bundles if
verification shows they are missing (`-H:IncludeResourceBundles=…`).

### 2. FFI crate — `crates/madura_javac`

`Cargo.toml` gains `links = "madura-javac"` and `build = "build.rs"`.

`build.rs`:

- `cargo:rerun-if-changed` on `src/JavacInvoker.kt` and `elide.pkl`, so `elide build`
  only reruns when its inputs change.
- Runs `elide build -p $CARGO_MANIFEST_DIR`; on failure, fails the cargo build with
  elide's output included.
- Verifies `.dev/artifacts/native-image/madura-javac.so` exists, then copies it into
  `$OUT_DIR`.
- Emits `cargo:rustc-link-search=native=$OUT_DIR`,
  `cargo:rustc-link-lib=dylib=madura-javac`, and `cargo:lib_dir=$OUT_DIR` (surfaced to
  dependent build scripts as `DEP_MADURA_JAVAC_LIB_DIR` via the `links` mechanism — no
  path guessing in the bin crate).

New `src/lib.rs`:

- `unsafe extern "C" { fn run_main(argc: c_int, argv: *mut *mut c_char) -> c_int; }`
- Safe wrapper `pub fn invoke(args: impl IntoIterator<Item = OsString>) -> Result<i32, InvokeError>`
  that marshals `argv[0] = "madura"` plus the caller's args into `CString`s (Unix
  `OsStr` bytes). `InvokeError` has a single variant today: an argument containing an
  interior NUL byte (carrying the offending argument), yielding an error rather than a
  panic.

### 3. Bin crate — `crates/madura`

New `build.rs`:

- Reads `DEP_MADURA_JAVAC_LIB_DIR` and stages the dev-tree dist shape under
  `target/lib/`: `libmadura-javac.so` (from the dep lib dir), plus `modules` and
  `ct.sym` copied from the build-time JDK (`$JAVA_HOME` — required at build time
  only).
- Emits two rpaths: `$ORIGIN/../lib` (the dist shape — works for both
  `target/<profile>/madura` → `target/lib` and `<root>/bin/madura` → `<root>/lib`)
  and the absolute dep lib dir (fallback for dev robustness).

`src/main.rs`: pure passthrough — forward `env::args_os().skip(1)` verbatim to
`madura_javac::invoke` and `process::exit(code)`. All dist-root/`java.home`
resolution lives in the Kotlin entry (single place, in-process). `getargs` is
removed from dependencies.

## Data flow

```
madura Foo.java -d out
  → Rust marshals argv verbatim (argv[0] = "madura") — pure passthrough, no root logic
  → run_main → JavacInvoker.main: java.home unset → resolve own binary via
    ProcessHandle (symlinks → toRealPath), root = bin/.., validate <root>/lib/modules,
    System.setProperty("java.home", root)
  → com.sun.tools.javac.Main.compile(args)  (notices→stdout, diagnostics→stderr)
  → javac reads platform classes from <root>/lib/modules via jrt (ct.sym for --release N)
  → .class files written; diagnostics on stdout/stderr unmodified
  → exit code propagates back
```

The exit code arrives either as `run_main`'s return value or via `System.exit`
terminating the process directly. For a passthrough CLI these are observably
identical, so the design does not depend on which one native-image performs.

## Error handling

| Failure | Behavior |
| --- | --- |
| `elide` missing or `elide build` fails | cargo build fails with elide's output |
| Artifacts missing after elide build | cargo build fails with a clear message |
| `$JAVA_HOME` unset/invalid at build time | `madura`'s build.rs fails with a clear message (needed to stage `modules`/`ct.sym`) |
| `<root>/lib/modules` missing at runtime | Kotlin entry: stderr message naming the expected layout, exit 2 |
| `jdk.compiler` absent | Native-image build fails (the entrypoint references `com.sun.tools.javac.Main` directly); no runtime failure mode |
| Invalid Java source | Real javac diagnostics on stderr, javac's own exit code |
| Interior NUL in an argument | Error message on stderr, nonzero exit |

## Testing

Integration test `crates/madura/tests/e2e.rs`, spawning `env!("CARGO_BIN_EXE_madura")`
in a temp dir — every spawn uses `env_remove("JAVA_HOME")` to prove hermeticity:

1. Valid `Hello.java` → exit 0, `Hello.class` exists.
2. Broken source → nonzero exit, diagnostic text on stderr.
3. `--version` → exit 0, prints a version string (flag-surface sanity).
4. `--release 21` compile → exit 0 (proves `ct.sym` + zip filesystem work in-image;
   contingency if it fails: include `jdk.zipfs` in the native image).

A dist packaging step assembles `target/dist/{bin,lib}` from the release build and
smoke-checks `target/dist/bin/madura` with `JAVA_HOME` unset.

## Known risk — de-risk first (RESOLVED)

`ToolProvider.getSystemJavaCompiler()` may return **null inside a native image** unless
`jdk.compiler` was baked into the image. Implementation step one is a smoke test of
`run_main` against the current stub `.so` (the stub already prints "Failed to load
compiler." when null). If null, fix `elide.pkl` / native-image flags before writing any
Rust.

**Outcome:** the compiler IS present (gate passed), but the risk materialized as the
missing-`java.home`/jrt problem described in the Amendment above; resolved via
`-H:+AllowJRTFileSystem` + the hermetic dist layout.

## Out of scope

- musl / fully-static builds. The workspace's musl target config anticipates a future
  fat-LTO static link of native-image output; the dylib path here is gnu-target-only.
- Isolate-based multi-compile in-process API (can be layered on later without breaking
  `run_main`).
- Non-Linux runtime paths.
- kotlinc or any toolchain surface beyond javac.
