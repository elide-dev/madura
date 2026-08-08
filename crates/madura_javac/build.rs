use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo::rerun-if-changed=src/JavacInvoker.kt");
    println!("cargo::rerun-if-changed=elide.pkl");
    println!("cargo::rerun-if-changed=native-image");

    // Run from the crate dir, NOT via `-p` from elsewhere: native-image resolves
    // its output dir against the process cwd and aborts otherwise (found in Task 2).
    // elide's cache misses ConfigurationFileDirectories contents; see elide-dev/WHIPLASH#1416
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

    let so = manifest_dir.join(".dev/artifacts/native-image/madura-javac.so");
    assert!(
        so.is_file(),
        "missing native-image artifact: {}",
        so.display()
    );

    // The artifact has no `lib` prefix and no SONAME; the renamed copy is the
    // canonical name for both link time (-l madura-javac) and runtime lookup.
    let staged = out_dir.join("libmadura-javac.so");
    fs::copy(&so, &staged).unwrap();

    println!("cargo::rustc-link-search=native={}", out_dir.display());
    println!("cargo::rustc-link-lib=dylib=madura-javac");
    // Absolute rpath so this package's own test binaries can load the library.
    println!("cargo::rustc-link-arg=-Wl,-rpath,{}", out_dir.display());
    println!("cargo::metadata=lib_dir={}", out_dir.display());
}
