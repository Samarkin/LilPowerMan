#[repr(usize)]
pub enum Timer {
    Main,
}

#[repr(u32)]
pub enum MenuItem {
    HelloWorld = 123,
    Exit = 1,
}

#[repr(u32)]
pub enum NotifyIcon {
    TdpLimit,
    ChargeRate,
}
