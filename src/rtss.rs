mod bindings;
mod shared_memory;

use crate::battery::BatteryStatus;
use crate::rtss::shared_memory::SharedMemoryBuilder;
use shared_memory::{open_shared_memory, SharedMemoryView};
use std::fmt::{Debug, Display, Formatter};
use windows::core::Error as WindowsError;

pub struct Rtss {
    ever_updated: bool,
}

pub enum Error {
    RtssV2NotRunning,
    RtssVersionNotSupported(String),
    UnexpectedMemoryLayout,
    NoEmptyOsdSlots,
    EntryOverflow,
    WindowsError(WindowsError),
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RtssV2NotRunning => write!(f, "RTSS is not running"),
            Self::RtssVersionNotSupported(v) => write!(f, "RTSS version is not supported: {v}"),
            Self::UnexpectedMemoryLayout => {
                write!(f, "RTSS shared memory layout does not match expectations")
            }
            Self::NoEmptyOsdSlots => write!(f, "All RTSS OSD slots are occupied"),
            Self::EntryOverflow => write!(f, "Entry does not fit in RTSS-allocated buffer"),
            Self::WindowsError(inner) => write!(f, "Unexpected WinAPI error: {inner}"),
        }
    }
}

impl Rtss {
    pub fn new() -> Rtss {
        Rtss {
            ever_updated: false,
        }
    }

    pub fn update(&mut self, battery: &BatteryStatus) -> Result<(), Error> {
        let mem = open_shared_memory()?;
        let mut view = SharedMemoryView::from_file(&mem)?;
        let mut builder = SharedMemoryBuilder::new();
        builder.add_text(&format!(
            "{}.{:03}<S=50>W<S>",
            battery.charge_rate / 1000,
            (battery.charge_rate % 1000).abs()
        ));
        if battery.charge_rate < 0 {
            // draining
            let mins = (-60.0 * (battery.capacity as f64 / battery.charge_rate as f64)) as i64;
            builder.add_text(&format!(" {mins}<S=50>mins<S>"));
        } else {
            builder.add_text(" (on charger)");
        }
        builder
            .add_newline()
            .add_text("<FR><S=50>FPS<S>")
            .write(&mut view)?;
        self.ever_updated = true;
        Ok(())
    }

    fn unregister(&mut self) -> Result<(), Error> {
        let mem = open_shared_memory()?;
        let mut view = SharedMemoryView::from_file(&mem)?;
        view.unregister()
    }
}

impl Drop for Rtss {
    fn drop(&mut self) {
        if self.ever_updated {
            match self.unregister() {
                Ok(()) => {}
                Err(Error::RtssV2NotRunning) => {}
                Err(err) => {
                    error!("Failed to unregister from the RTSS shared memory: {err}");
                }
            }
        }
    }
}
