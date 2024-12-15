use super::{Error, GdiPlus, Result};
use std::marker::PhantomData;
use std::ptr::null_mut;
use windows::core::Owned;
use windows::Win32::Graphics::GdiPlus::{
    GdipCreateBitmapFromScan0, GdipCreateHICONFromBitmap, GdipDisposeImage, GpBitmap,
    PixelFormatAlpha, PixelFormatCanonical, PixelFormatGDI,
};
use windows::Win32::UI::WindowsAndMessaging::HICON;

pub struct Bitmap<'init> {
    native: *mut GpBitmap,
    _context: PhantomData<&'init GdiPlus>,
}

const PIXEL_FORMAT32BPP_ARGB: u32 =
    10 | (32 << 8) | PixelFormatAlpha | PixelFormatGDI | PixelFormatCanonical;

impl<'init> Bitmap<'init> {
    pub fn new(_context: &'init GdiPlus, width: i32, height: i32) -> Result<Self> {
        // SAFETY: GDI+ call will initialize the bitmap
        let mut bitmap = null_mut();
        Error::check(unsafe {
            GdipCreateBitmapFromScan0(
                width,
                height,
                0,
                PIXEL_FORMAT32BPP_ARGB as i32,
                None,
                &mut bitmap,
            )
        })?;
        Ok(Bitmap {
            native: bitmap,
            _context: PhantomData,
        })
    }

    /// Get native GDI+ Bitmap pointer.
    ///
    /// # Safety
    /// The caller must ensure the returned pointer is not used after the bitmap is dropped.
    pub(crate) unsafe fn get_native(&self) -> *mut GpBitmap {
        self.native
    }

    pub fn as_hicon(&self) -> Owned<HICON> {
        let mut icon = Default::default();
        // SAFETY: The provided pointers are valid for the duration of the GDI+ call
        Error::check(unsafe { GdipCreateHICONFromBitmap(self.native, &mut icon) }).unwrap();
        // SAFETY: The GDI+ call initialized the icon, we own it, and it is safe to destroy it
        unsafe { Owned::new(icon) }
    }
}

impl<'init> Drop for Bitmap<'init> {
    fn drop(&mut self) {
        // SAFETY: The native pointer is guaranteed to be valid
        let result = unsafe { GdipDisposeImage(self.native as *mut _) };
        if let Err(err) = Error::check(result) {
            error!("Failed to dispose of a GDI+ bitmap: {}", err);
        }
    }
}
