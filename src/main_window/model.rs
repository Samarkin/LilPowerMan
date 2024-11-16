use crate::versioned::Versioned;
use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;

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
    pub applications: VecDeque<OsString>,
    pub options: Vec<u32>,
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
    pub app_limits: HashMap<OsString, u32>,
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
                        OsString::from("c:\\program files\\jetbrains\\rustrover 2024.2.2\\bin\\rustrover64.exe"),
                        10000,
                    ),
                    (
                        OsString::from("c:\\games\\steam\\steamapps\\common\\red dead redemption\\rdr.exe"),
                        20000,
                    ),
                ]),
                tdp: TdpSetting::Tracking,
            }),
        }
    }
}
