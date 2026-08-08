use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// What the native image is built from, relative to the crate directory. These
/// mirror the `rerun-if-changed` keys below: cargo decides whether this script
/// runs at all, and these decide whether it has anything to do once it does.
const IMAGE_INPUTS: &[&str] = &["src/JavacInvoker.kt", "elide.pkl", "native-image"];

/// The newest mtime at or under `path`, or `None` if it does not exist.
fn newest_mtime(path: &Path) -> Option<SystemTime> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_dir() {
        return meta.modified().ok();
    }
    fs::read_dir(path)
        .ok()?
        .filter_map(|entry| newest_mtime(&entry.ok()?.path()))
        .max()
        .or_else(|| meta.modified().ok())
}

/// Whether `so` is present and no older than every input it is built from.
///
/// The root `Makefile` builds this image too, so by the time cargo runs the
/// image is usually already on disk and current — rebuilding it here would mean
/// paying for a second `native-image` run per build. A missing or stale image
/// still rebuilds, so a touched `JavacInvoker.kt` is never silently linked
/// against yesterday's image.
fn image_is_current(manifest_dir: &Path, so: &Path) -> bool {
    let Some(built) = newest_mtime(so) else {
        return false;
    };
    IMAGE_INPUTS
        .iter()
        .all(|input| newest_mtime(&manifest_dir.join(input)).is_none_or(|source| source <= built))
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo::rerun-if-changed=src/JavacInvoker.kt");
    println!("cargo::rerun-if-changed=elide.pkl");
    println!("cargo::rerun-if-changed=native-image");

    // Without `native` there is nothing to link against, so the JVM toolchain is
    // not needed: the pure-Rust layer (and its benchmarks) build on their own.
    if env::var_os("CARGO_FEATURE_NATIVE").is_none() {
        return;
    }

    // A prebuilt image can be supplied directly, skipping elide and the
    // freshness check: CI benchmark runners cannot run `elide` at all (glibc),
    // and link the shared library shipped inside the dist artifact instead —
    // built from the same commit, for the same arch.
    println!("cargo::rerun-if-env-changed=MADURA_JAVAC_SO");
    if let Some(prebuilt) = env::var_os("MADURA_JAVAC_SO") {
        let prebuilt = PathBuf::from(prebuilt);
        assert!(
            prebuilt.is_file(),
            "MADURA_JAVAC_SO is not a file: {}",
            prebuilt.display()
        );
        emit_link_directives(&prebuilt, &out_dir);
        return;
    }

    let so = manifest_dir.join(".dev/artifacts/native-image/madura-javac.so");

    if !image_is_current(&manifest_dir, &so) {
        // Run from the crate dir, NOT via `-p` from elsewhere: native-image resolves
        // its output dir against the process cwd and aborts otherwise (found in Task 2).
        // `--no-cache` because elide's own cache misses ConfigurationFileDirectories
        // contents (see elide-dev/WHIPLASH#1416) — the freshness check above is what
        // keeps this from running on every build.
        let output = Command::new("elide")
            .arg("build")
            .arg("--no-cache")
            .arg("--release")
            .current_dir(&manifest_dir)
            .output()
            .expect("failed to run `elide` — is it installed and on PATH? (try `mise install`)");
        // Forward elide's output to stderr: stdout is reserved for cargo directives.
        eprint!("{}", String::from_utf8_lossy(&output.stdout));
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        assert!(
            output.status.success(),
            "`elide build` failed: {}",
            output.status
        );
    }

    assert!(
        so.is_file(),
        "missing native-image artifact: {}",
        so.display()
    );

    emit_link_directives(&so, &out_dir);
}

/// Stage `so` under its canonical name and emit the link/rpath directives.
///
/// The artifact has no `lib` prefix and no SONAME; the renamed copy is the
/// canonical name for both link time (-l madura-javac) and runtime lookup.
fn emit_link_directives(so: &Path, out_dir: &Path) {
    let staged = out_dir.join("libmadura-javac.so");
    fs::copy(so, &staged).unwrap();

    // DWARF rides in a `.gnu_debuglink` sidecar named after the *original*
    // artifact; debuggers and profilers look it up in the loaded library's own
    // directory, so staging it beside the copy gives perf/valgrind line-level
    // frames. The dist ships without one — absence is normal.
    let debug_name = format!(
        "{}.debug",
        so.file_name()
            .expect("artifact has a name")
            .to_string_lossy()
    );
    let debug = so.with_file_name(&debug_name);
    if debug.is_file() {
        fs::copy(&debug, out_dir.join(&debug_name)).unwrap();
    }

    println!("cargo::rustc-link-search=native={}", out_dir.display());
    println!("cargo::rustc-link-lib=dylib=madura-javac");
    // Absolute rpath so this package's own test and bench binaries can load
    // the library.
    println!("cargo::rustc-link-arg=-Wl,-rpath,{}", out_dir.display());
    println!("cargo::metadata=lib_dir={}", out_dir.display());
}
