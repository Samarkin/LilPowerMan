use super::id;
use super::model::{Model, PopupMenuModel, PopupMenuType, TdpModel, TdpState};
use crate::battery::{BatteriesIterator, Battery};
use crate::ryzenadj::RyzenAdj;
use crate::winapi::show_error_message_box;
use std::mem::take;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;

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

    fn get_tdp_menu_items(&self) -> Vec<u32> {
        // TODO: Determine based on chip's max TDP
        vec![5, 7, 10, 15, 20, 24, 28]
    }

    pub fn on_timer(&mut self) {
        self.model.tdp = match self.get_tdp_limit() {
            Some(mut value) => {
                let (menu_items, state) = take(&mut self.model.tdp)
                    .map(|m| (m.menu_items, m.state))
                    .unwrap_or_else(|| (self.get_tdp_menu_items(), TdpState::Tracking));
                if let TdpState::Forcing(target) = state {
                    if let Ok(current) = value {
                        if current != target {
                            if let Some(ryzen_adj) = &mut self.ryzen_adj {
                                match ryzen_adj.set_all_limits(target) {
                                    Ok(()) => value = Ok(target),
                                    Err(err) => value = Err(err.to_string()),
                                }
                            }
                        }
                    }
                }
                Some(TdpModel {
                    value,
                    menu_items,
                    state,
                })
            }
            None => None,
        };
        self.model.charge_icon = self.get_charge_rate();
    }

    pub fn on_menu_item_click(&mut self, id: u32) {
        if id == id::MenuItem::Observe as _ {
            if let Some(tdp) = &mut self.model.tdp {
                tdp.state = TdpState::Tracking;
            }
        } else if id == id::MenuItem::Exit as _ {
            // SAFETY: It is sound to destroy the window we own
            unsafe { DestroyWindow(self.window).unwrap() };
        } else if id > id::MenuItem::SetTdpBegin as _ {
            let target = id - id::MenuItem::SetTdpBegin as u32;
            if let Some(tdp) = &mut self.model.tdp {
                tdp.state = TdpState::Forcing(target * 1000);
            }
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
