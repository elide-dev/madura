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
