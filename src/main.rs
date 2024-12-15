#![windows_subsystem = "windows"]

#[macro_use]
extern crate log;

mod battery;
mod gdip;
mod icons;
mod main_window;
mod menu;
mod ryzenadj;
mod settings;
mod singleton;
mod versioned;
mod winapi;

use gdip::GdiPlus;
use main_window::MainWindow;
use singleton::Singleton;
use std::panic;
use winapi::show_error_message_box;
use winapi::windows_message_loop;

fn main() {
    info!("Application startup");
    panic::set_hook(Box::new(|panic_info| {
        error!("{}", panic_info);
        // FIXME: This kicks off a nested message loop, which is likely to repeat the panic
        show_error_message_box(panic_info.to_string().as_str());
    }));
    if !Singleton::is_first_instance() {
        info!("Another instance found. Shutting down");
        show_error_message_box("The application is already running on this computer");
        return;
    }
    let gdi_plus = GdiPlus::new();
    let _window = MainWindow::new(&gdi_plus);
    windows_message_loop();
    info!("Graceful shutdown");
}
