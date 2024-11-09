mod bitmap;
mod colors;
mod error;
mod font;
mod font_family;
mod graphics;

pub use bitmap::Bitmap;
pub use colors::Color;
pub use error::{Error, Result};
pub use font::Font;
pub use graphics::Graphics;
use std::ptr::null_mut;
use windows::Win32::Graphics::GdiPlus::{GdiplusShutdown, GdiplusStartup, GdiplusStartupInput};

/// Entry point of all interaction with GDI+.
///
/// The struct takes care of initializing and cleaning up GDI+ resources, and utilizes
/// borrow checker to ensure all GDI+ interaction happens between those two events.
pub struct GdiPlus {
    token: usize,
}

impl GdiPlus {
    /// Initializes GDI+ resources.
    pub fn new() -> Self {
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            ..Default::default()
        };
        let mut token = 0;
        // SAFETY: Last argument is optional, other arguments are valid.
        Error::check(unsafe { GdiplusStartup(&mut token, &input, null_mut()) }).unwrap();
        GdiPlus { token }
    }
}

impl Drop for GdiPlus {
    fn drop(&mut self) {
        // SAFETY: The token is guaranteed to be valid by the constructor.
        // Borrow checker ensures that all GDI+ objects are destroyed by now.
        unsafe { GdiplusShutdown(self.token) };
    }
}
