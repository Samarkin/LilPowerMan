use windows::Win32::Foundation::COLORREF;

pub const fn color_from_rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(((b as u32) << 16) | ((g as u32) << 8) | r as u32)
}

pub const COLOR_BLACK: COLORREF = color_from_rgb(0x00, 0x00, 0x00);
pub const COLOR_WHITE: COLORREF = color_from_rgb(0xFF, 0xFF, 0xFF);
pub const COLOR_CYAN: COLORREF = color_from_rgb(0x00, 0xFF, 0xFF);
pub const COLOR_RED: COLORREF = color_from_rgb(0xFF, 0x00, 0x00);
pub const COLOR_GREEN: COLORREF = color_from_rgb(0x00, 0xFF, 0x00);
