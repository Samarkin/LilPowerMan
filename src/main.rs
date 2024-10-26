#![windows_subsystem = "windows"]

mod battery;
mod main_window;
mod ryzenadj;
mod winapi;

use crate::main_window::MainWindow;
use winapi::windows_message_loop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _window = MainWindow::new()?;
    windows_message_loop()?;
    Ok(())
}
