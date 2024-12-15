use super::{Error, GdiPlus, Result};
use std::marker::PhantomData;
use std::ptr::null_mut;
use windows::core::PCWSTR;
use windows::Win32::Graphics::GdiPlus::{
    GdipCreateFontFamilyFromName, GdipDeleteFontFamily, GpFontFamily,
};

pub struct FontFamily<'init> {
    native: *mut GpFontFamily,
    _context: PhantomData<&'init GdiPlus>,
}

impl<'init> FontFamily<'init> {
    pub fn new(_context: &'init GdiPlus, name: &str) -> Result<Self> {
        let mut font_family = null_mut();
        let mut name: Vec<u16> = name.encode_utf16().collect();
        name.push(0);
        // SAFETY: The provided pointers are valid for the duration of the GDI+ call
        Error::check(unsafe {
            GdipCreateFontFamilyFromName(
                PCWSTR::from_raw(name.as_ptr()),
                null_mut(),
                &mut font_family,
            )
        })?;
        Ok(FontFamily {
            native: font_family,
            _context: PhantomData,
        })
    }

    /// Get native GDI+ FontFamily pointer.
    ///
    /// # Safety
    /// The caller must ensure that the returned pointer is not used after the instance is dropped.
    pub(crate) unsafe fn get_native(&self) -> *mut GpFontFamily {
        self.native
    }
}

impl<'init> Drop for FontFamily<'init> {
    fn drop(&mut self) {
        // SAFETY: The native pointer is guaranteed to be valid
        let result = unsafe { GdipDeleteFontFamily(self.native) };
        if let Err(err) = Error::check(result) {
            error!("Failed to delete GDI+ font family: {}", err);
        }
    }
}
