use std::ops::Deref;
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC, HDC};

pub struct AcquiredDC {
    hdc: HDC,
}

impl AcquiredDC {
    pub fn new() -> Self {
        // SAFETY: Getting DC of the entire screen is always sound
        let hdc = unsafe { GetDC(None) };
        if hdc.is_invalid() {
            panic!("Failed to get device context of the current screen");
        }
        AcquiredDC { hdc }
    }
}

impl Deref for AcquiredDC {
    type Target = HDC;

    fn deref(&self) -> &Self::Target {
        &self.hdc
    }
}

impl Drop for AcquiredDC {
    fn drop(&mut self) {
        // SAFETY: Runtime guarantees that `drop` will only be called once
        //   And the struct guarantees that DC is not of a single window.
        let result = unsafe { ReleaseDC(None, **self) };
        if result == 0 {
            error!("Failed to release DC");
        }
    }
}
