//! End-to-end compile benchmarks: the shipped `madura` binary against the
//! JDK's own `javac`.
//!
//! Unlike `argv.rs` — which measures the in-process marshalling layer and is
//! run under CPU simulation — this benchmark spawns both compilers as
//! subprocesses and measures wall-clock time. Process startup *is* the thing
//! being measured: `madura` is a native image with a jlink'd platform image
//! beside it, `javac` is a launcher that boots a full JVM, and on a one-file
//! compile that difference dominates everything else.
//!
//! Nothing here links the native image, so this benchmark builds with
//! `--no-default-features` alongside `argv.rs`; the binary under test is the
//! already-assembled distribution, located at run time.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use divan::{Bencher, black_box};

fn main() {
    divan::main();
}

/// Spawns are ~30ms (`madura`) to ~1s (`javac`), so one spawn per sample and a
/// small sample count keeps the whole benchmark under a minute.
const SAMPLE_COUNT: u32 = 20;
const SAMPLE_SIZE: u32 = 1;

/// The workspace root: this crate lives at `<root>/crates/madura_javac`.
fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate is nested two levels under the workspace root")
}

/// The launcher under test, overridable with `MADURA_BIN`.
///
/// It has to sit in a dist root, because the binary derives its `java.home`
/// from its own location: `<root>/bin/madura` beside `<root>/lib/{modules,ct.sym}`.
/// Both `target/dist` and the cargo dev tree (`target/release/madura` with
/// `target/lib/`, root = `target/`) satisfy that; the default is the former.
fn madura_bin() -> PathBuf {
    let path = match std::env::var_os("MADURA_BIN") {
        Some(explicit) => PathBuf::from(explicit),
        None => workspace_root().join("target/dist/bin/madura"),
    };
    assert!(
        path.is_file(),
        "no madura binary at {} — run `make all`, or set MADURA_BIN",
        path.display(),
    );
    path
}

/// The reference `javac`, overridable with `JAVAC_BIN`, otherwise taken from
/// `JAVA_HOME` (the openjdk mise installs) and finally from `PATH`.
fn javac_bin() -> PathBuf {
    if let Some(explicit) = std::env::var_os("JAVAC_BIN") {
        let path = PathBuf::from(explicit);
        assert!(
            path.is_file(),
            "JAVAC_BIN is not a file: {}",
            path.display()
        );
        return path;
    }
    if let Some(java_home) = std::env::var_os("JAVA_HOME") {
        let path = PathBuf::from(java_home).join("bin/javac");
        if path.is_file() {
            return path;
        }
    }
    PathBuf::from("javac")
}

/// Every `.java` file in the smoke corpus, sorted so the command line is stable
/// across runs (and so the benchmark grows with the corpus).
fn smoke_sources() -> Vec<PathBuf> {
    let dir = workspace_root().join("tests/smoke/simple");
    let mut sources: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read smoke corpus at {}: {e}", dir.display()))
        .map(|entry| entry.expect("readable directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "java"))
        .collect();
    sources.sort();
    assert!(!sources.is_empty(), "no sources in {}", dir.display());
    sources
}

/// A per-benchmark output directory, created once and overwritten by every
/// iteration — the same way a compiler writes into an existing build tree.
fn output_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("bench-compile")
        .join(name);
    std::fs::create_dir_all(&dir).expect("output directory is creatable");
    dir
}

/// Run `compiler` to completion with output discarded, asserting it succeeded.
fn compile(compiler: &Path, sources: &[PathBuf], out: &Path) {
    let status = Command::new(compiler)
        .args(sources)
        .arg("-d")
        .arg(out)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", compiler.display()));
    assert!(status.success(), "{} failed: {status}", compiler.display());
}

/// Run `compiler --version`, asserting it succeeded.
fn version(compiler: &Path) {
    let status = Command::new(compiler)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", compiler.display()));
    assert!(status.success(), "{} failed: {status}", compiler.display());
}

/// Compiling the smoke corpus with the shipped `madura`.
#[divan::bench(sample_count = SAMPLE_COUNT, sample_size = SAMPLE_SIZE)]
fn compile_smoke_madura(bencher: Bencher) {
    let compiler = madura_bin();
    let sources = smoke_sources();
    let out = output_dir("madura");
    bencher.bench(|| compile(black_box(&compiler), black_box(&sources), black_box(&out)));
}

/// The same compile, driven by the JDK's `javac` — the baseline `madura` exists
/// to beat.
#[divan::bench(sample_count = SAMPLE_COUNT, sample_size = SAMPLE_SIZE)]
fn compile_smoke_javac(bencher: Bencher) {
    let compiler = javac_bin();
    let sources = smoke_sources();
    let out = output_dir("javac");
    bencher.bench(|| compile(black_box(&compiler), black_box(&sources), black_box(&out)));
}

/// `--version` with no compilation at all: the floor of what an invocation can
/// cost, which is almost entirely runtime startup.
#[divan::bench(sample_count = SAMPLE_COUNT, sample_size = SAMPLE_SIZE)]
fn startup_madura(bencher: Bencher) {
    let compiler = madura_bin();
    bencher.bench(|| version(black_box(&compiler)));
}

/// The same floor for `javac`, which pays for a full JVM boot.
#[divan::bench(sample_count = SAMPLE_COUNT, sample_size = SAMPLE_SIZE)]
fn startup_javac(bencher: Bencher) {
    let compiler = javac_bin();
    bencher.bench(|| version(black_box(&compiler)));
}
