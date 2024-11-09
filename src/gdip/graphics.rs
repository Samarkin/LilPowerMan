use super::bitmap::Bitmap;
use super::colors::Color;
use super::font::Font;
use super::{Error, Result};
use std::marker::PhantomData;
use std::ptr::null_mut;
use windows::core::PCWSTR;
use windows::Win32::Graphics::GdiPlus::{
    GdipCreateSolidFill, GdipDeleteBrush, GdipDeleteGraphics, GdipDrawString,
    GdipGetImageGraphicsContext, GpGraphics, RectF,
};

pub struct Graphics<'init, 'bitmap> {
    native: *mut GpGraphics,
    _marker: PhantomData<&'bitmap mut Bitmap<'init>>,
}

impl<'init, 'bitmap> Graphics<'init, 'bitmap> {
    pub fn for_bitmap(bitmap: &'bitmap mut Bitmap<'init>) -> Self {
        let mut graphics = null_mut();
        // SAFETY: The provided pointers are valid for the lifetime of `Graphics`.
        Error::check(unsafe {
            GdipGetImageGraphicsContext(bitmap.get_native() as *mut _, &mut graphics)
        })
        .unwrap();
        Graphics {
            native: graphics,
            _marker: PhantomData,
        }
    }

    pub fn draw_string(
        &mut self,
        text: &str,
        font: &Font,
        color: Color,
        x: f32,
        y: f32,
    ) -> Result<()> {
        unsafe {
            let mut fill = null_mut();
            Error::check(GdipCreateSolidFill(color.into(), &mut fill))?;
            let brush = fill as *mut _ as *mut _;

            let str: Vec<u16> = text.encode_utf16().collect();
            let layout = RectF {
                X: x,
                Y: y,
                Width: 0.0,
                Height: 0.0,
            };
            Error::check(GdipDrawString(
                self.native,
                PCWSTR::from_raw(str.as_ptr()),
                str.len() as i32,
                font.get_native(),
                &layout,
                null_mut(),
                brush,
            ))?;

            Error::check(GdipDeleteBrush(brush))?;
            Ok(())
        }
    }
}

impl Drop for Graphics<'_, '_> {
    fn drop(&mut self) {
        // SAFETY: The native pointer is guaranteed to be valid
        let _ = unsafe { GdipDeleteGraphics(self.native) };
    }
}
