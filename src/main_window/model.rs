use crate::versioned::Versioned;
use std::collections::HashMap;

#[derive(Copy, Clone, PartialEq)]
pub enum TdpState {
    Tracking,
    Forcing,
    ForcingApplication { fallback: Option<u32> },
}

#[derive(Clone, PartialEq)]
pub struct TdpModel {
    pub value: Result<u32, String>,
    pub state: TdpState,
    pub menu_items: Vec<u32>,
}

#[derive(Clone, PartialEq)]
pub enum PopupMenuType {
    TdpIcon,
}

#[derive(Clone, PartialEq)]
pub struct PopupMenuModel {
    pub x: i32,
    pub y: i32,
    pub menu: PopupMenuType,
}

#[derive(Copy, Clone, PartialEq)]
pub enum TdpSetting {
    Tracking,
    Forcing(u32),
}

#[derive(Clone, PartialEq)]
pub struct Settings {
    pub app_limits: HashMap<String, u32>,
    pub tdp: TdpSetting,
}

/// Model defines the current state of the application.
#[derive(Clone, PartialEq)]
pub struct Model {
    pub tdp: Option<TdpModel>,
    pub charge_icon: Option<Result<i32, String>>,
    pub popup_menu: Option<PopupMenuModel>,
    pub settings: Versioned<Settings>,
}

impl Model {
    pub fn new() -> Self {
        Model {
            tdp: None,
            charge_icon: None,
            popup_menu: None,
            settings: Versioned::new(Settings {
                app_limits: HashMap::from([
                    (
                        "c:\\program files\\jetbrains\\rustrover 2024.2.2\\bin\\rustrover64.exe"
                            .to_string(),
                        10000,
                    ),
                    (
                        "c:\\games\\steam\\steamapps\\common\\red dead redemption\\rdr.exe"
                            .to_string(),
                        20000,
                    ),
                ]),
                tdp: TdpSetting::Tracking,
            }),
        }
    }
}
