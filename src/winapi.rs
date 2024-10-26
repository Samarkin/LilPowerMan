mod paint;

use std::ffi::c_void;
use windows::core::{Error, Owned, Result};
use windows::Win32::Foundation::{BOOL, HANDLE, HINSTANCE};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, LoadCursorW, TranslateMessage, HCURSOR, IDC_ARROW, MSG,
};

pub use paint::PaintContext;

pub fn get_instance_handle() -> Result<HINSTANCE> {
    // SAFETY: lpModuleName is None instead of a raw pointer
    let module_handle = unsafe { GetModuleHandleW(None) }?;
    Ok(module_handle.into())
}

pub fn get_default_cursor() -> Result<HCURSOR> {
    // SAFETY: lpCursorName is a pre-defined constant instead of a raw pointer
    unsafe { LoadCursorW(None, IDC_ARROW) }
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
fn get_message(msg: &mut MSG) -> Result<bool> {
    let result = unsafe { GetMessageW(msg, None, 0, 0) };
    unwrap_winapi_bool(result)
}

#[inline]
pub fn windows_message_loop() -> Result<()> {
    let mut msg: MSG = Default::default();
    while get_message(&mut msg)? {
        // SAFETY: msg has been initialized to the latest message
        unsafe {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        };
    }
    Ok(())
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
            Some(param as *const _ as *const c_void),
            size_of::<Input>() as u32,
            Some(&mut buffer as *mut _ as *mut c_void),
            size_of::<Output>() as u32,
            None,
            None,
        )?
    };
    Ok(buffer)
}
