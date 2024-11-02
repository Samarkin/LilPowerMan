use crate::battery::Battery;
use crate::icons::{NotifyIcon, WM_NOTIFY_ICON};
use crate::ryzenadj::RyzenAdj;
use crate::winapi::{get_default_cursor, get_instance_handle, PaintContext};
use std::marker::PhantomData;
use std::mem::take;
use std::ops::DerefMut;
use std::pin::Pin;
use windows::core::{w, Error, Owned, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CheckMenuItem, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyWindow,
    GetWindowLongPtrW, InsertMenuItemW, KillTimer, MessageBoxW, PostQuitMessage, RegisterClassExW,
    SetForegroundWindow, SetProcessDPIAware, SetTimer, SetWindowLongPtrW, TrackPopupMenu,
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, MB_OK, MENUITEMINFOW,
    MFS_ENABLED, MFT_STRING, MF_BYCOMMAND, MF_CHECKED, MIIM_FTYPE, MIIM_ID, MIIM_STRING,
    TPM_LEFTBUTTON, WINDOW_EX_STYLE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_NCCREATE, WM_PAINT,
    WM_RBUTTONUP, WM_TIMER, WNDCLASSEXW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

const IDT_MAIN_TIMER: usize = 0;
const IDM_HELLO_WORLD: u32 = 123;

pub struct MainWindow {
    handle: HWND,
    ryzen_adj: Option<RyzenAdj>,
    battery: Option<Battery>,
    tdp_icon: Option<NotifyIcon>,
    charge_icon: Option<NotifyIcon>,
    live_timers: Vec<usize>,
    // This marks MainWindow as !Send and !Sync
    _marker: PhantomData<*const ()>,
}

impl MainWindow {
    pub fn new(ryzen_adj: Option<RyzenAdj>, battery: Option<Battery>) -> Pin<Box<MainWindow>> {
        // SAFETY: The call does not have any preconditions and is always sound
        let result = unsafe { SetProcessDPIAware() };
        assert_ne!(result.0, 0, "SetProcessDPIAware failed");
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
            ryzen_adj,
            battery,
            tdp_icon: None,
            charge_icon: None,
            live_timers: vec![],
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

    fn get_text(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut text = String::new();
        if let Some(ryzen_adj) = &self.ryzen_adj {
            text += &format!("Current TDP: {} W", ryzen_adj.get_table()?.get_fast_limit());
        }
        if let Some(battery) = &self.battery {
            text += &format!(", Battery charge rate {} mW", battery.get_charge_rate()?);
        }
        Ok(text)
    }

    fn update_tdp_icon(icon: &mut NotifyIcon, ryzen_adj: &RyzenAdj) {
        match ryzen_adj.get_table() {
            Ok(table) => {
                let fast_limit = table.get_fast_limit();
                icon.update(
                    format!("Current TDP: {} W", fast_limit).as_str(),
                    format!("{}", fast_limit as i32).as_str(),
                );
            }
            Err(err) => icon.update(
                format!("Failed to get TDP information: {}", err).as_str(),
                "🛑",
            ),
        }
    }

    fn update_charge_icon(icon: &mut NotifyIcon, battery: &Battery) {
        match battery.get_charge_rate() {
            Ok(charge_rate) => icon.update(
                format!("Battery charge rate: {} mW", charge_rate).as_str(),
                format!("{}", charge_rate / 1000).as_str(),
            ),
            Err(err) => icon.update(
                format!("Failed to get battery information: {}", err).as_str(),
                "🛑",
            ),
        }
    }

    fn process_message(&mut self, message: u32, w_param: WPARAM, l_param: LPARAM) -> Option<isize> {
        match message {
            WM_CREATE => {
                let mut id = 0;
                if let Some(ryzen_adj) = &self.ryzen_adj {
                    // SAFETY: Window handle is valid, number of icons is not expected to reach u32::MAX
                    let mut tdp_icon = unsafe { NotifyIcon::new(self.handle, id).unwrap() };
                    Self::update_tdp_icon(&mut tdp_icon, ryzen_adj);
                    self.tdp_icon = Some(tdp_icon);
                    id += 1;
                }
                if let Some(battery) = &self.battery {
                    let mut charge_icon = unsafe { NotifyIcon::new(self.handle, id).unwrap() };
                    Self::update_charge_icon(&mut charge_icon, battery);
                    self.charge_icon = Some(charge_icon);
                    id += 1;
                }
                let result = unsafe { SetTimer(self.handle, IDT_MAIN_TIMER, 1000, None) };
                if result == 0 {
                    panic!("Set timer failed: {}", Error::from_win32());
                }
                self.live_timers.push(IDT_MAIN_TIMER);
            }
            WM_TIMER if w_param.0 == IDT_MAIN_TIMER => {
                if let Some(icon) = &mut self.tdp_icon {
                    if let Some(ryzen_adj) = &self.ryzen_adj {
                        Self::update_tdp_icon(icon, ryzen_adj);
                    }
                }
                if let Some(icon) = &mut self.charge_icon {
                    if let Some(battery) = &self.battery {
                        Self::update_charge_icon(icon, battery);
                    }
                }
            }
            WM_COMMAND => {
                let msg_source = w_param.0 as u32 >> 16;
                let id = w_param.0 as u16 as u32;
                if msg_source == 0 && id == IDM_HELLO_WORLD {
                    unsafe {
                        MessageBoxW(
                            self.handle,
                            w!("You clicked it!"),
                            w!("Hello, menu item!"),
                            MB_OK,
                        )
                    };
                }
            }
            WM_NOTIFY_ICON => {
                if let Some(icon) = &self.tdp_icon {
                    let event = l_param.0 as u16 as u32;
                    let id = l_param.0 as u32 >> 16;
                    if id == icon.get_id() && event == WM_RBUTTONUP {
                        let x = w_param.0 as i16 as i32;
                        let y = (w_param.0 >> 16) as i16 as i32;
                        let result = unsafe { SetForegroundWindow(self.handle) };
                        assert_ne!(result.0, 0, "Failed to set foreground window");
                        let menu = unsafe { Owned::new(CreatePopupMenu().unwrap()) };
                        let mut buf: Vec<u16> = "Hello, world!".encode_utf16().collect();
                        buf.push(0); // null-terminate
                        let menu_item_info = MENUITEMINFOW {
                            cbSize: size_of::<MENUITEMINFOW>() as u32,
                            fMask: MIIM_FTYPE | MIIM_ID | MIIM_STRING,
                            fType: MFT_STRING,
                            fState: MFS_ENABLED,
                            wID: IDM_HELLO_WORLD,
                            dwTypeData: PWSTR(buf.as_mut_ptr()),
                            ..Default::default()
                        };
                        unsafe { InsertMenuItemW(*menu, 0, true, &menu_item_info).unwrap() };
                        let result = unsafe {
                            CheckMenuItem(*menu, IDM_HELLO_WORLD, MF_BYCOMMAND.0 | MF_CHECKED.0)
                        };
                        assert_ne!(result, u32::MAX, "CheckMenuItem failed: Item not found");
                        let result = unsafe {
                            TrackPopupMenu(*menu, TPM_LEFTBUTTON, x, y, 0, self.handle, None)
                        };
                        if result.0 == 0 {
                            panic!("TrackPopupMenu failed: {}", Error::from_win32());
                        }
                    }
                }
            }
            WM_PAINT => {
                // SAFETY: We are responding to the WM_PAINT message
                let mut pc = unsafe { PaintContext::for_window(self.handle) };
                let text = self.get_text().unwrap_or_else(|e| format!("Error: {}", e));
                pc.draw_text(&text, 0, 0);
            }
            WM_DESTROY => {
                for timer in take(&mut self.live_timers) {
                    // SAFETY: The timer was created before its id got into live timers
                    unsafe { KillTimer(self.handle, timer).unwrap() }
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

impl Drop for MainWindow {
    fn drop(&mut self) {
        // The icons should get dropped before the window
        self.tdp_icon = None;
        self.charge_icon = None;
        if !self.handle.is_invalid() {
            // SAFETY: MainWindow can only contain a handle that it owns
            let _ = unsafe { DestroyWindow(self.handle) };
        }
    }
}
