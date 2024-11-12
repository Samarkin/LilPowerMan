use std::collections::HashMap;

#[derive(Copy, Clone, PartialEq)]
pub enum TdpStateFallback {
    TrackingUnknown,
    Tracking(u32),
    Forcing(u32),
}

#[derive(Copy, Clone, PartialEq)]
pub enum TdpState {
    Tracking,
    Forcing(u32),
    ForcingApplication {
        app_limit: u32,
        fallback: TdpStateFallback,
    },
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

#[derive(Clone)]
pub struct Settings {
    app_limits: HashMap<String, u32>,
    version: usize,
}

impl Settings {
    pub fn get_app_limits(&self) -> &HashMap<String, u32> {
        &self.app_limits
    }

    pub fn get_app_limits_mut(&mut self) -> &mut HashMap<String, u32> {
        self.version += 1;
        &mut self.app_limits
    }
}

impl PartialEq for Settings {
    fn eq(&self, other: &Self) -> bool {
        if self.version == other.version {
            return true;
        }
        self.app_limits == other.app_limits
    }
}

/// Model defines the current state of the application.
#[derive(Clone, PartialEq)]
pub struct Model {
    pub tdp: Option<TdpModel>,
    pub charge_icon: Option<Result<i32, String>>,
    pub popup_menu: Option<PopupMenuModel>,
    pub settings: Settings,
}

impl Model {
    pub fn new() -> Self {
        Model {
            tdp: None,
            charge_icon: None,
            popup_menu: None,
            settings: Settings {
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
                version: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derived_eq() {
        assert!(TdpState::Tracking == TdpState::Tracking);
        assert!(TdpState::Forcing(10) == TdpState::Forcing(10));

        assert!(TdpState::Tracking != TdpState::Forcing(10));
        assert!(TdpState::Forcing(10) != TdpState::Forcing(20));
    }
}
