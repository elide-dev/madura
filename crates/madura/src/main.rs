use std::process;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// The dist root (java.home) is resolved here, binary-relative, and passed
// into the image's `compile_javac` entry point, which trusts it as-is.
fn main() {
    match madura_javac::invoke(std::env::args_os().skip(1)) {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("madura: {err}");
            process::exit(err.exit_code());
        }
    }
}
