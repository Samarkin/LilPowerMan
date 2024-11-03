use super::id;
use super::model::{Model, PopupMenuType, TdpIconModel};
use crate::icons::NotifyIcon;
use crate::menu::PopupMenu;
use std::mem::replace;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::EndMenu;

/// View owns the UI components and renders model in the window.
pub struct View {
    window: HWND,
    model: Model,
    tdp_icon: Option<NotifyIcon>,
    tdp_icon_popup_menu: PopupMenu,
    charge_icon: Option<NotifyIcon>,
}

impl View {
    /// # Safety
    ///
    /// The window handle should stay valid for the entire lifetime of the retutned instance.
    pub unsafe fn new(window: HWND) -> Self {
        let mut tdp_icon_popup_menu = PopupMenu::new();
        tdp_icon_popup_menu.append_menu_item("Hello, world!", id::MenuItem::HelloWorld as _);
        tdp_icon_popup_menu.append_separator();
        tdp_icon_popup_menu.append_menu_item("E&xit", id::MenuItem::Exit as _);
        View {
            window,
            model: Model::new(),
            tdp_icon: None,
            tdp_icon_popup_menu,
            charge_icon: None,
        }
    }

    /// Updates UI according to the provided model.
    pub fn update(&mut self, new_model: &Model) {
        let old_model = replace(&mut self.model, new_model.clone());
        if let Some(tdp_icon_model) = &new_model.tdp_icon {
            // SAFETY: Window handle's validity is guaranteed by the owner
            let tdp_icon = self.tdp_icon.get_or_insert_with(|| unsafe {
                NotifyIcon::new(self.window, id::NotifyIcon::TdpLimit as _).unwrap()
            });
            Self::update_tdp_icon(tdp_icon, &old_model.tdp_icon, tdp_icon_model);
        } else {
            self.tdp_icon = None;
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
                // SAFETY: The handle points to a currently live window
                unsafe { menu.show(popup_menu.x, popup_menu.y, self.window) }
            }
        }
    }

    fn update_tdp_icon(
        tdp_icon: &mut NotifyIcon,
        old_model: &Option<TdpIconModel>,
        model: &TdpIconModel,
    ) {
        if Some(model) == old_model.as_ref() {
            return;
        }
        match &model.value {
            &Ok(tdp_limit) => {
                tdp_icon.update(
                    format!("Current TDP: {} mW", tdp_limit).as_str(),
                    format!("{}", tdp_limit / 1000).as_str(),
                );
            }
            Err(err) => {
                tdp_icon.update(
                    format!("Failed to get TDP information: {}", err).as_str(),
                    "🛑",
                );
            }
        }
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
            &Ok(charge_rate) => {
                charge_icon.update(
                    format!("Battery charge rate: {} mW", charge_rate).as_str(),
                    format!("{}", charge_rate / 1000).as_str(),
                );
            }
            Err(err) => {
                charge_icon.update(
                    format!("Failed to get battery information: {}", err).as_str(),
                    "🛑",
                );
            }
        }
    }
}
