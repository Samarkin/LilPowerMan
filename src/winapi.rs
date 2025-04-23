pub mod colors;
mod dc;
mod files;
mod paint;

use windows::core::{w, Error, Owned, Result, PCWSTR};
use windows::Win32::Foundation::{BOOL, HANDLE, HINSTANCE, SYSTEMTIME};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, LoadCursorW, MessageBoxW, TranslateMessage, HCURSOR, IDC_ARROW,
    MB_OK, MSG,
};

pub use dc::AcquiredDC;
pub use files::Files;
pub use paint::PaintContext;

const APP_NAME: PCWSTR = w!("LilPowerMan");

pub fn show_error_message_box(text: &str) {
    let mut text: Vec<u16> = text.encode_utf16().collect();
    text.push(0);
    unsafe { MessageBoxW(None, PCWSTR::from_raw(text.as_ptr()), APP_NAME, MB_OK) };
}

pub fn get_instance_handle() -> HINSTANCE {
    // SAFETY: lpModuleName is None instead of a raw pointer
    // The call is sound and should always return the handle of the main module (.exe file)
    unsafe { GetModuleHandleW(None) }.unwrap().into()
}

pub fn get_local_time() -> SYSTEMTIME {
    // SAFETY: The call is always sound
    unsafe { GetLocalTime() }
}

pub fn get_default_cursor() -> HCURSOR {
    // SAFETY: lpCursorName is a pre-defined constant instead of a raw pointer
    // The call is sound and should always return the handle of a pre-defined system cursor
    unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap()
}

#[inline]
fn unwrap_winapi_bool(bool: BOOL) -> Result<bool> {
    match bool.0 {
        1.. => Ok(true),
        0 => Ok(false),
        _ => Err(Error::from_win32()),
    }
}

#[inline]
fn get_message(msg: &mut MSG) -> bool {
    // SAFETY: hWnd is NULL and msg is guaranteed to be a valid pointer to writeable memory
    let result = unsafe { GetMessageW(msg, None, 0, 0) };
    // The call is not expected to fail given valid input
    unwrap_winapi_bool(result).unwrap()
}

#[inline]
pub fn windows_message_loop() {
    let mut msg: MSG = Default::default();
    while get_message(&mut msg) {
        // SAFETY: msg has been initialized to the latest message
        unsafe {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        };
    }
}

pub fn device_io_control<Input, Output: Default>(
    device: &Owned<HANDLE>,
    control_code: u32,
    param: &Input,
) -> Result<Output> {
    let mut buffer: Output = Default::default();
    // SAFETY: Owned handle outlives the copy
    unsafe {
        DeviceIoControl(
            **device,
            control_code,
            Some(param as *const _ as *const _),
            size_of::<Input>() as u32,
            Some(&mut buffer as *mut _ as *mut _),
            size_of::<Output>() as u32,
            None,
            None,
        )?
    };
    Ok(buffer)
}
