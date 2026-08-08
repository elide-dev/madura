use std::fs;
use std::path::PathBuf;
use std::process::Command;

// Every spawn removes JAVA_HOME: madura must be hermetic — platform metadata
// comes from <dist root>/lib/{modules,ct.sym}, never from the environment.
fn madura() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_madura"));
    cmd.env_remove("JAVA_HOME");
    cmd
}

fn workdir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A file in the shared smoke corpus at `<workspace>/tests/smoke`, which the
/// CI distribution smoke test and the in-process benchmarks also compile. The
/// corpus is the single place a new case has to be added to reach all three.
fn smoke(relative: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate is nested two levels under the workspace root")
        .join("tests/smoke")
        .join(relative);
    assert!(
        path.is_file(),
        "missing smoke corpus file: {}",
        path.display()
    );
    path
}

#[test]
fn compiles_valid_java_to_class_file() {
    let dir = workdir("valid");
    fs::write(
        dir.join("Hello.java"),
        "public class Hello { public static void main(String[] a) { System.out.println(\"hi\"); } }",
    )
    .unwrap();
    let out = madura()
        .current_dir(&dir)
        .args(["Hello.java", "-d", "out"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(dir.join("out/Hello.class").is_file());
}

#[test]
fn compiles_for_older_release_via_ct_sym() {
    let dir = workdir("release21");
    fs::write(
        dir.join("Hello.java"),
        "public class Hello { public static void main(String[] a) { System.out.println(\"hi\"); } }",
    )
    .unwrap();
    let out = madura()
        .current_dir(&dir)
        .args(["--release", "21", "Hello.java", "-d", "out"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(dir.join("out/Hello.class").is_file());
}

#[test]
fn reports_diagnostics_and_nonzero_exit_on_invalid_java() {
    let dir = workdir("invalid");
    fs::write(
        dir.join("Broken.java"),
        "public class Broken { this is not java }",
    )
    .unwrap();
    let out = madura()
        .current_dir(&dir)
        .arg("Broken.java")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error"), "stderr was: {stderr}");
}

#[test]
fn check_subcommand_analyzes_without_writing() {
    let dir = workdir("check-valid");
    fs::write(
        dir.join("Hello.java"),
        "public class Hello { public static void main(String[] a) { System.out.println(\"hi\"); } }",
    )
    .unwrap();
    let out = madura()
        .current_dir(&dir)
        .args(["check", "Hello.java", "-d", "out"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        !dir.join("out/Hello.class").exists(),
        "check mode must not write class files"
    );
}

#[test]
fn check_subcommand_reports_errors() {
    let dir = workdir("check-invalid");
    fs::write(
        dir.join("Broken.java"),
        "public class Broken { this is not java }",
    )
    .unwrap();
    let out = madura()
        .current_dir(&dir)
        .args(["check", "Broken.java"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error"), "stderr was: {stderr}");
}

#[test]
fn compile_subcommand_compiles() {
    let dir = workdir("compile-subcommand");
    fs::write(
        dir.join("Hello.java"),
        "public class Hello { public static void main(String[] a) { System.out.println(\"hi\"); } }",
    )
    .unwrap();
    let out = madura()
        .current_dir(&dir)
        .args(["compile", "Hello.java", "-d", "out"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(dir.join("out/Hello.class").is_file());
}

#[test]
fn version_flag_prints_javac_version() {
    let out = madura().arg("--version").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("javac"), "stdout was: {stdout}");
}

// --- Shared smoke corpus -------------------------------------------------
//
// The cases below run the corpus at `<workspace>/tests/smoke` through every
// mode. `job.build.yml` runs the same files against the assembled
// distribution, so a case added there is covered in both places.

#[test]
fn smoke_corpus_compiles_and_checks() {
    let dir = workdir("smoke-simple");
    let source = smoke("simple/Hello.java");

    // Compile mode writes the class file, under its package directory.
    let out = madura()
        .current_dir(&dir)
        .arg("compile")
        .arg(&source)
        .args(["-d", "out"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(dir.join("out/simple/Hello.class").is_file());

    // Check mode accepts the same source and writes nothing.
    let out = madura()
        .current_dir(&dir)
        .arg("check")
        .arg(&source)
        .args(["-d", "check-out"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        !dir.join("check-out").exists(),
        "check mode must not write output"
    );
}

#[test]
fn smoke_corpus_broken_source_fails_check() {
    let dir = workdir("smoke-broken-check");
    let out = madura()
        .current_dir(&dir)
        .arg("check")
        .arg(smoke("broken/Sample.java"))
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "check must reject the broken corpus; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error"), "stderr was: {stderr}");
}

#[test]
fn smoke_corpus_broken_source_fails_compile() {
    let dir = workdir("smoke-broken-compile");
    let out = madura()
        .current_dir(&dir)
        .arg("compile")
        .arg(smoke("broken/Sample.java"))
        .args(["-d", "out"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "compile must reject the broken corpus; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error"), "stderr was: {stderr}");
    assert!(
        !dir.join("out/some/pkg/here/Example.class").exists(),
        "a failed compile must not leave a class file behind"
    );
}

// Passthrough: no leading subcommand, so the corpus source is javac's own
// first argument — the mode the binary uses when it stands in for `javac`.
#[test]
fn smoke_corpus_passthrough_compiles() {
    let dir = workdir("smoke-passthrough");
    let out = madura()
        .current_dir(&dir)
        .arg(smoke("simple/Hello.java"))
        .args(["-d", "out"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(dir.join("out/simple/Hello.class").is_file());
}
