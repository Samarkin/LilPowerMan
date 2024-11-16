use windows::core::{w, Error, PCWSTR};
use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows::Win32::System::Threading::CreateMutexW;

pub struct Singleton;

const MUTEX_NAME: PCWSTR = w!("Global\\LilPowerManSingletonMutex");

impl Singleton {
    pub fn is_first_instance() -> bool {
        // SAFETY: The call is always sound, and we don't expect it to fail
        // We intentionally leak the handle here to ensure the mutex lives until the app terminates
        _ = unsafe { CreateMutexW(None, false, MUTEX_NAME).unwrap() };
        Error::from_win32() != Error::from(ERROR_ALREADY_EXISTS)
    }
}
