use super::commands::Command;
use super::id;
use super::model::{Model, PopupMenuModel, PopupMenuType, TdpModel, TdpSetting, TdpState};
use crate::battery::{BatteriesIterator, Battery};
use crate::ryzenadj::RyzenAdj;
use crate::winapi::show_error_message_box;
use std::mem::take;
use windows::core::{Error, Owned, PWSTR};
use windows::Win32::Foundation::{HWND, MAX_PATH};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, GetForegroundWindow, GetWindowThreadProcessId,
};

/// Controller owns the model and processes events coming from the window.
pub struct Controller {
    window: HWND,
    ryzen_adj: Option<RyzenAdj>,
    battery: Option<Battery>,
    model: Model,
}

impl Controller {
    /// # Safety
    ///
    /// The window handle should stay valid for the entire lifetime of the retutned instance.
    pub unsafe fn new(window: HWND) -> Self {
        let ryzen_adj = RyzenAdj::new().map_or_else(
            |err| {
                show_error_message_box(format!("Failed to initialize RyzenAdj: {}", err).as_str());
                None
            },
            Some,
        );
        let battery = BatteriesIterator::new().next().and_then(|r| {
            r.map_or_else(
                |err| {
                    show_error_message_box(format!("Failed to get battery info: {}", err).as_str());
                    None
                },
                Some,
            )
        });
        Controller {
            window,
            ryzen_adj,
            battery,
            model: Model::new(),
        }
    }

    fn get_tdp_limit(&self) -> Option<Result<u32, String>> {
        self.ryzen_adj.as_ref().map(|r| {
            r.get_table()
                .map(|t| t.get_fast_limit())
                .map_err(|e| e.to_string())
        })
    }

    fn get_charge_rate(&self) -> Option<Result<i32, String>> {
        // FIXME: Battery device stops working after charger disconnect
        self.battery
            .as_ref()
            .map(|b| b.get_charge_rate().map_err(|e| e.to_string()))
    }

    fn get_fg_application(&self) -> Result<String, Error> {
        // SAFETY: The call is always sound
        let hwnd = unsafe { GetForegroundWindow() };
        let mut pid = 0;
        // SAFETY: The provided pointer is valid for the duration of the WinAPI call
        let tid = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if tid == 0 {
            Err(Error::from_win32())?
        }
        // SAFETY: The call is always sound, we own the returned handle
        let p = unsafe { Owned::new(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)?) };
        let mut path = [0u16; MAX_PATH as usize];
        let mut len = MAX_PATH - 1;
        // SAFETY: The provided pointer is pointing to an allocated area of the specified size
        unsafe {
            QueryFullProcessImageNameW(
                *p,
                PROCESS_NAME_WIN32,
                PWSTR::from_raw(path.as_mut_ptr()),
                &mut len,
            )?
        };
        Ok(String::from_utf16_lossy(&path[..len as usize]))
    }

    fn get_tdp_options(&self) -> Vec<u32> {
        // TODO: Determine based on chip's max TDP
        vec![5000, 7500, 10000, 15000, 20000, 24000, 28000]
    }

    pub fn refresh_tdp(&mut self) -> Option<TdpModel> {
        let Some(mut value) = self.get_tdp_limit() else {
            return None;
        };
        let (options, mut state) = take(&mut self.model.tdp)
            .map(|m| (m.options, m.state))
            .unwrap_or_else(|| (self.get_tdp_options(), TdpState::Tracking));
        let mut target = None;
        let fg_app = self
            .get_fg_application()
            .unwrap_or_else(|_| String::new())
            .to_lowercase();
        if let Some(app_limit) = self.model.settings.app_limits.get(&fg_app) {
            let app_limit = *app_limit;
            target = Some(app_limit);
            state = TdpState::ForcingApplication {
                fallback: match value {
                    Ok(x) => Some(x),
                    Err(_) => None,
                },
            };
        } else if let TdpState::ForcingApplication { fallback } = state {
            // should stop forcing app
            match self.model.settings.tdp {
                TdpSetting::Tracking => {
                    target = fallback;
                    state = TdpState::Tracking;
                }
                TdpSetting::Forcing(x) => {
                    target = Some(x);
                    state = TdpState::Forcing;
                }
            };
        } else if let TdpSetting::Forcing(x) = self.model.settings.tdp {
            target = Some(x);
        }
        if let Some(target) = target {
            if let Some(ryzen_adj) = &mut self.ryzen_adj {
                if let Ok(current) = &value {
                    if target != *current {
                        value = match ryzen_adj.set_all_limits(target) {
                            Ok(()) => Ok(target),
                            Err(err) => Err(err.to_string()),
                        }
                    }
                }
            }
        }
        Some(TdpModel {
            value,
            options,
            state,
        })
    }

    pub fn on_timer(&mut self) {
        self.model.tdp = self.refresh_tdp();
        self.model.charge_icon = self.get_charge_rate();
    }

    pub fn on_command(&mut self, command: Command) {
        match command {
            Command::Observe => self.model.settings.tdp = TdpSetting::Tracking,
            Command::SetTdp(target) => self.model.settings.tdp = TdpSetting::Forcing(target),
            Command::Exit =>
            // SAFETY: It is sound to destroy the window we own
            unsafe { DestroyWindow(self.window).unwrap() },
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
