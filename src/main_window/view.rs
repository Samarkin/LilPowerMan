use super::commands::Command;
use super::id;
use super::model::{Model, PopupMenuType, TdpModel, TdpSetting, TdpState};
use crate::gdip::{Color, GdiPlus};
use crate::icons::NotifyIcon;
use crate::menu::PopupMenu;
use std::mem::replace;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::EndMenu;

const IDM_TDP_START: u32 = 1;

/// View owns the UI components and renders model in the window.
pub struct View<'gdip> {
    window: HWND,
    gdi_plus: &'gdip GdiPlus,
    model: Model,
    tdp_icon: Option<NotifyIcon<'gdip>>,
    tdp_icon_popup_menu: Option<PopupMenu>,
    charge_icon: Option<NotifyIcon<'gdip>>,
    tdp_menu_item_commands: Vec<Command>,
}

impl<'gdip> View<'gdip> {
    /// # Safety
    ///
    /// The window handle should stay valid for the entire lifetime of the retutned instance.
    pub unsafe fn new(window: HWND, gdi_plus: &'gdip GdiPlus) -> Self {
        View {
            window,
            gdi_plus,
            model: Model::new(),
            tdp_icon: None,
            tdp_icon_popup_menu: None,
            charge_icon: None,
            tdp_menu_item_commands: vec![],
        }
    }

    /// Updates UI according to the provided model.
    pub fn update(&mut self, new_model: &Model) {
        let old_model = replace(&mut self.model, new_model.clone());
        if let Some(tdp) = &new_model.tdp {
            self.update_tdp_icon(&old_model.tdp, tdp);
            self.update_tdp_menu(&old_model.tdp, tdp);
            self.update_tdp_selection(&old_model, &new_model);
        } else {
            self.tdp_icon = None;
            self.tdp_icon_popup_menu = None;
        }
        if let Some(charge_icon_model) = &new_model.charge_icon {
            // SAFETY: Window handle's validity is guaranteed by the owner
            let charge_icon = self.charge_icon.get_or_insert_with(|| unsafe {
                NotifyIcon::new(self.window, id::NotifyIcon::ChargeRate as _, self.gdi_plus)
                    .unwrap()
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
            NotifyIcon::new(self.window, id::NotifyIcon::TdpLimit as _, self.gdi_plus).unwrap()
        });
        match model.value {
            Ok(ref tdp_limit) => {
                tdp_icon.update(
                    format!("Current TDP: {} mW", tdp_limit).as_str(),
                    format!("{}", tdp_limit / 1000).as_str(),
                    match model.state {
                        TdpState::Tracking => Color::CYAN,
                        TdpState::Forcing => Color::WHITE,
                        TdpState::ForcingApplication { .. } => Color::YELLOW,
                    },
                );
            }
            Err(ref err) => {
                tdp_icon.update(
                    format!("Failed to get TDP information: {}", err).as_str(),
                    "🛑",
                    Color::RED,
                );
            }
        }
    }

    pub fn get_command_for_menu_item(&self, id: u32) -> Option<Command> {
        if id >= IDM_TDP_START && id < IDM_TDP_START + self.tdp_menu_item_commands.len() as u32 {
            self.tdp_menu_item_commands
                .get((id - IDM_TDP_START) as usize)
                .copied()
        } else {
            None
        }
    }

    fn add_tdp_command(&mut self, command: Command) -> u32 {
        let id = IDM_TDP_START + self.tdp_menu_item_commands.len() as u32;
        self.tdp_menu_item_commands.push(command);
        id
    }

    fn update_tdp_menu(&mut self, old_model: &Option<TdpModel>, model: &TdpModel) {
        if let Some(old_model) = old_model {
            if old_model.options == model.options {
                // Nothing to update
                return;
            }
        }
        // TODO: Update the existing menu instead of building a new one from scratch
        self.tdp_menu_item_commands.clear();
        let mut menu = PopupMenu::new();
        let id = self.add_tdp_command(Command::Observe);
        menu.append_menu_item("Just &observe", id);
        for tdp in &model.options {
            let id = self.add_tdp_command(Command::SetTdp(*tdp));
            menu.append_menu_item(&format!("{} W", (*tdp as f32) / 1000.0), id);
        }
        menu.append_separator();
        let id = self.add_tdp_command(Command::Exit);
        menu.append_menu_item("E&xit", id);
        self.tdp_icon_popup_menu = Some(menu);
    }

    fn update_tdp_selection(&mut self, old_model: &Model, model: &Model) {
        if model.settings == old_model.settings
            && model.tdp.as_ref().map(|t| &t.options) == old_model.tdp.as_ref().map(|t| &t.options)
        {
            return;
        }
        let Some(menu) = &mut self.tdp_icon_popup_menu else {
            return;
        };
        let checked_cmd = match model.settings.tdp {
            TdpSetting::Tracking => Command::Observe,
            TdpSetting::Forcing(x) => Command::SetTdp(x),
        };
        for (i, cmd) in self.tdp_menu_item_commands.iter().enumerate() {
            let id = i as u32 + IDM_TDP_START;
            menu.check_menu_item(id, cmd == &checked_cmd);
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
        match model {
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
                        Color::GREEN
                    } else {
                        Color::WHITE
                    },
                );
            }
            Err(err) => {
                charge_icon.update(
                    format!("Failed to get battery information: {}", err).as_str(),
                    "🛑",
                    Color::RED,
                );
            }
        }
    }
}
