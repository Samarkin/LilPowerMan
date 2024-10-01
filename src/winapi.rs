use windows::core::Error;
use windows::Win32::Foundation::{BOOL, HINSTANCE, LRESULT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, LoadCursorW, TranslateMessage, HCURSOR, IDC_ARROW, MSG,
};

#[inline]
pub fn get_instance_handle() -> windows::core::Result<HINSTANCE> {
    // SAFETY: lpModuleName is None instead of a raw pointer
    let module_handle = unsafe { GetModuleHandleW(None) }?;
    Ok(module_handle.into())
}

#[inline]
pub fn get_default_cursor() -> windows::core::Result<HCURSOR> {
    // SAFETY: lpCursorName is a pre-defined constant instead of a raw pointer
    unsafe { LoadCursorW(None, IDC_ARROW) }
}

#[inline]
fn unwrap_winapi_bool(bool: BOOL) -> windows::core::Result<bool> {
    match bool.0 {
        1.. => Ok(true),
        0 => Ok(false),
        _ => Err(Error::from_win32()),
    }
}

#[inline]
pub fn get_message(msg: &mut MSG) -> windows::core::Result<bool> {
    // SAFETY: msg is a valid pointer
    let result = unsafe { GetMessageW(msg, None, 0, 0) };
    unwrap_winapi_bool(result)
}

#[inline]
pub fn translate_message(msg: &MSG) -> bool {
    // SAFETY: msg is a valid pointer
    unsafe { TranslateMessage(msg) }.as_bool()
}

#[inline]
pub fn dispatch_message(msg: &MSG) -> LRESULT {
    // SAFETY: msg is a valid pointer
    unsafe { DispatchMessageW(msg) }
}
