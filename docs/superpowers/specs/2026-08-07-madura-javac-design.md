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
| Linking | Link-time dylib dependency; rpath `$ORIGIN` + absolute artifact dir |
| CLI scope | Pure argv passthrough; no Rust-side arg parsing; drop `getargs` |
| FFI surface | `run_main(argc, argv)`; isolate/JNI APIs unused |

## Components

### 1. Kotlin entrypoint — `crates/madura_javac/src/JavacInvoker.kt`

Replace the stub body of `JvmInvoker.main`:

- `ToolProvider.getSystemJavaCompiler()`; if null, print an error to **stderr** and
  `System.exit(2)`.
- Otherwise `System.exit(compiler.run(System.in, System.out, System.err, *args))`.

The spread operator and `System.*` calls compile to plain JVM bytecode with no Kotlin
stdlib dependency.

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

- Reads `DEP_MADURA_JAVAC_LIB_DIR`, copies `madura-javac.so` next to the built binary.
- Emits two rpaths: `$ORIGIN` (relocatable distribution — ship bin + .so side by side)
  and the absolute lib dir (dev-loop robustness, e.g. `cargo test` / `cargo run`).

`src/main.rs`: forward `env::args_os().skip(1)` to `madura_javac::invoke`, then
`process::exit(code)`. `getargs` is removed from dependencies.

## Data flow

```
madura Foo.java -d out
  → Rust marshals argv (argv[0] = "madura")
  → run_main (native-image JavaMainWrapper)
  → JvmInvoker.main
  → ToolProvider.getSystemJavaCompiler().run(stdin, stdout, stderr, args)
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
| System compiler null at runtime | stderr message, exit 2 |
| Invalid Java source | Real javac diagnostics on stderr, javac's own exit code |
| Interior NUL in an argument | Error message on stderr, nonzero exit |

## Testing

Integration test `crates/madura/tests/e2e.rs`, spawning `env!("CARGO_BIN_EXE_madura")`
in a temp dir:

1. Valid `Hello.java` → exit 0, `Hello.class` exists.
2. Broken source → nonzero exit, diagnostic text on stderr.
3. `--version` → exit 0, prints a version string (flag-surface sanity).

## Known risk — de-risk first

`ToolProvider.getSystemJavaCompiler()` may return **null inside a native image** unless
`jdk.compiler` was baked into the image. Implementation step one is a smoke test of
`run_main` against the current stub `.so` (the stub already prints "Failed to load
compiler." when null). If null, fix `elide.pkl` / native-image flags before writing any
Rust.

## Out of scope

- musl / fully-static builds. The workspace's musl target config anticipates a future
  fat-LTO static link of native-image output; the dylib path here is gnu-target-only.
- Isolate-based multi-compile in-process API (can be layered on later without breaking
  `run_main`).
- Non-Linux runtime paths.
- kotlinc or any toolchain surface beyond javac.
