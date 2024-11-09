#![windows_subsystem = "windows"]

mod battery;
mod gdip;
mod icons;
mod main_window;
mod menu;
mod ryzenadj;
mod winapi;

use gdip::GdiPlus;
use main_window::MainWindow;
use std::panic;
use winapi::show_error_message_box;
use winapi::windows_message_loop;

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        // FIXME: This kicks off a nested message loop, which is likely to repeat the panic
        show_error_message_box(panic_info.to_string().as_str());
    }));
    let gdi_plus = GdiPlus::new();
    let _window = MainWindow::new(&gdi_plus);
    windows_message_loop();
}
