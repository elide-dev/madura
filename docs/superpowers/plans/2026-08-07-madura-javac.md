# madura end-to-end javac — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `madura` behave exactly like `javac`, backed by the system Java compiler embedded in `madura-javac.so` (Kotlin → Elide native-image), shipped as a fully hermetic `<root>/{bin,lib}` distribution.

**Architecture:** `crates/madura_javac` is an FFI crate whose `build.rs` runs `elide build`, stages the native-image shared library under `OUT_DIR` as `libmadura-javac.so`, and exposes a safe `invoke()` wrapper over the exported `run_main(argc, argv)`. `crates/madura` is a thin bin crate that injects `-Djava.home=<dist root>` (resolved from its own executable path) and forwards argv; its `build.rs` stages `target/lib/{libmadura-javac.so,modules,ct.sym}` and sets the `$ORIGIN/../lib` rpath, so dev tree and shipped dist share one hermetic shape.

**Tech Stack:** Rust (nightly-2026-06-13, edition 2024), Elide 1.4.2 (`elide` on PATH via mise), GraalVM native-image shared library, Kotlin compiled against JDK core libs only.

**Spec:** `docs/superpowers/specs/2026-08-07-madura-javac-design.md`

## Global Constraints

- Kotlin source must not use the Kotlin stdlib or reflection — JDK core libraries only (`elide.pkl` sets `noStdlib = true`).
- No new Rust dependencies. `getargs` is removed from `crates/madura`; `mimalloc` stays.
- CLI is pure argv passthrough except for ONE injected leading argument, `-Djava.home=<dist root>` (consumed by the native image before `main`); exit codes propagate unmodified.
- **Hermetic distribution:** layout `<root>/bin/<binary>` + `<root>/lib/{libmadura-javac.so,modules,ct.sym}`; dev tree mirrors it as `target/<profile>/madura` + `target/lib/…` (root = `target/`). No JDK at runtime; `$JAVA_HOME` required at build time only (source of `modules`/`ct.sym`).
- Host target only: `x86_64-unknown-linux-gnu`. The musl/static target is out of scope.
- The artifact `.dev/artifacts/native-image/madura-javac.so` has **no `DT_SONAME` and no `lib` prefix**. Every staged copy MUST be renamed to `libmadura-javac.so` — that is both the link-time name (`-l madura-javac`) and the runtime name the loader resolves via rpath. Never stage it under its original name.
- Elide build state (`.dev/`) is gitignored; never commit artifacts.
- `elide build` takes minutes when Kotlin/pkl inputs change; `cargo:rerun-if-changed` limits reruns to those inputs.
- All commits end with: `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`

---

### Task 1: De-risk gate — smoke-test `run_main` on the existing stub `.so`

The stub Kotlin main already checks `ToolProvider.getSystemJavaCompiler()` and prints
`Failed to load compiler.` when it is null. Calling `run_main` on the current artifact
therefore answers the spec's known risk (is `jdk.compiler` baked into the image?)
before any Rust is written. **This task is a gate: if it fails, stop — do not proceed
to later tasks.** Nothing here is committed; the driver is throwaway.

**Files:**
- Create (throwaway, outside repo): `/tmp/claude-1000/madura-smoke/smoke.c`

**Interfaces:**
- Consumes: `run_main(int, char**)` exported by `crates/madura_javac/.dev/artifacts/native-image/madura-javac.so`
- Produces: a compiled `./smoke` driver reused by Task 2's verification steps

- [ ] **Step 1: Write the C driver**

```bash
mkdir -p /tmp/claude-1000/madura-smoke
cat > /tmp/claude-1000/madura-smoke/smoke.c <<'EOF'
#include <stdio.h>

extern int run_main(int argc, char** argv);

int main(int argc, char** argv) {
  int code = run_main(argc, argv);
  printf("run_main returned %d\n", code);
  return code;
}
EOF
```

- [ ] **Step 2: Compile it against the artifact**

```bash
ART=/home/sam/workspace/labs/SUPERCRITICAL/crates/madura_javac/.dev/artifacts/native-image
cc /tmp/claude-1000/madura-smoke/smoke.c -o /tmp/claude-1000/madura-smoke/smoke \
  -L"$ART" -l:madura-javac.so -Wl,-rpath,"$ART"
```

(`-l:` links the exact filename, so no rename is needed for this throwaway. If `cc` is
missing, use `clang-22`.)

Expected: compiles with no output.

- [ ] **Step 3: Run it and check the compiler is present in the image**

```bash
/tmp/claude-1000/madura-smoke/smoke; echo "exit=$?"
```

Expected: output contains `Hello Kotlin Entry` and `exit=0`.
(`run_main returned 0` may or may not print — `System.exit` inside the image may
terminate the process directly. Either is fine; only the exit code matters.)

**GATE:** If the output contains `Failed to load compiler.` (exit 2), STOP. Report to
the user that `jdk.compiler` is not baked into the image and that `elide.pkl` /
native-image flags need investigation before any other task can proceed.

---

### Task 2: Kotlin — real javac passthrough + JRT-enabled image

> **AMENDED after first implementation round:** the Kotlin swap alone is not enough.
> Inside a native image `java.home` is unset (javac NPEs in `Locations.<clinit>`) and
> reading `lib/modules` needs `-H:+AllowJRTFileSystem`. Verification passes
> `-Djava.home=$JAVA_HOME` explicitly (the image parses `-D` argv at runtime);
> hermetic resolution lands in Task 4.

**Files:**
- Modify: `crates/madura_javac/src/JavacInvoker.kt` (replace entire contents)
- Modify: `crates/madura_javac/elide.pkl` (add native-image flag)

**Interfaces:**
- Consumes: nothing from other tasks
- Produces: `run_main` behavior — argv (minus argv[0]) goes to
  `JavaCompiler.run(System.in, System.out, System.err, ...args)`; process exit code is
  the compiler's return value; missing compiler → stderr message + exit 2. The image
  honors `-Djava.home=<path>` from argv and reads platform classes from
  `<path>/lib/modules` via jrt. Tasks 3–5 rely on exactly this behavior.

- [ ] **Step 1: Establish the failing state (stub does not compile Java)**

```bash
cd /tmp/claude-1000/madura-smoke
cat > Hello.java <<'EOF'
public class Hello {
  public static void main(String[] args) {
    System.out.println("hi");
  }
}
EOF
./smoke Hello.java -d out; echo "exit=$?"
ls out/Hello.class
```

Expected: FAIL — output is `Hello Kotlin Entry` (the stub ignores args), and
`ls` reports `No such file or directory`.

- [ ] **Step 2: Replace JavacInvoker.kt**

Replace the entire contents of `crates/madura_javac/src/JavacInvoker.kt` with:

```kotlin
package dev.elide.jvm;

import javax.tools.ToolProvider

object JvmInvoker {
  @JvmStatic fun main(args: Array<String>) {
    val compiler = ToolProvider.getSystemJavaCompiler()
    if (compiler == null) {
      System.err.println("madura: system Java compiler is not available in this image")
      System.exit(2)
      return
    }
    System.exit(compiler.run(System.`in`, System.out, System.err, *args))
  }
}
```

(No Kotlin stdlib: `System.*`, `ToolProvider`, and the spread operator all compile to
plain JVM bytecode against JDK core libs.)

- [ ] **Step 3: Enable the JRT filesystem in the native image**

In `crates/madura_javac/elide.pkl`, extend the native-image flags block:

```pkl
      flags {
        "--shared"
        "--verbose"
        "-H:-CheckToolchain"
        "-H:+AllowJRTFileSystem"
      }
```

- [ ] **Step 4: Rebuild the shared library**

```bash
cd /home/sam/workspace/labs/SUPERCRITICAL
elide build -p ./crates/madura_javac
```

Expected: build succeeds; `crates/madura_javac/.dev/artifacts/native-image/madura-javac.so`
has a fresh mtime. (Native-image link takes a minute or more.)

- [ ] **Step 5: Verify real compilation through the FFI boundary**

`java.home` is unset inside the image, so verification passes it explicitly — the
image's `JavaMainWrapper` consumes `-Dkey=value` argv entries before `main`:

```bash
cd /tmp/claude-1000/madura-smoke
JDK="$JAVA_HOME"   # /usr/lib/jvm/gvm.jdk25 on this machine; must contain lib/modules
./smoke -Djava.home="$JDK" Hello.java -d out; echo "exit=$?"
ls out/Hello.class
./smoke -Djava.home="$JDK" --version; echo "exit=$?"
printf 'public class Broken { this is not java }' > Broken.java
./smoke -Djava.home="$JDK" Broken.java; echo "exit=$?"
```

Expected:
1. First run: `exit=0` and `out/Hello.class` exists.
2. `--version`: prints a `javac ...` version string, `exit=0`.
3. `Broken.java`: prints javac diagnostics containing `error`, nonzero exit.

Contingency: if diagnostics crash with `MissingResourceException`, add
`-H:IncludeResourceBundles=com.sun.tools.javac.resources.compiler` and
`-H:IncludeResourceBundles=com.sun.tools.javac.resources.javac` to the flags block,
rebuild, and re-verify.

- [ ] **Step 6: Commit**

```bash
cd /home/sam/workspace/labs/SUPERCRITICAL
git add crates/madura_javac/src/JavacInvoker.kt crates/madura_javac/elide.pkl
git commit -m "feat(madura_javac): invoke the system java compiler with passthrough args

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

(If the Kotlin swap was already committed in an earlier round, commit only the
elide.pkl change with message
`build(madura_javac): enable JRT filesystem for javac platform metadata`.)

---

### Task 3: `madura_javac` crate — build.rs orchestration + safe FFI wrapper

**Files:**
- Modify: `crates/madura_javac/Cargo.toml` (add `links`)
- Modify: `crates/madura_javac/build.rs` (replace the empty stub)
- Create: `crates/madura_javac/src/lib.rs`
- Test: `crates/madura_javac/tests/invoke.rs`

**Interfaces:**
- Consumes: `run_main` from the artifact built in Task 2; `elide` on PATH.
- Produces (used by Task 4):
  - `madura_javac::invoke(args: impl IntoIterator<Item = OsString>) -> Result<i32, NulArgError>`
  - `madura_javac::NulArgError { pub arg: OsString }` (implements `Display` + `Error`)
  - Build metadata: `DEP_MADURA_JAVAC_LIB_DIR` env var for dependent build scripts,
    pointing at a directory containing `libmadura-javac.so`.

- [ ] **Step 1: Write the failing test**

Create `crates/madura_javac/tests/invoke.rs`:

```rust
use std::ffi::OsString;

#[test]
fn rejects_interior_nul_arguments() {
    let err = madura_javac::invoke([OsString::from("Foo\0.java")])
        .expect_err("interior NUL must be rejected before reaching FFI");
    assert_eq!(err.arg, OsString::from("Foo\0.java"));
}
```

(This test exercises only the error path, which returns before any FFI call — safe to
run in-process. The success path is covered end-to-end in Task 4, because `run_main`
may terminate the calling process.)

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p madura_javac`
Expected: FAIL — cargo rejects the manifest outright ("no targets specified in the
manifest" or similar) because the package still has no `src/lib.rs`; integration
tests alone don't constitute a target.

- [ ] **Step 3: Add the `links` key**

In `crates/madura_javac/Cargo.toml`, replace the `[package]` section with:

```toml
[package]
name = "madura_javac"
version = "0.1.0"
edition = "2024"
links = "madura-javac"
build = "build.rs"
```

- [ ] **Step 4: Write build.rs**

Replace the contents of `crates/madura_javac/build.rs` with:

```rust
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo::rerun-if-changed=src/JavacInvoker.kt");
    println!("cargo::rerun-if-changed=elide.pkl");

    // Run from the crate dir, NOT via `-p` from elsewhere: native-image resolves
    // its output dir against the process cwd and aborts otherwise (found in Task 2).
    let output = Command::new("elide")
        .arg("build")
        .current_dir(&manifest_dir)
        .output()
        .expect("failed to run `elide` — is it installed and on PATH? (try `mise install`)");
    // Forward elide's output to stderr: stdout is reserved for cargo directives.
    eprint!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "`elide build` failed: {}", output.status);

    let so = manifest_dir.join(".dev/artifacts/native-image/madura-javac.so");
    assert!(so.is_file(), "missing native-image artifact: {}", so.display());

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
```

- [ ] **Step 5: Write src/lib.rs**

Create `crates/madura_javac/src/lib.rs`:

```rust
use std::error::Error;
use std::ffi::{CString, OsString};
use std::fmt;
use std::os::raw::{c_char, c_int};
use std::os::unix::ffi::OsStrExt;

unsafe extern "C" {
    fn run_main(argc: c_int, argv: *mut *mut c_char) -> c_int;
}

/// An argument could not be passed to the compiler because it contains an
/// interior NUL byte.
#[derive(Debug)]
pub struct NulArgError {
    pub arg: OsString,
}

impl fmt::Display for NulArgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "argument contains an interior NUL byte: {:?}", self.arg)
    }
}

impl Error for NulArgError {}

/// Invoke the embedded `javac` with the given arguments, as if invoked from
/// the command line, returning the compiler's exit code.
///
/// The image may terminate the process directly (`System.exit`) instead of
/// returning; treat this as the final call of the process.
pub fn invoke<I>(args: I) -> Result<i32, NulArgError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut argv_owned = vec![CString::new("madura").expect("static name has no NUL")];
    for arg in args {
        match CString::new(arg.as_os_str().as_bytes()) {
            Ok(c) => argv_owned.push(c),
            Err(_) => return Err(NulArgError { arg }),
        }
    }
    let mut argv: Vec<*mut c_char> = argv_owned.iter().map(|c| c.as_ptr().cast_mut()).collect();
    let argc = argv.len() as c_int;
    argv.push(std::ptr::null_mut());
    Ok(unsafe { run_main(argc, argv.as_mut_ptr()) })
}
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p madura_javac`
Expected: PASS (1 test). This also proves the whole pipeline: build.rs ran elide,
staged `libmadura-javac.so` into `OUT_DIR`, the test binary linked `-l madura-javac`,
and the loader resolved it via the absolute rpath.

- [ ] **Step 7: Commit**

```bash
git add crates/madura_javac/Cargo.toml crates/madura_javac/build.rs \
        crates/madura_javac/src/lib.rs crates/madura_javac/tests/invoke.rs
git commit -m "feat(madura_javac): build.rs elide orchestration + safe run_main wrapper

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: `madura` bin — hermetic javac-compatible CLI with e2e tests

> **AMENDED for the hermetic dist decision:** the binary resolves its dist root from
> its own executable path and injects `-Djava.home=<root>`; `build.rs` stages
> `target/lib/{libmadura-javac.so,modules,ct.sym}` so the dev tree mirrors the dist
> shape `<root>/{bin,lib}`. No JDK is required at runtime; `$JAVA_HOME` is required
> at **build** time to source `modules`/`ct.sym`.
>
> **AMENDED again (e2e finding, user-adjudicated):** `version_flag_prints_javac_version`
> exposed that `ToolProvider...run(...)` ignores stdout — javac's `JavacTool.run` sends
> the version banner and usage to stderr, unlike the real `javac` launcher. Fix (user
> approved): `crates/madura_javac/src/JavacInvoker.kt` switches its body to
> `System.exit(com.sun.tools.javac.Main.compile(args))` (import `com.sun.tools.javac.Main`;
> the null-check is dropped — a missing `jdk.compiler` fails the image build instead).
> This is a Task 4 fix-round change to Task 2's file, plus an image rebuild. The fix
> round also commits the `Cargo.lock` refresh from the dependency swap.

**Files:**
- Test: `crates/madura/tests/e2e.rs` (create — written first)
- Modify: `crates/madura/Cargo.toml`
- Create: `crates/madura/build.rs`
- Modify: `crates/madura/src/main.rs` (replace hello-world)

**Interfaces:**
- Consumes: `madura_javac::invoke` / `NulArgError` (Task 3);
  `DEP_MADURA_JAVAC_LIB_DIR` containing `libmadura-javac.so` (Task 3); the image's
  runtime `-Djava.home` argv handling (Task 2).
- Produces: the `madura` binary — argv passthrough with a single injected
  `-Djava.home=<dist root>`, javac exit codes, dist-shaped staging under
  `target/lib/`, rpaths `$ORIGIN/../lib` + absolute dep lib dir.

- [ ] **Step 1: Write the failing e2e tests**

Create `crates/madura/tests/e2e.rs`:

```rust
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
    fs::write(dir.join("Broken.java"), "public class Broken { this is not java }").unwrap();
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
```

(If `compiles_for_older_release_via_ct_sym` fails with a zip/filesystem-provider
error, the image lacks `jdk.zipfs`: report it in your task report as a concern and
leave the test `#[ignore]`d with a comment naming the missing module — enabling it
belongs to the image config, not this crate.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p madura`
Expected: FAIL — all four tests. The current binary prints `Hello, world!` and exits
0 regardless of args, so no `.class` file appears, the invalid case "succeeds", and
`--version` output lacks `javac`.

- [ ] **Step 3: Swap dependencies in Cargo.toml**

In `crates/madura/Cargo.toml`, replace the `[dependencies]` section with:

```toml
[dependencies]
madura_javac = { path = "../madura_javac" }
mimalloc = { workspace = true, features = ["v2"] }
```

(`getargs` is dropped per the spec: pure passthrough needs no arg parsing. Leave it in
`[workspace.dependencies]` in the root `Cargo.toml` — removing it there is optional
and out of scope.)

- [ ] **Step 4: Create build.rs**

Create `crates/madura/build.rs`:

```rust
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Copy `src` to `dst` only when missing or size-mismatched (lib/modules is
// ~180MB; unconditional copies would slow every build).
fn stage(src: &Path, dst: &Path) {
    let fresh = match (fs::metadata(src), fs::metadata(dst)) {
        (Ok(s), Ok(d)) => s.len() == d.len(),
        _ => false,
    };
    if !fresh {
        fs::copy(src, dst).unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    }
}

fn main() {
    let lib_dir = PathBuf::from(
        env::var("DEP_MADURA_JAVAC_LIB_DIR")
            .expect("DEP_MADURA_JAVAC_LIB_DIR is set by madura_javac's build script"),
    );
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Hermetic platform metadata is sourced from the build-time JDK.
    let java_home = PathBuf::from(env::var("JAVA_HOME").expect(
        "JAVA_HOME must point at a JDK at build time (source of lib/modules and lib/ct.sym)",
    ));
    let modules = java_home.join("lib/modules");
    let ct_sym = java_home.join("lib/ct.sym");
    assert!(modules.is_file(), "not a jimage-bearing JDK: {}", modules.display());
    assert!(ct_sym.is_file(), "JDK lacks ct.sym: {}", ct_sym.display());

    // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out; three ancestors up is
    // target/<profile>; its parent is the dist root in the dev tree, so the
    // staged lib/ sits at target/lib — matching the $ORIGIN/../lib rpath from
    // target/<profile>/madura, and mirroring the shipped <root>/{bin,lib} shape.
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR nested under the profile directory");
    let staged_lib = profile_dir.parent().expect("profile dir has a parent").join("lib");
    fs::create_dir_all(&staged_lib).unwrap();

    // The .so is small and $ORIGIN/../lib is the FIRST rpath entry, so this
    // copy must be unconditional — a stale staged copy would shadow OUT_DIR.
    fs::copy(lib_dir.join("libmadura-javac.so"), staged_lib.join("libmadura-javac.so")).unwrap();
    stage(&modules, &staged_lib.join("modules"));
    stage(&ct_sym, &staged_lib.join("ct.sym"));

    println!("cargo::rerun-if-env-changed=JAVA_HOME");
    println!("cargo::rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib");
    println!("cargo::rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
}
```


- [ ] **Step 5: Replace src/main.rs**

Replace the contents of `crates/madura/src/main.rs` with:

```rust
use std::ffi::OsString;
use std::path::PathBuf;
use std::process;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// The dist is hermetic: <root>/bin/madura (or target/<profile>/madura in the
// dev tree) finds platform metadata at <root>/lib/{modules,ct.sym}. javac
// reads them via -Djava.home=<root>, which the native image parses from argv
// before main.
fn dist_root() -> Result<PathBuf, String> {
    let exe = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .map_err(|e| format!("cannot resolve own executable path: {e}"))?;
    let root = exe
        .parent()
        .and_then(|bin| bin.parent())
        .ok_or_else(|| format!("executable has no dist root: {}", exe.display()))?;
    if !root.join("lib/modules").is_file() {
        return Err(format!(
            "missing platform image: {} (madura must live in a <root>/bin or target/<profile> layout with <root>/lib/modules)",
            root.join("lib/modules").display(),
        ));
    }
    Ok(root.to_path_buf())
}

fn main() {
    let root = match dist_root() {
        Ok(root) => root,
        Err(msg) => {
            eprintln!("madura: {msg}");
            process::exit(1);
        }
    };
    let mut java_home = OsString::from("-Djava.home=");
    java_home.push(root.as_os_str());
    let args = std::iter::once(java_home).chain(std::env::args_os().skip(1));
    match madura_javac::invoke(args) {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("madura: {err}");
            process::exit(1);
        }
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p madura`
Expected: PASS (4 tests). The spawned binary (`target/debug/madura`) resolves its
dist root to `target/`, finds `libmadura-javac.so` via the `$ORIGIN/../lib` rpath
(`target/lib/`, staged by build.rs), and javac reads `target/lib/modules` — all with
`JAVA_HOME` removed from the environment.

- [ ] **Step 7: Manual sanity check**

```bash
env -u JAVA_HOME cargo run -q -p madura -- --version
env -u JAVA_HOME cargo run -q -p madura; echo "exit=$?"
```

Expected: first prints a `javac ...` version string; second prints javac usage/help
text with a nonzero exit (no args → same behavior as bare `javac`) — confirm it is
usage text, not a crash. (`env -u JAVA_HOME` double-checks hermeticity: only the
cargo *build* needs JAVA_HOME, and cargo won't rebuild here since nothing changed —
if it tries to, run plain `cargo build -p madura` first.)

- [ ] **Step 8: Commit**

```bash
git add crates/madura/Cargo.toml crates/madura/build.rs \
        crates/madura/src/main.rs crates/madura/tests/e2e.rs
git commit -m "feat(madura): javac-compatible bin over madura-javac.so with e2e tests

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: Dist packaging, workspace verification + docs

**Files:**
- Create: `scripts/make-dist.sh` (assemble the hermetic distribution)
- Modify: `crates/madura_javac/README.md` (document the build flow)

**Interfaces:**
- Consumes: everything from Tasks 2–4; the staged `target/lib/` contents.
- Produces: `target/dist/{bin/madura, lib/{libmadura-javac.so,modules,ct.sym}}`;
  green workspace build/test/clippy; README documenting how the pieces fit.

- [ ] **Step 1: Full workspace verification**

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

Expected: build and tests green (5 tests total: 1 in madura_javac + 4 e2e); clippy
clean. Fix any clippy warnings in files this plan created (do not touch unrelated
config).

- [ ] **Step 2: Create the dist packaging script**

Create `scripts/make-dist.sh` (mark executable):

```bash
#!/usr/bin/env bash
# Assemble the hermetic madura distribution: <root>/{bin,lib}.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
profile="${1:-release}"
dist="$repo/target/dist"

cargo build --profile "$profile" -p madura

rm -rf "$dist"
mkdir -p "$dist/bin" "$dist/lib"
cp "$repo/target/$profile/madura" "$dist/bin/madura"
cp "$repo/target/lib/libmadura-javac.so" "$dist/lib/"
cp "$repo/target/lib/modules" "$dist/lib/"
cp "$repo/target/lib/ct.sym" "$dist/lib/"

echo "dist assembled at $dist"
```

Note: `cargo build --profile release` places output in `target/release`; the staged
`target/lib` is shared across profiles (build.rs stages it on every madura build).

- [ ] **Step 3: Verify the dist is hermetic**

```bash
./scripts/make-dist.sh
cd /tmp/claude-1000/madura-smoke
env -u JAVA_HOME /home/sam/workspace/labs/SUPERCRITICAL/target/dist/bin/madura --version
rm -rf out-dist
env -u JAVA_HOME /home/sam/workspace/labs/SUPERCRITICAL/target/dist/bin/madura Hello.java -d out-dist
ls out-dist/Hello.class
```

Expected: version string prints; `Hello.class` produced — with no `JAVA_HOME` in the
environment and no JDK consulted (everything from `target/dist/lib`).

- [ ] **Step 4: Document the build flow in the crate README**

Replace/append in `crates/madura_javac/README.md` so it contains:

```markdown
# madura_javac

FFI crate wrapping `madura-javac.so`, a GraalVM native-image shared library that
embeds the system Java compiler. The Kotlin entrypoint (`src/JavacInvoker.kt`)
is compiled against JDK core libraries only (no Kotlin stdlib, no reflection).

## Build flow

`cargo build` drives everything:

1. `build.rs` runs `elide build -p <this crate>` (re-run only when
   `src/JavacInvoker.kt` or `elide.pkl` change; native-image takes minutes).
2. The artifact `.dev/artifacts/native-image/madura-javac.so` is staged into
   `OUT_DIR` as `libmadura-javac.so` (the artifact ships without a `lib` prefix
   or SONAME; the renamed copy is the canonical link/runtime name).
3. Cargo links `-l madura-javac` and exposes the staging dir to dependents as
   `DEP_MADURA_JAVAC_LIB_DIR` (via the `links = "madura-javac"` key).

The `madura` bin crate stages a hermetic, dist-shaped `target/lib/` next to the
profile dir — `libmadura-javac.so` plus `modules` and `ct.sym` from the
build-time `$JAVA_HOME` — and links with rpaths `$ORIGIN/../lib` + the absolute
staging dir. At runtime the binary injects `-Djava.home=<dist root>` (resolved
from its own path; the native image parses `-D` argv before `main`), so
`target/<profile>/madura`, its tests, and the shipped `<root>/{bin,lib}` dist
all run with **no JDK and no environment setup** (`-H:+AllowJRTFileSystem` in
`elide.pkl` provides jrt access to `lib/modules`). `scripts/make-dist.sh`
assembles `target/dist`.

## API

`madura_javac::invoke(args) -> Result<i32, NulArgError>` — invokes the embedded
`javac` with passthrough argv and returns its exit code. The image may terminate
the process directly (`System.exit`); treat `invoke` as the process's final call.

Requires `elide` on PATH (`mise install`) and `$JAVA_HOME` at build time.
```

- [ ] **Step 5: Commit**

```bash
git add scripts/make-dist.sh crates/madura_javac/README.md
git commit -m "feat(dist): hermetic dist packaging + build-flow docs

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```
