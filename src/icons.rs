use crate::gdip::{Bitmap, Color, Font, GdiPlus, Graphics};
use std::cmp::min;
use windows::core::{Error, Owned, Result};
use windows::Win32::Foundation::{ERROR_INVALID_PARAMETER, HWND};
use windows::Win32::Graphics::GdiPlus::{FontStyleBold, UnitPoint};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NIM_SETVERSION, NOTIFYICONDATAW, NOTIFYICONDATAW_0, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{HICON, WM_APP};

pub const WM_NOTIFY_ICON: u32 = WM_APP + 1;

struct IconFactory<'gdip> {
    gdi_plus: &'gdip GdiPlus,
    font: Font<'gdip>,
    // TODO: Add brush cache
}

impl<'gdip> IconFactory<'gdip> {
    pub fn new(gdi_plus: &'gdip GdiPlus) -> IconFactory {
        // We expect the font to be found
        let font = Font::new(gdi_plus, "Segoe UI", 9.0, UnitPoint, FontStyleBold).unwrap();
        IconFactory { gdi_plus, font }
    }

    pub fn render_icon(&self, text: &str, color: Color) -> Owned<HICON> {
        // We don't expect errors since the provided size is valid
        let mut bitmap = Bitmap::new(self.gdi_plus, 32, 32).unwrap();
        Graphics::for_bitmap(&mut bitmap)
            .draw_string(text, &self.font, color, 0.0, 0.0)
            .unwrap();
        bitmap.as_hicon()
    }
}

pub struct NotifyIcon<'gdip> {
    window: HWND,
    id: u32,
    icon_factory: IconFactory<'gdip>,
}

impl<'gdip> NotifyIcon<'gdip> {
    /// # Safety
    ///
    /// Caller must guarantee that the provided window will stay valid
    /// for the entire lifetime of the returned instance.
    pub unsafe fn new(window: HWND, id: u32, gdi_plus: &'gdip GdiPlus) -> Result<NotifyIcon> {
        let icon_factory = IconFactory::new(gdi_plus);
        let icon = icon_factory.render_icon("⏳", Color::WHITE);
        let mut notify_icon_data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: window,
            uID: id,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_NOTIFY_ICON,
            Anonymous: NOTIFYICONDATAW_0 {
                uVersion: NOTIFYICON_VERSION_4,
            },
            hIcon: *icon,
            ..Default::default()
        };
        let tip: Vec<u16> = "Hello, world".encode_utf16().collect();
        notify_icon_data.szTip[..tip.len()].copy_from_slice(&tip[..tip.len()]);
        // SAFETY: Notify icon data is a local structure
        if unsafe { Shell_NotifyIconW(NIM_ADD, &notify_icon_data) }.0 == 0
            || unsafe { Shell_NotifyIconW(NIM_SETVERSION, &notify_icon_data) }.0 == 0
        {
            Err(Error::from(ERROR_INVALID_PARAMETER))
        } else {
            Ok(NotifyIcon {
                window,
                id,
                icon_factory,
            })
        }
    }

    pub fn update(&mut self, tip: &str, icon: &str, color: Color) {
        let icon = self.icon_factory.render_icon(icon, color);
        let mut notify_icon_data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.window,
            uID: self.id,
            uFlags: NIF_TIP | NIF_ICON,
            hIcon: *icon,
            ..Default::default()
        };
        let tip: Vec<u16> = tip.encode_utf16().collect();
        // ensure at least one character remains NULL
        let len = min(notify_icon_data.szTip.len() - 1, tip.len());
        notify_icon_data.szTip[..len].copy_from_slice(&tip[..len]);
        // SAFETY: Notify icon data is a local structure
        if unsafe { Shell_NotifyIconW(NIM_MODIFY, &notify_icon_data) }.0 == 0 {
            // We don't expect this operation to fail as long as hWnd and uID are valid
            panic!("Shell_NotifyIconW(NIM_MODIFY) failed");
        }
    }
}

impl Drop for NotifyIcon<'_> {
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
