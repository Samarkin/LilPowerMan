use crate::battery::BatteriesIterator;
use crate::ryzenadj::RyzenAdj;
use crate::winapi::{get_default_cursor, get_instance_handle, PaintContext};
use std::marker::PhantomData;
use windows::core::{w, Error};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, PostQuitMessage, RegisterClassExW, CS_HREDRAW,
    CS_VREDRAW, CW_USEDEFAULT, WINDOW_EX_STYLE, WM_DESTROY, WM_PAINT, WNDCLASSEXW,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

pub struct MainWindow {
    handle: HWND,
    // This marks MainWindow as !Send and !Sync
    _marker: PhantomData<*const ()>,
}

impl MainWindow {
    pub fn new() -> Result<MainWindow, Error> {
        let window_class_name = w!("MainWindow");
        let instance = get_instance_handle()?;
        let wnd_class_params = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(Self::process_message),
            hInstance: instance,
            hCursor: get_default_cursor()?,
            lpszClassName: window_class_name,
            ..Default::default()
        };
        let window_class_atom = unsafe { RegisterClassExW(&wnd_class_params) };
        if window_class_atom == 0 {
            return Err(Error::from_win32());
        }
        let handle = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                window_class_name,
                w!("Hello, world"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                400,
                300,
                None,
                None,
                instance,
                None,
            )
        }?;
        Ok(MainWindow {
            handle,
            _marker: PhantomData,
        })
    }

    fn get_text() -> Result<String, Box<dyn std::error::Error>> {
        let mut text = String::new();
        text += &format!(
            "Current TDP: {} W",
            RyzenAdj::new()?.get_table()?.get_fast_limit()
        );
        for b in BatteriesIterator::new()? {
            text += &format!(", Battery charge rate {} mW", b?.get_charge_rate()?);
        }
        Ok(text)
    }

    extern "system" fn process_message(
        window: HWND,
        message: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        match message {
            WM_PAINT => {
                // SAFETY: We are responding to the WM_PAINT message
                let pc = unsafe { PaintContext::for_window(window) };
                let text = Self::get_text().unwrap_or_else(|e| format!("Error: {}", e));
                pc.draw_text(&text, 0, 0);
                LRESULT(0)
            }
            WM_DESTROY => {
                // SAFETY: This is a typical response to WM_DESTROY message
                unsafe { PostQuitMessage(0) }
                LRESULT(0)
            }
            _ =>
            // SAFETY: We are in the context of message processor, validity of arguments is guaranteed by the caller (OS)
            unsafe { DefWindowProcW(window, message, w_param, l_param) },
        }
    }
}

impl Drop for MainWindow {
    fn drop(&mut self) {
        // SAFETY: MainWindow always contains a valid `handle`
        unsafe {
            let _ = DestroyWindow(self.handle);
        }
    }
}
