use crate::winapi::colors::{COLOR_BLACK, COLOR_WHITE};
use crate::winapi::{AcquiredDC, PaintContext};
use std::cmp::min;
use std::ffi::c_void;
use std::ptr::null_mut;
use windows::core::{Error, Owned, Result};
use windows::Win32::Foundation::{BOOL, COLORREF, ERROR_INVALID_PARAMETER, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, CreateFontIndirectW, CreateSolidBrush, ANSI_CHARSET,
    BITMAPINFO, BITMAPV5HEADER, BI_RGB, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DIB_RGB_COLORS,
    FF_SWISS, FW_BOLD, HBITMAP, HFONT, LOGFONTW, OUT_OUTLINE_PRECIS, VARIABLE_PITCH,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NIM_SETVERSION, NOTIFYICONDATAW, NOTIFYICONDATAW_0, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO, WM_APP};

pub const WM_NOTIFY_ICON: u32 = WM_APP + 1;

struct IconFactory {
    font: Owned<HFONT>,
}

impl IconFactory {
    pub fn new() -> IconFactory {
        IconFactory {
            font: Self::create_font(),
        }
    }

    fn create_font() -> Owned<HFONT> {
        let mut log_font = LOGFONTW {
            lfHeight: 30,
            lfWeight: FW_BOLD.0 as i32,
            lfCharSet: ANSI_CHARSET,
            lfOutPrecision: OUT_OUTLINE_PRECIS,
            lfClipPrecision: CLIP_DEFAULT_PRECIS,
            lfQuality: CLEARTYPE_QUALITY,
            lfPitchAndFamily: VARIABLE_PITCH.0 | FF_SWISS.0,
            ..Default::default()
        };
        let face_name: Vec<u16> = "Segoe UI".encode_utf16().collect();
        log_font.lfFaceName[..8].copy_from_slice(&face_name[..8]);
        let font = unsafe { CreateFontIndirectW(&log_font) };
        assert!(!font.is_invalid(), "Font creation failed");
        // SAFETY: The font is owned by us
        unsafe { Owned::new(font) }
    }

    fn paint_on_bitmap(&self, bitmap: &Owned<HBITMAP>, text: &str, fg: COLORREF, bg: COLORREF) {
        // SAFETY: Owned bitmap is guaranteed to not be destroyed yet.
        let mut pc = unsafe { PaintContext::for_bitmap(**bitmap) };
        // SAFETY: The call is sound regardless of the arguments
        let bg_brush = unsafe { Owned::new(CreateSolidBrush(bg)) };
        assert!(!bg_brush.is_invalid(), "Failed to create brush");
        pc.fill_rect(
            &RECT {
                left: 0,
                right: 32,
                top: 0,
                bottom: 32,
            },
            *bg_brush,
        );
        pc.set_font(&self.font);
        pc.set_text_color(fg);
        pc.set_bg_color(bg);
        pc.draw_text(text, 0, 0);
    }

    pub fn render_icon(&self, text: &str, fg: COLORREF, bg: COLORREF) -> Owned<HICON> {
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
        // SAFETY: the provided arguments are all valid local variables
        // The call is not expected to fail as long as the arguments are valid
        let bitmap = unsafe {
            Owned::new(
                CreateDIBSection(
                    *hdc,
                    &header as *const _ as *const BITMAPINFO,
                    DIB_RGB_COLORS,
                    &mut bits,
                    None,
                    0,
                )
                .unwrap(),
            )
        };
        self.paint_on_bitmap(&bitmap, text, fg, bg);
        // SAFETY: Passing None instead of a pointer
        let mask = unsafe { Owned::new(CreateBitmap(32, 32, 1, 1, None)) };
        assert!(!mask.is_invalid(), "Failed to create the mask bitmap");
        let icon_info = ICONINFO {
            fIcon: BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: *mask,
            hbmColor: *bitmap,
        };
        // SAFETY: mask and color bitmaps are valid and not currently selected into any DC
        unsafe { Owned::new(CreateIconIndirect(&icon_info).unwrap()) }
    }
}

pub struct NotifyIcon {
    window: HWND,
    id: u32,
    icon_factory: IconFactory,
}

impl NotifyIcon {
    /// # Safety
    ///
    /// Caller must guarantee that the provided window will stay valid
    /// for the entire lifetime of the returned instance.
    pub unsafe fn new(window: HWND, id: u32) -> Result<NotifyIcon> {
        let icon_factory = IconFactory::new();
        let icon = icon_factory.render_icon("⏳", COLOR_BLACK, COLOR_WHITE);
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

    pub fn update(&mut self, tip: &str, icon: &str, fg: COLORREF, bg: COLORREF) {
        let icon = self.icon_factory.render_icon(icon, fg, bg);
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
