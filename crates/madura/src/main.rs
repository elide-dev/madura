use std::process;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// The dist root (java.home) is resolved here, binary-relative, and passed
// into the image's `compile_javac` entry point, which trusts it as-is.
fn main() {
    // by default, we assume a full compilation cycle
    let mut check = false;
    let mut skip_argc = 1;

    // however, if the first argument is `check`, we flip a bool to enter check-only mode,
    // and skip an additional argument; `compile` names the default mode explicitly. Any
    // other first argument belongs to javac itself, so the binary behaves as `javac` does.
    match std::env::args_os().nth(1) {
        Some(cmd) if cmd == "check" => {
            check = true;
            skip_argc = 2;
        }
        Some(cmd) if cmd == "compile" => skip_argc = 2,
        _ => {}
    }

    match madura_javac::invoke(check, std::env::args_os().skip(skip_argc)) {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("madura: {err}");
            process::exit(err.exit_code());
        }
    }
}
