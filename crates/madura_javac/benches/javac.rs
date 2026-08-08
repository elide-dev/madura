//! In-process, CPU-simulated end-to-end compile benchmarks.
//!
//! `compile.rs` measures the shipped binary by spawning it: walltime mode,
//! where process startup is the point. This suite links the native image and
//! drives `compile_javac` in-process instead, under CodSpeed's simulated CPU —
//! deterministic instruction counts and flamegraphs for everything a `madura`
//! invocation does past `exec`: argv marshalling, isolate creation, `java.home`
//! wiring, javac itself, and class emission. Hotspots and regressions in the
//! compile path land here first, with the profile to attribute them.
//!
//! Iterations share one isolate, warmed before measurement: SVM's G1 permits a
//! single isolate per process lifetime, so the library creates it once and
//! every iteration reuses it. Isolate and process boot are therefore *not*
//! measured here — the walltime `compile.rs` suite owns those — which also
//! makes this the cleaner compile-path signal: no runtime-boot instructions
//! diluting the javac profile.
//!
//! Unlike the sibling suites this one needs the `native` feature; built
//! without it (as CI builds `argv` and `compile`) it compiles to an empty
//! harness.

fn main() {
    divan::main();
}

/// The workspace root: this crate lives at `<root>/crates/madura_javac`.
#[cfg(feature = "native")]
fn workspace_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate is nested two levels under the workspace root")
}

#[cfg(feature = "native")]
mod inproc {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use divan::{Bencher, black_box};

    use super::workspace_root;

    /// One spawn-equivalent per sample; compiles are tens of milliseconds, so
    /// this keeps the whole suite in the low seconds outside instrumentation.
    const SAMPLE_COUNT: u32 = 10;
    const SAMPLE_SIZE: u32 = 1;

    /// The platform-image root handed to the image as `java.home`:
    /// `MADURA_HOME` if set, else the assembled dist, else the jlink'd jdkroot
    /// the cargo build maintains.
    fn platform_root() -> PathBuf {
        if let Some(explicit) = std::env::var_os("MADURA_HOME") {
            let root = PathBuf::from(explicit);
            assert!(
                root.join("lib/modules").is_file(),
                "MADURA_HOME has no lib/modules: {}",
                root.display()
            );
            return root;
        }
        for candidate in [
            workspace_root().join("target/dist"),
            workspace_root().join("target/jdkroot"),
        ] {
            if candidate.join("lib/modules").is_file() {
                return candidate;
            }
        }
        panic!(
            "no platform image beside target/ — run `cargo build` or `make all`, or set MADURA_HOME"
        );
    }

    /// Every `.java` file in the smoke corpus, sorted so the command line is
    /// stable across runs (and so the benchmark grows with the corpus).
    fn smoke_sources() -> Vec<OsString> {
        let dir = workspace_root().join("tests/smoke/simple");
        let mut sources: Vec<OsString> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read smoke corpus at {}: {e}", dir.display()))
            .map(|entry| {
                entry
                    .expect("readable directory entry")
                    .path()
                    .into_os_string()
            })
            .filter(|path| Path::new(path).extension().is_some_and(|ext| ext == "java"))
            .collect();
        sources.sort();
        assert!(!sources.is_empty(), "no sources in {}", dir.display());
        sources
    }

    /// A per-benchmark output directory, created once and overwritten by every
    /// iteration — the same way a compiler writes into an existing build tree.
    fn output_dir(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("bench-javac")
            .join(name);
        std::fs::create_dir_all(&dir).expect("output directory is creatable");
        dir
    }

    /// One full in-process invocation, panicking on anything but exit code 0.
    fn compile(home: &Path, args: &[OsString]) {
        let code = madura_javac::invoke_with(home, args.iter().cloned())
            .unwrap_or_else(|err| panic!("invocation failed before javac ran: {err}"));
        assert_eq!(code, 0, "javac exited with {code}");
    }

    /// Compiling the smoke corpus, end to end: marshalling, entering the
    /// (warm) isolate, `java.home` wiring, javac, and class files on disk.
    #[divan::bench(sample_count = SAMPLE_COUNT, sample_size = SAMPLE_SIZE)]
    fn compile_smoke(bencher: Bencher) {
        let home = platform_root();
        let out = output_dir("smoke");
        let mut args = smoke_sources();
        args.push(OsString::from("-d"));
        args.push(out.into_os_string());
        compile(&home, &args); // verify once, outside the measurement
        bencher.bench(|| compile(black_box(&home), black_box(&args)));
    }

    /// `--version` with no compilation: the smallest javac entry there is,
    /// on the same warm isolate. When `compile_smoke` regresses and this does
    /// not, the hotspot is in the compile path; when both do, it sits in the
    /// shared entry (marshalling, isolate crossing, javac argument handling).
    #[divan::bench(sample_count = SAMPLE_COUNT, sample_size = SAMPLE_SIZE)]
    fn version(bencher: Bencher) {
        let home = platform_root();
        let args = [OsString::from("--version")];
        compile(&home, &args);
        bencher.bench(|| compile(black_box(&home), black_box(&args)));
    }
}
