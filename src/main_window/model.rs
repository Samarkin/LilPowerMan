#[derive(Copy, Clone, PartialEq)]
pub enum TdpIconColor {
    Tracking,
    Forcing,
}

#[derive(Clone, PartialEq)]
pub struct TdpIconModel {
    pub value: Result<u32, String>,
    pub color: TdpIconColor,
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

/// Model defines the visual state of the application.
#[derive(Clone, PartialEq)]
pub struct Model {
    pub tdp_icon: Option<TdpIconModel>,
    pub charge_icon: Option<Result<i32, String>>,
    pub popup_menu: Option<PopupMenuModel>,
}

impl Model {
    pub fn new() -> Self {
        Model {
            tdp_icon: None,
            charge_icon: None,
            popup_menu: None,
        }
    }
}
