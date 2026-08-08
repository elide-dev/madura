#![cfg(feature = "native")]

use std::ffi::OsString;

#[test]
fn rejects_interior_nul_arguments() {
    let err = madura_javac::invoke([OsString::from("Foo\0.java")])
        .expect_err("interior NUL must be rejected before reaching FFI");
    assert_eq!(err.arg, OsString::from("Foo\0.java"));
}
