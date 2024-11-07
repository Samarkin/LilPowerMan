use super::id;
use super::model::{Model, PopupMenuType, TdpModel, TdpState};
use crate::icons::NotifyIcon;
use crate::menu::PopupMenu;
use crate::winapi::colors::{COLOR_BLACK, COLOR_CYAN, COLOR_GREEN, COLOR_RED, COLOR_WHITE};
use std::mem::replace;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::EndMenu;

/// View owns the UI components and renders model in the window.
pub struct View {
    window: HWND,
    model: Model,
    tdp_icon: Option<NotifyIcon>,
    tdp_icon_popup_menu: Option<PopupMenu>,
    charge_icon: Option<NotifyIcon>,
}

impl View {
    /// # Safety
    ///
    /// The window handle should stay valid for the entire lifetime of the retutned instance.
    pub unsafe fn new(window: HWND) -> Self {
        View {
            window,
            model: Model::new(),
            tdp_icon: None,
            tdp_icon_popup_menu: None,
            charge_icon: None,
        }
    }

    /// Updates UI according to the provided model.
    pub fn update(&mut self, new_model: &Model) {
        let old_model = replace(&mut self.model, new_model.clone());
        if let Some(tdp) = &new_model.tdp {
            self.update_tdp_icon(&old_model.tdp, tdp);
            self.update_tdp_menu(&old_model.tdp, tdp);
        } else {
            self.tdp_icon = None;
            self.tdp_icon_popup_menu = None;
        }
        if let Some(charge_icon_model) = &new_model.charge_icon {
            // SAFETY: Window handle's validity is guaranteed by the owner
            let charge_icon = self.charge_icon.get_or_insert_with(|| unsafe {
                NotifyIcon::new(self.window, id::NotifyIcon::ChargeRate as _).unwrap()
            });
            Self::update_charge_icon(charge_icon, &old_model.charge_icon, charge_icon_model);
        } else {
            self.charge_icon = None;
        }
        if new_model.popup_menu != old_model.popup_menu {
            // SAFETY: The call is always sound, but will return an error
            //   if there is currently no menu displayed
            let _ = unsafe { EndMenu() };
            if let Some(popup_menu) = &new_model.popup_menu {
                let menu = match popup_menu.menu {
                    PopupMenuType::TdpIcon => &self.tdp_icon_popup_menu,
                };
                if let Some(menu) = menu {
                    // SAFETY: The handle points to a currently live window
                    unsafe { menu.show(popup_menu.x, popup_menu.y, self.window) }
                }
            }
        }
    }

    fn update_tdp_icon(&mut self, old_model: &Option<TdpModel>, model: &TdpModel) {
        if let Some(old_model) = old_model {
            if old_model.state == model.state && old_model.value == model.value {
                // Nothing to update
                return;
            }
        }
        // SAFETY: Window handle's validity is guaranteed by the owner
        let tdp_icon = self.tdp_icon.get_or_insert_with(|| unsafe {
            NotifyIcon::new(self.window, id::NotifyIcon::TdpLimit as _).unwrap()
        });
        match &model.value {
            &Ok(tdp_limit) => {
                tdp_icon.update(
                    format!("Current TDP: {} mW", tdp_limit).as_str(),
                    format!("{}", tdp_limit / 1000).as_str(),
                    COLOR_BLACK,
                    if model.state == TdpState::Tracking {
                        COLOR_CYAN
                    } else {
                        COLOR_WHITE
                    },
                );
            }
            Err(err) => {
                tdp_icon.update(
                    format!("Failed to get TDP information: {}", err).as_str(),
                    "🛑",
                    COLOR_RED,
                    COLOR_WHITE,
                );
            }
        }
    }

    fn update_tdp_menu(&mut self, old_model: &Option<TdpModel>, model: &TdpModel) {
        if let Some(old_model) = old_model {
            if old_model.menu_items == model.menu_items && old_model.state == model.state {
                // Nothing to update
                return;
            }
        }
        // TODO: Update the existing menu instead of building a new one from scratch
        let mut menu = PopupMenu::new();
        let id = id::MenuItem::Observe as _;
        menu.append_menu_item("Just observe", id);
        menu.check_menu_item(id, model.state == TdpState::Tracking);
        for x in &model.menu_items {
            let id = id::MenuItem::SetTdpBegin as u32 + x;
            menu.append_menu_item(format!("{x} W").as_str(), id);
            menu.check_menu_item(id, model.state == TdpState::Forcing(x * 1000));
        }
        menu.append_separator();
        menu.append_menu_item("E&xit", id::MenuItem::Exit as _);
        self.tdp_icon_popup_menu = Some(menu);
    }

    fn update_charge_icon(
        charge_icon: &mut NotifyIcon,
        old_model: &Option<Result<i32, String>>,
        model: &Result<i32, String>,
    ) {
        if Some(model) == old_model.as_ref() {
            return;
        }
        match &model {
            Ok(charge_rate) => {
                let is_charging = *charge_rate >= 0;
                let abs_rate = charge_rate.abs();
                let is_single_digit = abs_rate < 10000;
                charge_icon.update(
                    format!("Battery charge rate: {} mW", charge_rate).as_str(),
                    if is_single_digit {
                        format!("{}.{}", abs_rate / 1000, (abs_rate / 100) % 10)
                    } else {
                        format!("{}", abs_rate / 1000)
                    }
                    .as_str(),
                    if is_charging {
                        COLOR_BLACK
                    } else {
                        COLOR_WHITE
                    },
                    if is_charging { COLOR_GREEN } else { COLOR_RED },
                );
            }
            Err(err) => {
                charge_icon.update(
                    format!("Failed to get battery information: {}", err).as_str(),
                    "🛑",
                    COLOR_RED,
                    COLOR_WHITE,
                );
            }
        }
    }
}
