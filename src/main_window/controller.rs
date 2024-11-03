use super::id;
use super::model::{Model, PopupMenuModel, PopupMenuType, TdpIconColor, TdpIconModel};
use super::Measurements;
use windows::core::w;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{DestroyWindow, MessageBoxW, MB_OK};

enum State {
    Tracking,
    Forcing(u32),
}

/// Controller owns the model and processes events coming from the window.
pub struct Controller {
    window: HWND,
    model: Model,
    state: State,
}

impl Controller {
    /// # Safety
    ///
    /// The window handle should stay valid for the entire lifetime of the retutned instance.
    pub unsafe fn new(window: HWND) -> Self {
        Controller {
            window,
            model: Model::new(),
            state: State::Tracking,
        }
    }

    fn get_tdp_icon_color(&self) -> TdpIconColor {
        match self.state {
            State::Tracking => TdpIconColor::Tracking,
            State::Forcing(_) => TdpIconColor::Forcing,
        }
    }

    pub fn on_timer(&mut self, measurements: Measurements) {
        self.model.tdp_icon = measurements.tdp_limit.map(|v| TdpIconModel {
            value: v,
            color: self.get_tdp_icon_color(),
        });
        self.model.charge_icon = measurements.charge_rate;
    }

    pub fn on_menu_item_click(&mut self, id: u32) {
        if id == id::MenuItem::HelloWorld as _ {
            unsafe {
                MessageBoxW(
                    self.window,
                    w!("You clicked it!"),
                    w!("Hello, menu item!"),
                    MB_OK,
                );
            }
        } else if id == id::MenuItem::Exit as _ {
            // SAFETY: It is sound to destroy the window we own
            unsafe { DestroyWindow(self.window).unwrap() };
        }
    }

    pub fn on_notify_icon_click(&mut self, id: u32, x: i32, y: i32) {
        if id == id::NotifyIcon::TdpLimit as _ {
            self.model.popup_menu = Some(PopupMenuModel {
                x,
                y,
                menu: PopupMenuType::TdpIcon,
            })
        }
    }

    pub fn on_menu_dismissed(&mut self) {
        self.model.popup_menu = None;
    }

    pub fn get_model(&self) -> &Model {
        &self.model
    }
}
