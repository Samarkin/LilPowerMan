#![windows_subsystem = "windows"]

mod battery;
mod icons;
mod main_window;
mod menu;
mod ryzenadj;
mod winapi;

use battery::BatteriesIterator;
use main_window::MainWindow;
use ryzenadj::RyzenAdj;
use std::panic;
use winapi::show_error_message_box;
use winapi::windows_message_loop;

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        show_error_message_box(panic_info.to_string().as_str());
    }));
    let ryzen_adj = RyzenAdj::new().map_or_else(
        |err| {
            show_error_message_box(format!("Failed to initialize RyzenAdj: {}", err).as_str());
            None
        },
        Some,
    );
    let battery = BatteriesIterator::new().next().and_then(|r| {
        r.map_or_else(
            |err| {
                show_error_message_box(format!("Failed to get battery info: {}", err).as_str());
                None
            },
            Some,
        )
    });
    let _window = MainWindow::new(ryzen_adj, battery);
    windows_message_loop();
}
