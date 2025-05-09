use super::commands::Command;
use super::id;
use super::model::{Model, PopupMenuModel, PopupMenuType, TdpModel, TdpState};
use crate::battery::{BatteriesIterator, Battery, BatteryStatus, Error as BatteryError};
use crate::rtss::{Error as RtssError, Rtss};
use crate::ryzenadj::RyzenAdj;
use crate::settings::{SettingsStorage, TdpSetting};
use crate::winapi::show_error_message_box;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::mem::take;
use std::os::windows::ffi::OsStringExt;
use windows::core::{Error, Owned, PWSTR};
use windows::Win32::Foundation::{ERROR_NO_SUCH_DEVICE, HWND, MAX_PATH};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, GetForegroundWindow, GetWindowThreadProcessId,
};

const MAX_RECENT_APPLICATIONS: usize = 5;

/// Controller owns the model and processes events coming from the window.
pub struct Controller {
    window: HWND,
    ryzen_adj: Option<RyzenAdj>,
    battery: Option<Battery>,
    rtss: Rtss,
    settings_storage: SettingsStorage,
    self_path: Option<OsString>,
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
            |r| {
                trace!("RyzenAdj initialized");
                Some(r)
            },
        );
        let battery = BatteriesIterator::new().next().and_then(|r| {
            r.map_or_else(
                |err| {
                    show_error_message_box(format!("Failed to get battery info: {}", err).as_str());
                    None
                },
                |b| {
                    trace!("Battery module initialized");
                    Some(b)
                },
            )
        });
        assert!(
            ryzen_adj.is_some() || battery.is_some(),
            "All subsystems failed to initialize"
        );

        let rtss = Rtss::new();
        let settings_storage = SettingsStorage::new();
        let model = Model::new(&settings_storage);
        Controller {
            window,
            ryzen_adj,
            battery,
            rtss,
            settings_storage,
            model,
            self_path: Self::get_self_path().ok(),
        }
    }

    fn get_tdp_limit(&self) -> Option<Result<u32, String>> {
        self.ryzen_adj.as_ref().map(|r| {
            r.get_table()
                .map(|t| t.get_fast_limit())
                .map_err(|e| e.to_string())
        })
    }

    fn get_battery_status(&mut self) -> Option<Result<BatteryStatus, String>> {
        let mut result = self.battery.as_ref().map(Battery::get_status);
        if let Some(Err(BatteryError::WindowsError(err))) = &result {
            if err == &Error::from(ERROR_NO_SUCH_DEVICE) {
                match BatteriesIterator::new().next() {
                    None => {
                        show_error_message_box("Battery disconnected");
                        result = None;
                        self.battery = None;
                    }
                    Some(Ok(new_battery)) => {
                        result = Some(new_battery.get_status());
                        self.battery = Some(new_battery);
                    }
                    Some(Err(e)) => {
                        result = Some(Err(e));
                    }
                }
            }
        }
        result.map(|r| r.map_err(|e| e.to_string()))
    }

    fn get_fg_application_pid() -> Result<u32, Error> {
        // SAFETY: The call is always sound
        let hwnd = unsafe { GetForegroundWindow() };
        let mut pid = 0;
        // SAFETY: The provided pointer is valid for the duration of the WinAPI call
        let tid = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if tid == 0 {
            Err(Error::from_win32())?
        }
        Ok(pid)
    }

    fn get_application_path(pid: u32) -> Result<OsString, Error> {
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
        Ok(OsString::from_wide(&path[..len as usize]).to_ascii_lowercase())
    }

    fn get_self_pid() -> u32 {
        // SAFETY: The call is always sound
        unsafe { GetCurrentProcessId() }
    }

    fn get_self_path() -> Result<OsString, Error> {
        Self::get_application_path(Self::get_self_pid())
    }

    fn get_fg_application() -> Result<OsString, Error> {
        Self::get_fg_application_pid().and_then(Self::get_application_path)
    }

    fn get_tdp_options(&self) -> Vec<u32> {
        // TODO: Determine based on chip's max TDP
        vec![5000, 7500, 10000, 15000, 20000, 24000, 28000]
    }

    fn refresh_tdp(&mut self) -> Option<TdpModel> {
        let Some(mut value) = self.get_tdp_limit() else {
            trace!("Bypassing TDP refresh");
            return None;
        };
        trace!("Refreshing TDP model");
        let (options, mut applications, old_state) = take(&mut self.model.tdp)
            .map(|m| (m.options, m.applications, m.state))
            .unwrap_or_else(|| (self.get_tdp_options(), VecDeque::new(), TdpState::Tracking));
        let target;
        let state;
        let fg_app = Self::get_fg_application().ok();
        let app_limit = fg_app
            .as_ref()
            .and_then(|s| self.model.settings.get_app_limit(s));
        if let Some(app_limit) = app_limit {
            target = Some(app_limit);
            state = match old_state {
                TdpState::ForcingApplication { .. } => old_state,
                TdpState::Forcing => TdpState::ForcingApplication { fallback: None },
                TdpState::Tracking => TdpState::ForcingApplication {
                    fallback: match value {
                        Ok(x) => Some(x),
                        Err(_) => None,
                    },
                },
            };
        } else {
            // should stop forcing app
            match self.model.settings.get_tdp_setting() {
                TdpSetting::Forcing(x) => {
                    target = Some(x);
                    state = TdpState::Forcing;
                }
                TdpSetting::Tracking => {
                    if let TdpState::ForcingApplication { fallback } = old_state {
                        target = fallback;
                    } else {
                        target = None;
                    }
                    state = TdpState::Tracking;
                }
            }
        }
        if let Some(fg_app) = fg_app {
            if Some(&fg_app) != self.self_path.as_ref() && !applications.contains(&fg_app) {
                applications.push_front(fg_app);
                while applications.len() > MAX_RECENT_APPLICATIONS {
                    applications.pop_back();
                }
            }
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
            applications,
            state,
        })
    }

    fn update_rtss(&mut self, battery_status: &BatteryStatus) {
        match self.rtss.update(battery_status) {
            Ok(()) => {}
            Err(RtssError::RtssV2NotRunning) => {}
            Err(err) => error!("Failed to update RTSS shared memory: {}", err),
        }
    }

    pub fn on_timer(&mut self) {
        self.model.tdp = self.refresh_tdp();
        let battery_status = self.get_battery_status();
        if let Some(Ok(status)) = &battery_status {
            self.update_rtss(&status);
        }
        self.model.charge_icon = battery_status.map(|r| r.map(|s| s.charge_rate));
    }

    pub fn on_command(&mut self, command: Command) {
        match command {
            Command::Observe => self
                .settings_storage
                .set_tdp_setting(&mut self.model.settings, TdpSetting::Tracking),
            Command::ResetApplicationTdp(app) => self
                .settings_storage
                .remove_app_limit(&mut self.model.settings, &app),
            Command::SetApplicationTdp(app, limit) => {
                self.settings_storage
                    .set_app_limit(&mut self.model.settings, app, limit)
            }
            Command::SetTdp(target) => self
                .settings_storage
                .set_tdp_setting(&mut self.model.settings, TdpSetting::Forcing(target)),
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
        } else if id == id::NotifyIcon::ChargeRate as _ {
            self.model.popup_menu = Some(PopupMenuModel {
                x,
                y,
                menu: PopupMenuType::ChargeIcon,
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
