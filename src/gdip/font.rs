use super::font_family::FontFamily;
use super::{Error, GdiPlus, Result};
use std::marker::PhantomData;
use std::ptr::null_mut;
use windows::Win32::Graphics::GdiPlus::{FontStyle, GdipCreateFont, GdipDeleteFont, GpFont, Unit};

pub struct Font<'init> {
    native: *mut GpFont,
    _context: PhantomData<&'init GdiPlus>,
}

impl<'init> Font<'init> {
    pub fn new(
        _context: &'init GdiPlus,
        name: &str,
        emsize: f32,
        unit: Unit,
        style: FontStyle,
    ) -> Result<Self> {
        let font_family = FontFamily::new(_context, name)?;
        let mut font = null_mut();
        // SAFETY: The provided pointers are valid for the duration of the GDI+ call
        Error::check(unsafe {
            GdipCreateFont(font_family.get_native(), emsize, style.0, unit, &mut font)
        })?;
        Ok(Font {
            native: font,
            _context: PhantomData,
        })
    }

    /// Get native GDI+ Font pointer.
    ///
    /// # Safety
    /// The caller must ensure that the returned pointer is not used after the instance is dropped.
    pub(crate) unsafe fn get_native(&self) -> *mut GpFont {
        self.native
    }
}

impl<'init> Drop for Font<'init> {
    fn drop(&mut self) {
        // SAFETY: The native pointer is guaranteed to be valid
        let result = unsafe { GdipDeleteFont(self.native) };
        if let Err(err) = Error::check(result) {
            error!("Failed to delete GDI+ font: {}", err);
        }
    }
}
