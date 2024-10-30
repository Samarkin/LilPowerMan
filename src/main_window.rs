use crate::battery::BatteriesIterator;
use crate::icons::{NotifyIcon, WM_NOTIFY_ICON};
use crate::ryzenadj::RyzenAdj;
use crate::winapi::{get_default_cursor, get_instance_handle, PaintContext};
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::pin::Pin;
use windows::core::{w, Error};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, PostQuitMessage,
    RegisterClassExW, SetWindowLongPtrW, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
    GWLP_USERDATA, WINDOW_EX_STYLE, WM_CREATE, WM_DESTROY, WM_NCCREATE, WM_PAINT, WNDCLASSEXW,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

pub struct MainWindow {
    handle: HWND,
    icons: Vec<NotifyIcon>,
    // This marks MainWindow as !Send and !Sync
    _marker: PhantomData<*const ()>,
}

impl MainWindow {
    pub fn new() -> Pin<Box<MainWindow>> {
        let window_class_name = w!("MainWindow");
        let instance = get_instance_handle();
        let wnd_class_params = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(Self::wnd_proc),
            hInstance: instance,
            hCursor: get_default_cursor(),
            lpszClassName: window_class_name,
            ..Default::default()
        };
        // SAFETY: Argument points to a valid structure that outlives the call
        let window_class_atom = unsafe { RegisterClassExW(&wnd_class_params) };
        if window_class_atom == 0 {
            // The returned atom is expected to be non-null unless there's an issue with input
            panic!("{}", Error::from_win32());
        }
        let mut window = Box::pin(MainWindow {
            handle: HWND::default(),
            icons: vec![],
            _marker: PhantomData,
        });
        // SAFETY: The function is sound as long as all arguments are valid
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
                Some(window.deref_mut() as *mut _ as _),
            )
        }
        .unwrap();
        assert_eq!(
            handle, window.handle,
            "Window creation did not set the handle"
        );
        window
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

    fn process_message(&mut self, message: u32, w_param: WPARAM, l_param: LPARAM) -> Option<isize> {
        match message {
            WM_CREATE => {
                // SAFETY: Window handle is valid, number of icons is not expected to reach u32::MAX
                let icon = unsafe { NotifyIcon::new(self.handle, self.icons.len() as u32).unwrap() };
                self.icons.push(icon);
            }
            WM_NOTIFY_ICON => {}
            WM_PAINT => {
                // SAFETY: We are responding to the WM_PAINT message
                let pc = unsafe { PaintContext::for_window(self.handle) };
                let text = Self::get_text().unwrap_or_else(|e| format!("Error: {}", e));
                pc.draw_text(&text, 0, 0);
            }
            WM_DESTROY => {
                // SAFETY: This is a typical response to WM_DESTROY message
                unsafe { PostQuitMessage(0) }
            }
            _ => {}
        }
        None
    }

    extern "system" fn wnd_proc(
        window_handle: HWND,
        message: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if message == WM_NCCREATE {
            let cs = l_param.0 as *const CREATESTRUCTW;
            assert!(!cs.is_null(), "CREATESTRUCT pointer is missing");
            // SAFETY: We trust the OS to provide us with a valid pointer
            // This code runs before `MainWindow::new returns`, so
            //   no other code can access the MainWindow instance at this point
            let window = unsafe { ((*cs).lpCreateParams as *mut Self).as_mut() }
                .expect("MainWindow pointer is missing from the provided CREATESTRUCT");
            window.handle = window_handle;
            // SAFETY: The function is sound as long as the input is valid
            unsafe { SetWindowLongPtrW(window_handle, GWLP_USERDATA, window as *mut _ as _) };
        } else {
            // SAFETY: The function is sound as long as the input is valid
            let user_data = unsafe { GetWindowLongPtrW(window_handle, GWLP_USERDATA) };
            // SAFETY: During the lifetime of this reference,
            //   MainWindow is not accessed through any other reference
            if let Some(window) = unsafe { (user_data as *mut Self).as_mut() } {
                assert_eq!(window.handle, window_handle, "Invalid MainWindow pointer");
                if let Some(l_result) = window.process_message(message, w_param, l_param) {
                    return LRESULT(l_result);
                }
            }
        }
        // SAFETY: We are in the context of message processor,
        //   validity of the arguments is guaranteed by the caller (OS)
        unsafe { DefWindowProcW(window_handle, message, w_param, l_param) }
    }
}

impl Drop for MainWindow {
    fn drop(&mut self) {
        // The icons should get dropped before the window
        self.icons.clear();
        // SAFETY: MainWindow always contains a valid `handle`
        let _ = unsafe { DestroyWindow(self.handle) };
    }
}
