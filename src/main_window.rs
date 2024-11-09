mod controller;
mod id;
mod model;
mod view;

use self::controller::Controller;
use self::view::View;
use crate::gdip::GdiPlus;
use crate::icons::WM_NOTIFY_ICON;
use crate::winapi::get_instance_handle;
use std::marker::PhantomData;
use std::mem::take;
use std::ops::DerefMut;
use std::pin::Pin;
use windows::core::{w, Error};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetWindowLongPtrW, KillTimer, PostQuitMessage,
    RegisterClassExW, SetProcessDPIAware, SetTimer, SetWindowLongPtrW, CREATESTRUCTW,
    CW_USEDEFAULT, GWLP_USERDATA, HWND_MESSAGE, WINDOW_EX_STYLE, WM_COMMAND, WM_CREATE, WM_DESTROY,
    WM_EXITMENULOOP, WM_NCCREATE, WM_RBUTTONUP, WM_TIMER, WNDCLASSEXW, WS_OVERLAPPED,
};

pub struct MainWindow<'gdip> {
    handle: HWND,
    gdi_plus: &'gdip GdiPlus,
    controller: Option<Controller>,
    view: Option<View<'gdip>>,
    live_timers: Vec<id::Timer>,
    // This marks MainWindow as !Send and !Sync
    _marker: PhantomData<*const ()>,
}

impl<'gdip> MainWindow<'gdip> {
    pub fn new(gdi_plus: &'gdip GdiPlus) -> Pin<Box<Self>> {
        // SAFETY: The call does not have any preconditions and is always sound
        let result = unsafe { SetProcessDPIAware() };
        assert_ne!(result.0, 0, "SetProcessDPIAware failed");
        let window_class_name = w!("MainWindow");
        let instance = get_instance_handle();
        let wnd_class_params = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(Self::wnd_proc),
            hInstance: instance,
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
            gdi_plus,
            controller: None,
            view: None,
            live_timers: vec![],
            _marker: PhantomData,
        });
        // SAFETY: The function is sound as long as all arguments are valid
        let handle = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                window_class_name,
                w!("MainWindow"),
                WS_OVERLAPPED,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                HWND_MESSAGE,
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

    fn with_controller(&mut self, f: impl FnOnce(&mut Controller)) {
        if let Some(controller) = &mut self.controller {
            f(controller);
            if let Some(view) = &mut self.view {
                let model = controller.get_model();
                view.update(model);
            }
        }
    }

    fn process_message(&mut self, message: u32, w_param: WPARAM, l_param: LPARAM) -> Option<isize> {
        match message {
            WM_CREATE => {
                // SAFETY: The window handle is valid now and will stay valid
                //   until view and controller are dropped
                self.view = Some(unsafe { View::new(self.handle, self.gdi_plus) });
                self.controller = Some(unsafe { Controller::new(self.handle) });
                let result = unsafe { SetTimer(self.handle, id::Timer::Main as usize, 1000, None) };
                if result == 0 {
                    panic!("Set timer failed: {}", Error::from_win32());
                }
                self.live_timers.push(id::Timer::Main);
            }
            WM_TIMER => {
                if w_param.0 == id::Timer::Main as usize {
                    self.with_controller(|c| c.on_timer());
                }
            }
            WM_COMMAND => {
                let msg_source = w_param.0 as u32 >> 16;
                let id = w_param.0 as u16 as u32;
                if msg_source == 0 {
                    self.with_controller(|c| c.on_menu_item_click(id));
                }
            }
            WM_EXITMENULOOP => {
                self.with_controller(|c| c.on_menu_dismissed());
            }
            WM_NOTIFY_ICON => {
                let event = l_param.0 as u16 as u32;
                let id = l_param.0 as u32 >> 16;
                if event == WM_RBUTTONUP {
                    let x = w_param.0 as i16 as i32;
                    let y = (w_param.0 >> 16) as i16 as i32;
                    self.with_controller(|c| c.on_notify_icon_click(id, x, y));
                }
            }
            WM_DESTROY => {
                self.view = None;
                self.controller = None;
                for timer in take(&mut self.live_timers) {
                    // SAFETY: The timer was created before its id got into live timers
                    unsafe { KillTimer(self.handle, timer as usize).unwrap() }
                }
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
