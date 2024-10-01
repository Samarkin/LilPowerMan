#![windows_subsystem = "windows"]

mod winapi;

use winapi::{
    dispatch_message, get_default_cursor, get_instance_handle, get_message, translate_message,
};
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, PostQuitMessage, RegisterClassExW, CS_HREDRAW,
            CS_VREDRAW, CW_USEDEFAULT, MSG, WM_DESTROY, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
            WS_VISIBLE,
        },
    },
};

fn main() -> Result<()> {
    let window_class_name = w!("MainWindow");
    unsafe {
        let instance = get_instance_handle()?;
        let window_class_atom = RegisterClassExW(&WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(process_message),
            hInstance: instance,
            hCursor: get_default_cursor()?,
            lpszClassName: window_class_name,
            ..Default::default()
        });
        debug_assert!(window_class_atom != 0);
        let _window_handle = CreateWindowExW(
            Default::default(),
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
        )?;
    }

    let mut msg: MSG = Default::default();
    while get_message(&mut msg)? {
        translate_message(&msg);
        dispatch_message(&msg);
    }
    Ok(())
}

unsafe extern "system" fn process_message(
    window: HWND,
    message: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match message {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(window, message, w_param, l_param),
    }
}
