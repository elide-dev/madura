use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

fn main() {
    let lib_dir = PathBuf::from(
        env::var("DEP_MADURA_JAVAC_LIB_DIR")
            .expect("DEP_MADURA_JAVAC_LIB_DIR is set by madura_javac's build script"),
    );
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Hermetic platform metadata is sourced from the build-time JDK.
    let java_home = PathBuf::from(env::var("MADURA_JAVA_HOME").expect(
        "MADURA_JAVA_HOME must point at a JDK at build time (source of lib/modules and lib/ct.sym)",
    ));
    let modules = java_home.join("lib/modules");
    let ct_sym = java_home.join("lib/ct.sym");
    assert!(
        modules.is_file(),
        "not a jimage-bearing JDK: {}",
        modules.display()
    );
    assert!(ct_sym.is_file(), "JDK lacks ct.sym: {}", ct_sym.display());

    // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out; three ancestors up is
    // target/<profile>; its parent is the dist root in the dev tree, so the
    // staged lib/ sits at target/lib — matching the $ORIGIN/../lib rpath from
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

    // The .so is small and $ORIGIN/../lib is the FIRST rpath entry, so this
    // copy must be unconditional — a stale staged copy would shadow OUT_DIR.
    fs::copy(
        lib_dir.join("libmadura-javac.so"),
        staged_lib.join("libmadura-javac.so"),
    )
    .unwrap();
    stage(&modules, &staged_lib.join("modules"));
    stage(&ct_sym, &staged_lib.join("ct.sym"));

    println!("cargo::rerun-if-env-changed=JAVA_HOME");
    println!("cargo::rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib");
    println!("cargo::rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
}
