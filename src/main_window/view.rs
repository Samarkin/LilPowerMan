use super::commands::Command;
use super::id;
use super::model::{Model, PopupMenuType, TdpModel, TdpState};
use crate::gdip::{Color, GdiPlus};
use crate::icons::NotifyIcon;
use crate::menu::PopupMenu;
use crate::settings::TdpSetting;
use std::mem::replace;
use std::path::Path;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::EndMenu;

const IDM_TDP_START: u32 = 1;
const IDM_CHARGE_START: u32 = 257;

/// View owns the UI components and renders model in the window.
pub struct View<'gdip> {
    window: HWND,
    gdi_plus: &'gdip GdiPlus,
    model: Model,
    tdp_icon: Option<NotifyIcon<'gdip>>,
    tdp_icon_popup_menu: Option<PopupMenu>,
    tdp_icon_menu_commands: Vec<Command>,
    charge_icon: Option<NotifyIcon<'gdip>>,
    charge_icon_popup_menu: Option<PopupMenu>,
    charge_icon_menu_commands: Vec<Command>,
}

impl<'gdip> View<'gdip> {
    /// # Safety
    ///
    /// The window handle should stay valid for the entire lifetime of the retutned instance.
    pub unsafe fn new(window: HWND, gdi_plus: &'gdip GdiPlus) -> Self {
        View {
            window,
            gdi_plus,
            model: Model::default(),
            tdp_icon: None,
            tdp_icon_popup_menu: None,
            tdp_icon_menu_commands: vec![],
            charge_icon: None,
            charge_icon_popup_menu: None,
            charge_icon_menu_commands: vec![],
        }
    }

    /// Updates UI according to the provided model.
    pub fn update(&mut self, new_model: &Model) {
        let old_model = replace(&mut self.model, new_model.clone());
        if let Some(tdp) = &new_model.tdp {
            self.update_tdp_icon(&old_model.tdp, tdp);
            let menu_rebuilt = self.update_tdp_menu(&old_model.tdp, tdp);
            self.update_tdp_selection(&old_model, &new_model, menu_rebuilt);
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
            self.build_charge_icon_menu();
        } else {
            self.charge_icon = None;
            self.charge_icon_popup_menu = None;
        }
        if new_model.popup_menu != old_model.popup_menu {
            // SAFETY: The call is always sound, but will return an error
            //   if there is currently no menu displayed
            let _ = unsafe { EndMenu() };
            if let Some(popup_menu) = &new_model.popup_menu {
                let menu = match popup_menu.menu {
                    PopupMenuType::TdpIcon => &self.tdp_icon_popup_menu,
                    PopupMenuType::ChargeIcon => &self.charge_icon_popup_menu,
                };
                if let Some(menu) = menu {
                    // SAFETY: The handle points to a currently live window
                    _ = unsafe { menu.show(popup_menu.x, popup_menu.y, self.window) }
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
                let tip;
                let color;
                match model.state {
                    TdpState::Tracking => {
                        tip = format!("Current TDP: {} mW", tdp_limit);
                        color = Color::CYAN;
                    }
                    TdpState::Forcing => {
                        tip = format!("TDP setting: {} mW", tdp_limit);
                        color = Color::WHITE;
                    }
                    TdpState::ForcingApplication { .. } => {
                        tip = format!("Application TDP setting: {} mW", tdp_limit);
                        color = Color::YELLOW;
                    }
                };
                let text = format!("{}", tdp_limit / 1000);
                tdp_icon.update(tip.as_str(), text.as_str(), color);
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
        if id >= IDM_TDP_START && id < IDM_TDP_START + self.tdp_icon_menu_commands.len() as u32 {
            self.tdp_icon_menu_commands
                .get((id - IDM_TDP_START) as usize)
                .cloned()
        } else if id >= IDM_CHARGE_START
            && id < IDM_CHARGE_START + self.charge_icon_menu_commands.len() as u32
        {
            self.charge_icon_menu_commands
                .get((id - IDM_CHARGE_START) as usize)
                .cloned()
        } else {
            None
        }
    }

    fn add_tdp_command(&mut self, command: Command) -> u32 {
        let id = IDM_TDP_START + self.tdp_icon_menu_commands.len() as u32;
        self.tdp_icon_menu_commands.push(command);
        id
    }

    fn add_charge_command(&mut self, command: Command) -> u32 {
        let id = IDM_CHARGE_START + self.charge_icon_menu_commands.len() as u32;
        self.charge_icon_menu_commands.push(command);
        id
    }

    fn update_tdp_menu(&mut self, old_model: &Option<TdpModel>, model: &TdpModel) -> bool {
        if let Some(old_model) = old_model {
            if old_model.options == model.options && old_model.applications == model.applications {
                // Nothing to update
                return false;
            }
        }
        // TODO: Update the existing menu instead of building a new one from scratch
        self.tdp_icon_menu_commands.clear();
        let mut menu = PopupMenu::new();
        if model.applications.len() > 0 {
            for app in &model.applications {
                let mut app_menu = PopupMenu::new();
                let id = self.add_tdp_command(Command::ResetApplicationTdp(app.clone()));
                app_menu.append_menu_item("Default", id);
                for tdp in &model.options {
                    let id = self.add_tdp_command(Command::SetApplicationTdp(app.clone(), *tdp));
                    app_menu.append_menu_item(&format!("{} W", (*tdp as f32) / 1000.0), id);
                }
                let path = Path::new(app);
                let file_name = path
                    .file_name()
                    .unwrap_or(app)
                    .to_str()
                    .unwrap_or("<UNKNOWN>");
                menu.append_submenu(file_name, app_menu);
            }
            menu.append_separator();
        }
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
        true
    }

    fn update_tdp_selection(&mut self, old_model: &Model, model: &Model, menu_rebuilt: bool) {
        if model.settings == old_model.settings && !menu_rebuilt {
            return;
        }
        let Some(menu) = &mut self.tdp_icon_popup_menu else {
            return;
        };
        for (i, cmd) in self.tdp_icon_menu_commands.iter().enumerate() {
            let id = i as u32 + IDM_TDP_START;
            let checked = match cmd {
                Command::Observe => model.settings.get_tdp_setting() == TdpSetting::Tracking,
                Command::ResetApplicationTdp(app) => model.settings.get_app_limit(app).is_none(),
                Command::SetApplicationTdp(app, limit) => {
                    model.settings.get_app_limit(app) == Some(*limit)
                }
                Command::SetTdp(target) => {
                    model.settings.get_tdp_setting() == TdpSetting::Forcing(*target)
                }
                Command::Exit => continue,
            };
            menu.check_menu_item(id, checked);
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

    fn build_charge_icon_menu(&mut self) {
        if self.charge_icon_popup_menu.is_some() {
            return;
        }
        self.charge_icon_menu_commands.clear();
        let mut menu = PopupMenu::new();
        let id = self.add_charge_command(Command::Exit);
        menu.append_menu_item("E&xit", id);
        self.charge_icon_popup_menu = Some(menu);
    }
}
