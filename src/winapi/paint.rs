use windows::core::Owned;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, DeleteDC, EndPaint, FillRect, GetSysColorBrush, SelectObject,
    TextOutW, COLOR_WINDOW, HBITMAP, HBRUSH, HDC, HFONT, PAINTSTRUCT,
};

enum DeviceContextSource {
    Window(HWND, PAINTSTRUCT),
    Owned,
}

pub struct PaintContext {
    hdc: HDC,
    hdc_source: DeviceContextSource,
}

impl PaintContext {
    /// Creates new paint context for a window.
    ///
    /// # Safety
    ///
    /// Should only be called in response to `WM_PAINT` message.
    pub unsafe fn for_window(window: HWND) -> PaintContext {
        let mut ps = PAINTSTRUCT::default();
        // SAFETY: ps is a valid local structure
        let hdc = unsafe { BeginPaint(window, &mut ps) };
        if hdc.is_invalid() {
            panic!("BeginPaint returned invalid HDC");
        }
        let mut pc = PaintContext {
            hdc_source: DeviceContextSource::Window(window, ps),
            hdc,
        };
        if ps.fErase.as_bool() {
            // SAFETY: Using constant system color that is guaranteed to be valid
            let brush = unsafe { GetSysColorBrush(COLOR_WINDOW) };
            if brush.is_invalid() {
                panic!("GetSysColorBrush returned invalid brush");
            }
            pc.fill_rect(&ps.rcPaint, brush);
        }
        pc
    }

    /// Creates new paint context for a bitmap.
    ///
    /// # Safety
    ///
    /// Bitmap must be valid.
    pub unsafe fn for_bitmap(bitmap: HBITMAP) -> PaintContext {
        // SAFETY: Creating an in-memory DC compatible with the current screen is always sound
        let hdc = unsafe { CreateCompatibleDC(None) };
        if hdc.is_invalid() {
            panic!("CreateCompatibleDC returned an invalid HDC");
        }
        // SAFETY: We own the DC, and validity of the bitmap is guaranteed by the caller
        // Return value is the previous bitmap. It can safely be ignored since we just created the DC
        let _ = unsafe { SelectObject(hdc, bitmap) };
        PaintContext {
            hdc_source: DeviceContextSource::Owned,
            hdc,
        }
    }

    pub fn fill_rect(&mut self, rect: &RECT, brush: HBRUSH) {
        // SAFETY: `hdc` is guaranteed to be valid for `PaintContext`
        if unsafe { FillRect(self.hdc, rect, brush) } == 0 {
            panic!("Failed to call FillRect");
        }
    }

    pub fn set_font<'this, 'font>(&'this mut self, font: &'font Owned<HFONT>)
    where
        'font: 'this,
    {
        // SAFETY: We verified that the caller owns the font that will stay valid long enough
        unsafe { SelectObject(self.hdc, **font) };
    }

    pub fn draw_text(&mut self, text: &str, x: i32, y: i32) {
        let chars: Vec<u16> = text.encode_utf16().collect();
        // SAFETY: `hdc` is guaranteed to be valid for `PaintContext`, `chars` points to a valid local array
        if unsafe { TextOutW(self.hdc, x, y, chars.as_slice()).0 } == 0 {
            panic!("Failed to call TextOut");
        }
    }
}

impl Drop for PaintContext {
    fn drop(&mut self) {
        match self.hdc_source {
            DeviceContextSource::Window(window, ps) => {
                // SAFETY: BeginPaint preceded creation of this instance of PaintContext
                // Return value is always non-zero according to the documentation.
                let _ = unsafe { EndPaint(window, &ps) };
            }
            DeviceContextSource::Owned => {
                // SAFETY: This branch means we own the DC
                let _ = unsafe { DeleteDC(self.hdc) };
            }
        }
    }
}
