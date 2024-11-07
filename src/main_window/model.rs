#[derive(Copy, Clone, PartialEq)]
pub enum TdpState {
    Tracking,
    Forcing(u32),
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

/// Model defines the current state of the application.
#[derive(Clone, PartialEq)]
pub struct Model {
    pub tdp: Option<TdpModel>,
    pub charge_icon: Option<Result<i32, String>>,
    pub popup_menu: Option<PopupMenuModel>,
}

impl Model {
    pub fn new() -> Self {
        Model {
            tdp: None,
            charge_icon: None,
            popup_menu: None,
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
