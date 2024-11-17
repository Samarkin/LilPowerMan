use crate::settings::{Settings, SettingsStorage};
use crate::versioned::Versioned;
use std::collections::VecDeque;
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

/// Model defines the current state of the application.
#[derive(Clone, Default, PartialEq)]
pub struct Model {
    pub tdp: Option<TdpModel>,
    pub charge_icon: Option<Result<i32, String>>,
    pub popup_menu: Option<PopupMenuModel>,
    pub settings: Versioned<Settings>,
}

impl Model {
    pub fn new(settings_storage: &SettingsStorage) -> Self {
        Model {
            tdp: None,
            charge_icon: None,
            popup_menu: None,
            settings: Versioned::new(settings_storage.load()),
        }
    }
}
