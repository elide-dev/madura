#![cfg(feature = "native")]

use std::ffi::OsString;

#[test]
fn rejects_interior_nul_arguments() {
    let err = madura_javac::invoke(false, [OsString::from("Foo\0.java")])
        .expect_err("interior NUL must be rejected before reaching FFI");
    match err {
        madura_javac::InvokeError::NulArg(nul) => {
            assert_eq!(nul.arg, OsString::from("Foo\0.java"));
        }
        other => panic!("expected NulArg, got: {other}"),
    }
}
