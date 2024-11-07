#![windows_subsystem = "windows"]

mod battery;
mod icons;
mod main_window;
mod menu;
mod ryzenadj;
mod winapi;

use main_window::MainWindow;
use std::panic;
use winapi::show_error_message_box;
use winapi::windows_message_loop;

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        show_error_message_box(panic_info.to_string().as_str());
    }));
    let _window = MainWindow::new();
    windows_message_loop();
}
