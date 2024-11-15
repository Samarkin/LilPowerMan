#[derive(Copy, Clone, PartialEq)]
pub enum Command {
    Observe,
    SetTdp(u32),
    Exit,
}
