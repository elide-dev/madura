use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// Copy `src` to `dst` only when missing, size-mismatched, or older than the
// source (lib/modules is ~180MB; unconditional copies would slow every build).
// `fs::copy` stamps the destination with the copy time, so `dst >= src` holds
// after staging and breaks when the JDK is updated in place.
fn stage(src: &Path, dst: &Path) {
    let fresh = match (fs::metadata(src), fs::metadata(dst)) {
        (Ok(s), Ok(d)) => {
            s.len() == d.len()
                && match (s.modified(), d.modified()) {
                    (Ok(sm), Ok(dm)) => dm >= sm,
                    _ => false,
                }
        }
        _ => false,
    };
    if !fresh {
        fs::copy(src, dst)
            .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
}

// Mirror of the Makefile's `target/jdkroot` rule. `--module-path app.jar` is
// omitted: only platform modules are added, so it never affected the output.
fn jlink_jdkroot(out: &Path) {
    // jlink refuses to write into an existing directory, and a half-built
    // root from an interrupted run must not survive.
    let _ = fs::remove_dir_all(out);
    let status = Command::new("jlink")
        .args([
            "--add-modules",
            "java.base,java.compiler,jdk.compiler",
            "--strip-debug",
            "--no-header-files",
            "--no-man-pages",
            "--output",
        ])
        .arg(out)
        .status()
        .expect("failed to run `jlink` — is a JDK on PATH? (try `mise install`)");
    assert!(status.success(), "jlink failed: {status}");
}

fn main() {
    let lib_dir = PathBuf::from(
        env::var("DEP_MADURA_JAVAC_LIB_DIR")
            .expect("DEP_MADURA_JAVAC_LIB_DIR is set by madura_javac's build script"),
    );
    let lib_file = env::var("DEP_MADURA_JAVAC_LIB_FILE")
        .expect("DEP_MADURA_JAVAC_LIB_FILE is set by madura_javac's build script");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Hermetic platform metadata is sourced from the build-time JDK. The
    // default comes from .cargo/config.toml ([env] → <workspace>/target/jdkroot);
    // a real environment variable (Makefile, CI) overrides it.
    let java_home = PathBuf::from(env::var("MADURA_JAVA_HOME").expect(
        "MADURA_JAVA_HOME must point at a JDK at build time (source of lib/modules and lib/ct.sym)",
    ));

    // When the *default* jdkroot is what's named and it hasn't been built yet,
    // jlink it here so bare `cargo build`/`cargo clippy` work on a fresh tree.
    // An explicitly-set path is never auto-created: pointing MADURA_JAVA_HOME
    // at a JDK that does not exist is a misconfiguration worth failing on.
    let default_jdkroot = manifest_dir()
        .ancestors()
        .nth(2)
        .expect("crate lives at <workspace>/crates/madura")
        .join("target/jdkroot");
    if java_home == default_jdkroot && !java_home.join("lib/modules").is_file() {
        jlink_jdkroot(&java_home);
    }

    let modules = java_home.join("lib/modules");
    let ct_sym = java_home.join("lib/ct.sym");

    // Track the staged inputs themselves: a missing path always reruns the
    // script, so deleting target/jdkroot (or updating the JDK in place) heals
    // on the next bare `cargo build` instead of silently reusing the cache.
    println!("cargo::rerun-if-changed={}", modules.display());
    println!("cargo::rerun-if-changed={}", ct_sym.display());

    assert!(
        modules.is_file(),
        "not a jimage-bearing JDK: {}",
        modules.display()
    );
    assert!(ct_sym.is_file(), "JDK lacks ct.sym: {}", ct_sym.display());

    // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out; three ancestors up is
    // target/<profile>; its parent is the dist root in the dev tree, so the
    // staged lib/ sits at target/lib — matching the <origin>/../lib rpath from
    // target/<profile>/madura, and mirroring the shipped <root>/{bin,lib} shape.
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR nested under the profile directory");
    let staged_lib = profile_dir
        .parent()
        .expect("profile dir has a parent")
        .join("lib");
    fs::create_dir_all(&staged_lib).unwrap();

    // The library is small and <origin>/../lib is the FIRST rpath entry, so this
    // copy must be unconditional — a stale staged copy would shadow OUT_DIR.
    fs::copy(lib_dir.join(&lib_file), staged_lib.join(&lib_file)).unwrap();
    stage(&modules, &staged_lib.join("modules"));
    stage(&ct_sym, &staged_lib.join("ct.sym"));

    // "Directory of the binary doing the loading", spelled per object format:
    // `$ORIGIN` in ELF, `@loader_path` in Mach-O.
    let origin = match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "macos" => "@loader_path",
        _ => "$ORIGIN",
    };

    println!("cargo::rerun-if-env-changed=MADURA_JAVA_HOME");
    println!("cargo::rustc-link-arg=-Wl,-rpath,{origin}/../lib");
    println!("cargo::rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
}
