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
