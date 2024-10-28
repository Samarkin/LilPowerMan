use crate::winapi::{AcquiredDC, PaintContext};
use std::ffi::c_void;
use std::ptr::null_mut;
use windows::core::{Error, Owned, Result};
use windows::Win32::Foundation::{BOOL, ERROR_INVALID_PARAMETER, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, GetSysColorBrush, BITMAPINFO, BITMAPV5HEADER, BI_RGB,
    COLOR_WINDOW, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO, WM_APP};

pub const WM_NOTIFY_ICON: u32 = WM_APP + 1;

pub struct NotifyIcon {
    window: HWND,
    id: u32,
}

impl NotifyIcon {
    /// # Safety
    ///
    /// Window must be valid.
    pub unsafe fn new(window: HWND, id: u32) -> Result<NotifyIcon> {
        let icon = Self::new_icon()?;
        let mut notify_icon_data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: window,
            uID: id,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_NOTIFY_ICON,
            hIcon: *icon,
            ..Default::default()
        };
        let tip: Vec<u16> = "Hello, world".encode_utf16().collect();
        notify_icon_data.szTip[..tip.len()].copy_from_slice(&tip[..tip.len()]);
        // SAFETY: Notify icon data is a local structure
        if unsafe { Shell_NotifyIconW(NIM_ADD, &notify_icon_data) }.0 == 0 {
            Err(Error::from(ERROR_INVALID_PARAMETER))
        } else {
            Ok(NotifyIcon { window, id })
        }
    }

    fn paint_on_bitmap(bitmap: &Owned<HBITMAP>) {
        // SAFETY: Owned bitmap is guaranteed to not be destroyed yet.
        let pc = unsafe { PaintContext::for_bitmap(**bitmap) };
        // SAFETY: Using constant system color that is guaranteed to be valid
        let brush = unsafe { GetSysColorBrush(COLOR_WINDOW) };
        if brush.is_invalid() {
            panic!("GetSysColorBrush returned invalid brush");
        }
        pc.fill_rect(
            &RECT {
                left: 0,
                right: 32,
                top: 0,
                bottom: 32,
            },
            brush,
        );
        pc.draw_text("LIL", 0, 0);
        pc.draw_text("TDP", 0, 16);
    }

    fn new_icon() -> Result<Owned<HICON>> {
        let header = BITMAPV5HEADER {
            bV5Size: size_of::<BITMAPV5HEADER>() as u32,
            bV5Width: 32,
            bV5Height: 32,
            bV5Planes: 1,
            bV5BitCount: 32,
            bV5Compression: BI_RGB,
            ..Default::default()
        };
        let hdc = AcquiredDC::new();
        let mut bits: *mut c_void = null_mut();
        let bitmap = unsafe {
            Owned::new(CreateDIBSection(
                *hdc,
                &header as *const _ as *const BITMAPINFO,
                DIB_RGB_COLORS,
                &mut bits,
                None,
                0,
            )?)
        };
        Self::paint_on_bitmap(&bitmap);
        let mask = unsafe { Owned::new(CreateBitmap(32, 32, 1, 1, None)) };
        let icon_info = ICONINFO {
            fIcon: BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: *mask,
            hbmColor: *bitmap,
        };
        let icon = unsafe { Owned::new(CreateIconIndirect(&icon_info)?) };
        Ok(icon)
    }
}

impl Drop for NotifyIcon {
    fn drop(&mut self) {
        let notify_icon_data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.window,
            uID: self.id,
            ..Default::default()
        };
        // SAFETY: Notify icon data is a local structure
        let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &notify_icon_data) };
    }
}
