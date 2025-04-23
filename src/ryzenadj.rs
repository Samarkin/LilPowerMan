use libloading::os::windows::Symbol;
use libloading::Library;
use std::ffi::c_void;
use std::fmt::{Debug, Display, Formatter};

#[repr(transparent)]
#[derive(Clone, Copy)]
struct RyzenAccess(*mut c_void);

impl RyzenAccess {
    fn is_invalid(&self) -> bool {
        self.0.is_null()
    }
}

pub enum Error {
    LibraryLoading(libloading::Error),
    InitFailure,
    FamilyNotSupported,
    SMUTimeout,
    SMUUnsupported,
    SMURejected,
    InvalidMemoryAccess,
    UnknownErrorCode(i32),
}

impl Error {
    fn check(errorcode: i32) -> Result<(), Self> {
        match errorcode {
            0 => Ok(()),
            -1 => Err(Self::FamilyNotSupported),
            -2 => Err(Self::SMUTimeout),
            -3 => Err(Self::SMUUnsupported),
            -4 => Err(Self::SMURejected),
            -5 => Err(Self::InvalidMemoryAccess),
            x => Err(Self::UnknownErrorCode(x)),
        }
    }
}

impl From<libloading::Error> for Error {
    fn from(error: libloading::Error) -> Self {
        Self::LibraryLoading(error)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LibraryLoading(inner) => write!(f, "Failed to load library: {inner}"),
            Self::InitFailure => write!(f, "Failed to init RyzenAdj"),
            Self::FamilyNotSupported => write!(f, "APU family is not supported"),
            Self::SMUTimeout => write!(f, "SMU Timeout"),
            Self::SMUUnsupported => write!(f, "SMU operation is unsupported"),
            Self::SMURejected => write!(f, "SMU operation is rejected"),
            Self::InvalidMemoryAccess => write!(f, "Memory access error"),
            Self::UnknownErrorCode(x) => write!(f, "Unknown error code {x}"),
        }
    }
}

impl std::error::Error for Error {}

struct Native {
    /// # Safety
    ///
    /// Caller should ensure library is still loaded.
    init_ryzenadj: Symbol<unsafe extern "C" fn() -> RyzenAccess>,
    /// # Safety
    ///
    /// Caller should ensure library is still loaded and `RyzenAccess` instance has not been cleaned up.
    refresh_table: Symbol<unsafe extern "C" fn(RyzenAccess) -> i32>,
    /// # Safety
    ///
    /// Caller should ensure library is still loaded and `RyzenAccess` instance has not been cleaned up.
    /// Caller should refresh table before accessing any values.
    get_fast_limit: Symbol<unsafe extern "C" fn(RyzenAccess) -> f32>,
    /// # Safety
    ///
    /// Caller should ensure library is still loaded and `RyzenAccess` instance has not been cleaned up.
    set_stapm_limit: Symbol<unsafe extern "C" fn(RyzenAccess, u32) -> i32>,
    /// # Safety
    ///
    /// Caller should ensure library is still loaded and `RyzenAccess` instance has not been cleaned up.
    set_fast_limit: Symbol<unsafe extern "C" fn(RyzenAccess, u32) -> i32>,
    /// # Safety
    ///
    /// Caller should ensure library is still loaded and `RyzenAccess` instance has not been cleaned up.
    set_slow_limit: Symbol<unsafe extern "C" fn(RyzenAccess, u32) -> i32>,
    /// # Safety
    ///
    /// Caller should ensure library is still loaded.
    /// Caller should not call this more than once per `RyzenAccess` instance.
    cleanup_ryzenadj: Symbol<unsafe extern "C" fn(RyzenAccess)>,
}

pub struct RyzenAdjTable<'lib> {
    main: &'lib RyzenAdj,
}

impl<'lib> RyzenAdjTable<'lib> {
    /// Returns current TDP fast limit in milliwatts.
    pub fn get_fast_limit(&self) -> u32 {
        debug!("Reading TDP fast limit");
        // SAFETY: Validity of Library and `RyzenAccess` pointers is guaranteed
        // for the lifetime of `RyzenAdj` instance
        // The table has been refreshed as part of `RyzenAdjTable` initialization.
        let value = unsafe { (self.main.native.get_fast_limit)(self.main.ry) };
        (value * 1000f32) as u32
    }
}

/// # Safety
///
/// Caller should ensure symbol declaration matches the provided type T.
#[inline] // This method is primarily needed to simplify type inference
unsafe fn get_native_symbol<T>(
    library: &Library,
    symbol: &[u8],
) -> Result<Symbol<T>, libloading::Error> {
    Ok(unsafe { library.get::<T>(symbol)?.into_raw() })
}

pub struct RyzenAdj {
    _library: Library, // The code does not directly access this field, but the library needs to stay loaded for the entire RyzenAdj lifetime
    native: Native,
    ry: RyzenAccess,
}

impl RyzenAdj {
    pub fn new() -> Result<Self, Error> {
        debug!("Loading RyzenAdj library");
        // SAFETY: Bundled DLL version does not include any initialization/termination routines
        let library = unsafe { Library::new("./libryzenadj.dll")? };
        // SAFETY: The specified types match the library header
        let native = unsafe {
            Native {
                init_ryzenadj: get_native_symbol(&library, b"init_ryzenadj")?,
                cleanup_ryzenadj: get_native_symbol(&library, b"cleanup_ryzenadj")?,
                refresh_table: get_native_symbol(&library, b"refresh_table")?,
                get_fast_limit: get_native_symbol(&library, b"get_fast_limit")?,
                set_fast_limit: get_native_symbol(&library, b"set_fast_limit")?,
                set_slow_limit: get_native_symbol(&library, b"set_slow_limit")?,
                set_stapm_limit: get_native_symbol(&library, b"set_stapm_limit")?,
            }
        };
        debug!("Initializing RyzenAdj");
        // SAFETY: The library is still loaded in memory
        let ry = unsafe { (native.init_ryzenadj)() };
        if ry.is_invalid() {
            Err(Error::InitFailure)
        } else {
            Ok(RyzenAdj {
                _library: library,
                native,
                ry,
            })
        }
    }

    /// Provides access to the refreshed table of CPU information.
    pub fn get_table(&self) -> Result<RyzenAdjTable, Error> {
        debug!("Reading TDP table");
        // SAFETY: Validity of Library and `RyzenAccess` pointers is guaranteed
        // for the lifetime of `RyzenAdj` instance
        Error::check(unsafe { (self.native.refresh_table)(self.ry) })?;
        Ok(RyzenAdjTable { main: self })
    }

    /// Tries to change the TDP limit to the provided value in milliwatts.
    /// This action invalidates the table, thus it requires a unique reference to `RyzenAdj`.
    pub fn set_all_limits(&mut self, value: u32) -> Result<(), Error> {
        // SAFETY: Validity of Library and `RyzenAccess` pointers is guaranteed
        // for the lifetime of `RyzenAdj` instance
        unsafe {
            debug!("Setting STAPM limit");
            log::logger().flush();
            Error::check((self.native.set_stapm_limit)(self.ry, value))?;
            debug!("Setting slow TDP limit");
            log::logger().flush();
            Error::check((self.native.set_slow_limit)(self.ry, value))?;
            debug!("Setting fast TDP limit");
            log::logger().flush();
            Error::check((self.native.set_fast_limit)(self.ry, value))?;
            debug!("All limits set");
        }
        Ok(())
    }
}

impl Drop for RyzenAdj {
    fn drop(&mut self) {
        debug!("Cleaning up RyzenAdj");
        // SAFETY: Validity of Library and `RyzenAccess` pointers is guaranteed
        // for the lifetime of `RyzenAdj` instance.
        // The language guarantees that `Drop::drop` will not be called twice.
        unsafe { (self.native.cleanup_ryzenadj)(self.ry) }
    }
}
