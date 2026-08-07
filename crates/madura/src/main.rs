use std::process;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// Pure passthrough: all dist-root/java.home resolution happens inside the
// image's Kotlin entry (binary-relative via ProcessHandle).
fn main() {
    match madura_javac::invoke(std::env::args_os().skip(1)) {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("madura: {err}");
            process::exit(1);
        }
    }
}
