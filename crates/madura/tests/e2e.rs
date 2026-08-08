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
fn version_flag_prints_javac_version() {
    let out = madura().arg("--version").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("javac"), "stdout was: {stdout}");
}
