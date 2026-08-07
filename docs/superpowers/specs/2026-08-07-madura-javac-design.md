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
| Platform metadata | Image built with `-H:+AllowJRTFileSystem`; the bin injects `-Djava.home=<dist root>` (resolved relative to the executable) so javac reads `<root>/lib/modules`. No JDK required at runtime |

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

Replace the stub body of `JvmInvoker.main`:

- `ToolProvider.getSystemJavaCompiler()`; if null, print an error to **stderr** and
  `System.exit(2)`.
- Otherwise `System.exit(compiler.run(System.in, System.out, System.err, *args))`.

The spread operator and `System.*` calls compile to plain JVM bytecode with no Kotlin
stdlib dependency.

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

`src/main.rs`: compute the dist root from the canonicalized executable path
(`current_exe().parent().parent()`), verify `<root>/lib/modules` exists (clear error
otherwise), then forward `-Djava.home=<root>` followed by `env::args_os().skip(1)` to
`madura_javac::invoke`, and `process::exit(code)`. `getargs` is removed from
dependencies.

## Data flow

```
madura Foo.java -d out
  → Rust resolves dist root from its own exe path; verifies <root>/lib/modules
  → Rust marshals argv (argv[0] = "madura", argv[1] = -Djava.home=<root>, then user args)
  → run_main (native-image JavaMainWrapper consumes the -D before main)
  → JvmInvoker.main
  → ToolProvider.getSystemJavaCompiler().run(stdin, stdout, stderr, args)
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
| `<root>/lib/modules` missing at runtime | stderr message naming the expected path, nonzero exit |
| System compiler null at runtime | stderr message, exit 2 |
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
