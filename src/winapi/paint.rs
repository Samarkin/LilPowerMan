use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, FillRect, GetSysColorBrush, TextOutW, COLOR_WINDOW, HBRUSH, HDC,
    PAINTSTRUCT,
};

pub struct PaintContext {
    window: HWND,
    ps: PAINTSTRUCT,
    hdc: HDC,
}

impl PaintContext {
    /// Creates new paint context.
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
        let pc = PaintContext { window, ps, hdc };
        if pc.ps.fErase.as_bool() {
            // SAFETY: Using constant system color that is guaranteed to be valid
            let brush = unsafe { GetSysColorBrush(COLOR_WINDOW) };
            if brush.is_invalid() {
                panic!("GetSysColorBrush returned invalid brush");
            }
            pc.fill_rect(&pc.ps.rcPaint, brush);
        }
        pc
    }

    pub fn fill_rect(&self, rect: &RECT, brush: HBRUSH) {
        // SAFETY: `hdc` is guaranteed to be valid for `PaintContext`
        if unsafe { FillRect(self.hdc, rect, brush) } == 0 {
            panic!("Failed to call FillRect");
        }
    }

    pub fn draw_text(&self, text: &str, x: i32, y: i32) {
        let chars: Vec<u16> = text.encode_utf16().collect();
        // SAFETY: `hdc` is guaranteed to be valid for `PaintContext`, `chars` points to a valid local array
        if unsafe { TextOutW(self.hdc, x, y, chars.as_slice()).0 } == 0 {
            panic!("Failed to call TextOut");
        }
    }
}

impl Drop for PaintContext {
    fn drop(&mut self) {
        // SAFETY: BeginPaint always precedes creation of an instance of PaintContext
        // Return value is always non-zero according to the documentation.
        let _ = unsafe { EndPaint(self.window, &self.ps) };
    }
}
