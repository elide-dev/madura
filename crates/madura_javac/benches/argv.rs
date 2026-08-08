//! Benchmarks for the Rust side of a `madura` invocation.
//!
//! Everything `madura` does before control crosses into the native image is
//! argument marshalling: each `OsString` from the command line is copied into an
//! owned `CString`, and the whole batch is rejected if any argument carries an
//! interior NUL. `javac` command lines routinely reach thousands of arguments
//! (one per source file, plus a class path made of long paths), so this layer is
//! measured across the shapes a real build produces.

use std::ffi::{CString, OsString};

use divan::{Bencher, black_box};

fn main() {
    divan::main();
}

/// Argument counts spanning a single-file compile up to a full-module batch.
const SOURCE_COUNTS: &[usize] = &[1, 32, 512, 4096];

/// A `javac`-shaped source path, deep enough to look like a real package tree.
fn source_path(index: usize) -> OsString {
    OsString::from(format!(
        "src/main/java/dev/elide/jvm/internal/generated/module{}/Class{index}.java",
        index % 32,
    ))
}

/// The flags a release build passes ahead of the source list.
fn leading_flags() -> Vec<OsString> {
    [
        "--release",
        "21",
        "-d",
        "target/classes",
        "-encoding",
        "UTF-8",
        "-Xlint:all",
        "-Werror",
        "-parameters",
        "-g",
        "-classpath",
        "target/classes:target/deps/kotlin-stdlib-2.4.0.jar:target/deps/annotations-26.0.2.jar",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

/// A full command line: flags followed by `count` source files.
fn command_line(count: usize) -> Vec<OsString> {
    let mut args = leading_flags();
    args.extend((0..count).map(source_path));
    args
}

/// Marshalling a whole `javac` command line, scaled by source count.
#[divan::bench(args = SOURCE_COUNTS)]
fn command_line_marshal(bencher: Bencher, count: usize) {
    let args = command_line(count);
    bencher
        .with_inputs(|| args.clone())
        .bench_values(|args| black_box(madura_javac::argv(black_box(args))).map(|v| v.len()));
}

/// The shortest possible invocation (`madura --version`), where fixed overhead
/// dominates.
#[divan::bench]
fn version_flag(bencher: Bencher) {
    let args = vec![OsString::from("--version")];
    bencher
        .with_inputs(|| args.clone())
        .bench_values(|args| black_box(madura_javac::argv(black_box(args))).map(|v| v.len()));
}

/// A single-file compile — the common interactive case.
#[divan::bench]
fn single_file_compile(bencher: Bencher) {
    let args: Vec<OsString> = ["Hello.java", "-d", "out"]
        .into_iter()
        .map(OsString::from)
        .collect();
    bencher
        .with_inputs(|| args.clone())
        .bench_values(|args| black_box(madura_javac::argv(black_box(args))).map(|v| v.len()));
}

/// Very long individual arguments (a class path assembled from a deep dependency
/// graph), which stress the per-argument copy rather than the per-argument
/// bookkeeping.
#[divan::bench(args = [64, 1024])]
fn long_classpath(bencher: Bencher, entries: usize) {
    let classpath = (0..entries)
        .map(|i| {
            format!("/home/runner/.m2/repository/dev/elide/module{i}/1.0.0/module{i}-1.0.0.jar")
        })
        .collect::<Vec<_>>()
        .join(":");
    let args: Vec<OsString> = vec![
        OsString::from("-classpath"),
        OsString::from(classpath),
        OsString::from("Hello.java"),
    ];
    bencher
        .with_inputs(|| args.clone())
        .bench_values(|args| black_box(madura_javac::argv(black_box(args))).map(|v| v.len()));
}

/// Non-ASCII arguments: `OsString` bytes are copied verbatim, so multi-byte
/// paths simply mean more bytes per argument.
#[divan::bench]
fn unicode_paths(bencher: Bencher) {
    let args: Vec<OsString> = (0..256)
        .map(|i| OsString::from(format!("src/主要/ソース/пакет/módulo{i}/Класс{i}.java")))
        .collect();
    bencher
        .with_inputs(|| args.clone())
        .bench_values(|args| black_box(madura_javac::argv(black_box(args))).map(|v| v.len()));
}

/// The rejection path: an interior NUL late in a large command line must be
/// detected, and everything marshalled so far dropped.
#[divan::bench(args = [32, 1024])]
fn rejects_interior_nul(bencher: Bencher, count: usize) {
    let mut args = command_line(count);
    args.push(OsString::from("Bad\0Name.java"));
    bencher
        .with_inputs(|| args.clone())
        .bench_values(|args| black_box(madura_javac::argv(black_box(args))).is_err());
}

/// Building the raw `argv` pointer table on top of the owned strings — the last
/// step before the FFI boundary.
#[divan::bench(args = SOURCE_COUNTS)]
fn argv_pointer_table(bencher: Bencher, count: usize) {
    let owned: Vec<CString> = madura_javac::argv(command_line(count)).unwrap();
    bencher.bench(|| {
        let mut argv: Vec<*mut std::os::raw::c_char> = black_box(&owned)
            .iter()
            .map(|c| c.as_ptr().cast_mut())
            .collect();
        argv.push(std::ptr::null_mut());
        black_box(argv.len())
    });
}
