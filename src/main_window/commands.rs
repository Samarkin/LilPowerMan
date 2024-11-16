use std::ffi::OsString;

#[derive(Clone, PartialEq)]
pub enum Command {
    Observe,
    ResetApplicationTdp(OsString),
    SetApplicationTdp(OsString, u32),
    SetTdp(u32),
    Exit,
}
