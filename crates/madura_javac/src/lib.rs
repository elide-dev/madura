use std::error::Error;
use std::ffi::{CString, OsString};
use std::fmt;
#[cfg(feature = "native")]
use std::os::raw::{c_char, c_int};
use std::os::unix::ffi::OsStrExt;

#[cfg(feature = "native")]
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

/// Marshal `args` into the owned C strings that back the compiler's `argv`,
/// prefixed with the program name.
///
/// This is the whole Rust-side cost of a `madura` invocation: every argument is
/// copied into a NUL-terminated allocation, and a single interior NUL anywhere
/// in the command line aborts the whole marshalling before any FFI happens.
pub fn argv<I>(args: I) -> Result<Vec<CString>, NulArgError>
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
    Ok(argv_owned)
}

/// Invoke the embedded `javac` with the given arguments, as if invoked from
/// the command line, returning the compiler's exit code.
///
/// The image may terminate the process directly (`System.exit`) instead of
/// returning; treat this as the final call of the process.
#[cfg(feature = "native")]
pub fn invoke<I>(args: I) -> Result<i32, NulArgError>
where
    I: IntoIterator<Item = OsString>,
{
    let argv_owned = argv(args)?;
    let mut argv: Vec<*mut c_char> = argv_owned.iter().map(|c| c.as_ptr().cast_mut()).collect();
    let argc = argv.len() as c_int;
    argv.push(std::ptr::null_mut());
    Ok(unsafe { run_main(argc, argv.as_mut_ptr()) })
}
