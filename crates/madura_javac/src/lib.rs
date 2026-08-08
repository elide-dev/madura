use std::error::Error;
use std::ffi::{CString, OsString};
use std::fmt;
#[cfg(feature = "native")]
use std::os::raw::{c_char, c_int, c_void};
use std::os::unix::ffi::OsStrExt;
#[cfg(feature = "native")]
use std::path::{Path, PathBuf};

// graal_isolate_t / graal_isolatethread_t are opaque; compile_javac is the
// image's CEntryPoint (madura-javac.h):
//   int32_t compile_javac(graal_isolatethread_t*, char* bin, char* home,
//                         int32_t argc, char** argv, bool check_only);
#[cfg(feature = "native")]
unsafe extern "C" {
    fn graal_create_isolate(
        params: *mut c_void,
        isolate: *mut *mut c_void,
        thread: *mut *mut c_void,
    ) -> c_int;
    fn graal_get_current_thread(isolate: *mut c_void) -> *mut c_void;
    fn graal_attach_thread(isolate: *mut c_void, thread: *mut *mut c_void) -> c_int;
    fn compile_javac(
        thread: *mut c_void,
        bin_path: *mut c_char,
        home_path: *mut c_char,
        arg_count: c_int,
        arg_array: *mut *mut c_char,
        check_only: bool,
    ) -> c_int;
}

/// The process-wide isolate, created on first use and never torn down.
///
/// One isolate per process is a hard constraint, not a convenience: the image
/// is built with `--gc=G1`, and SVM's G1 supports a single isolate per process
/// lifetime — a second `graal_create_isolate` aborts (`SVMIsolateData::
/// _heap_base == nullptr`), even after tearing the first one down. Threads
/// other than the creating one attach on demand in [`isolate_thread`].
#[cfg(feature = "native")]
static ISOLATE: std::sync::Mutex<Option<usize>> = std::sync::Mutex::new(None);

/// The current thread's entry token for the process isolate, creating the
/// isolate on first use and attaching this thread if it is new to it.
#[cfg(feature = "native")]
fn isolate_thread() -> Result<*mut c_void, InvokeError> {
    let mut guard = ISOLATE.lock().expect("isolate creation never panics");
    let isolate = match *guard {
        Some(isolate) => isolate as *mut c_void,
        None => {
            let mut isolate = std::ptr::null_mut();
            let mut thread = std::ptr::null_mut();
            let rc =
                unsafe { graal_create_isolate(std::ptr::null_mut(), &mut isolate, &mut thread) };
            if rc != 0 {
                return Err(InvokeError::IsolateInit(rc));
            }
            *guard = Some(isolate as usize);
            // The creating thread is already attached.
            return Ok(thread);
        }
    };
    drop(guard);

    let thread = unsafe { graal_get_current_thread(isolate) };
    if !thread.is_null() {
        return Ok(thread);
    }
    let mut thread = std::ptr::null_mut();
    let rc = unsafe { graal_attach_thread(isolate, &mut thread) };
    if rc != 0 {
        return Err(InvokeError::IsolateInit(rc));
    }
    Ok(thread)
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

/// A `madura` invocation could not reach the compiler.
#[derive(Debug)]
pub enum InvokeError {
    /// An argument carried an interior NUL byte.
    NulArg(NulArgError),
    /// The shipped platform image (`<root>/lib/modules`) was not found
    /// relative to the running binary.
    NoPlatformImage { exe: Option<OsString> },
    /// `graal_create_isolate` failed with the given code.
    IsolateInit(i32),
}

impl fmt::Display for InvokeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NulArg(err) => err.fmt(f),
            Self::NoPlatformImage { exe } => write!(
                f,
                "cannot locate the shipped platform image at <root>/lib/modules (binary: {})",
                exe.as_deref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .as_deref()
                    .unwrap_or("<unknown>"),
            ),
            Self::IsolateInit(code) => {
                write!(f, "failed to create the GraalVM isolate (code {code})")
            }
        }
    }
}

impl Error for InvokeError {}

impl From<NulArgError> for InvokeError {
    fn from(err: NulArgError) -> Self {
        Self::NulArg(err)
    }
}

impl InvokeError {
    /// Process exit code for this failure: NUL arguments are a caller mistake
    /// (1); a missing platform image or isolate failure is an environment
    /// problem (2, matching the previous in-image diagnostic path).
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NulArg(_) => 1,
            _ => 2,
        }
    }
}

/// Marshal `args` into the owned C strings that back the compiler's `argv`.
///
/// This is the whole Rust-side cost of a `madura` invocation: every argument is
/// copied into a NUL-terminated allocation, and a single interior NUL anywhere
/// in the command line aborts the whole marshalling before any FFI happens.
pub fn argv<I>(args: I) -> Result<Vec<CString>, NulArgError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut argv_owned = Vec::new();
    for arg in args {
        match CString::new(arg.as_os_str().as_bytes()) {
            Ok(c) => argv_owned.push(c),
            Err(_) => return Err(NulArgError { arg }),
        }
    }
    Ok(argv_owned)
}

/// Resolve the dist root from the binary's own path (`<root>/bin/madura`, or
/// `target/<profile>/madura` in the dev tree), requiring `<root>/lib/modules`
/// — the same binary-relative rule as the image's own `main()`.
#[cfg(feature = "native")]
fn platform_home(exe: &Path) -> Option<PathBuf> {
    let real = exe.canonicalize().ok()?;
    let root = real.parent()?.parent()?;
    root.join("lib")
        .join("modules")
        .is_file()
        .then(|| root.to_path_buf())
}

/// Invoke the embedded `javac` with the given arguments, as if invoked from
/// the command line, returning the compiler's exit code.
///
/// The dist root is resolved binary-relative here and handed to the image as
/// its `java.home`; the image trusts it as-is.
#[cfg(feature = "native")]
pub fn invoke<I>(check_only: bool, args: I) -> Result<i32, InvokeError>
where
    I: IntoIterator<Item = OsString>,
{
    // Caller errors (bad arguments) before environment errors.
    let argv_owned = argv(args)?;

    let exe = std::env::current_exe().ok();
    let home =
        exe.as_deref()
            .and_then(platform_home)
            .ok_or_else(|| InvokeError::NoPlatformImage {
                exe: exe.as_deref().map(|p| p.as_os_str().to_owned()),
            })?;

    call(check_only, &home, &argv_owned)
}

/// Invoke the embedded `javac` against an explicit platform-image root — the
/// directory whose `lib/modules` becomes `java.home`, trusted as-is.
///
/// Hosts that already know where the platform image lives call this directly:
/// applications embedding the shared library, and the in-process benchmarks,
/// whose binaries do not sit in a dist layout.
#[cfg(feature = "native")]
pub fn invoke_with<I>(check_only: bool, home: &Path, args: I) -> Result<i32, InvokeError>
where
    I: IntoIterator<Item = OsString>,
{
    call(check_only, home, &argv(args)?)
}

#[cfg(feature = "native")]
fn call(check_only: bool, home: &Path, argv_owned: &[CString]) -> Result<i32, InvokeError> {
    // Paths on Linux cannot contain NUL, so these conversions cannot fail.
    let bin = std::env::current_exe()
        .ok()
        .map(|p| CString::new(p.as_os_str().as_bytes()).expect("paths have no NUL"));
    let home = CString::new(home.as_os_str().as_bytes()).expect("paths have no NUL");
    let mut arg_ptrs: Vec<*mut c_char> = argv_owned.iter().map(|c| c.as_ptr().cast_mut()).collect();
    let argc = arg_ptrs.len() as c_int;

    let thread = isolate_thread()?;

    // No isolate teardown, ever: G1 would refuse a replacement isolate anyway
    // (see [`ISOLATE`]), and for the single-shot binary a teardown would only
    // add GC and unmap work before the process exits.
    Ok(unsafe {
        compile_javac(
            thread,
            bin.as_ref()
                .map_or(std::ptr::null_mut(), |c| c.as_ptr().cast_mut()),
            home.as_ptr().cast_mut(),
            argc,
            arg_ptrs.as_mut_ptr(),
            check_only,
        )
    })
}
