#[repr(usize)]
pub enum Timer {
    Main,
}

#[repr(u32)]
pub enum MenuItem {
    Exit = 1,
    Observe = 2,
    SetTdpBegin = 256,
}

#[repr(u32)]
pub enum NotifyIcon {
    TdpLimit,
    ChargeRate,
}
