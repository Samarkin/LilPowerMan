use windows::Win32::Graphics::GdiPlus::Color as GdipColor;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Color(u32);

impl Color {
    pub const WHITE: Color = Color(GdipColor::White as _);
    pub const CYAN: Color = Color(GdipColor::Cyan as _);
    pub const RED: Color = Color(GdipColor::Red as _);
    pub const GREEN: Color = Color(GdipColor::Green as _);
    pub const YELLOW: Color = Color(GdipColor::Yellow as _);
}

impl Into<u32> for Color {
    fn into(self) -> u32 {
        self.0
    }
}
