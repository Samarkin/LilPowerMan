#![windows_subsystem = "windows"]

#[macro_use]
extern crate log;

mod battery;
mod gdip;
mod icons;
mod logging;
mod main_window;
mod menu;
mod ryzenadj;
mod settings;
mod singleton;
mod versioned;
mod winapi;

use gdip::GdiPlus;
use log::{LevelFilter, Log};
use logging::FileLogger;
use main_window::MainWindow;
use singleton::Singleton;
use std::panic;
use winapi::show_error_message_box;
use winapi::windows_message_loop;

fn main() {
    let logger = FileLogger::new();
    // SAFETY: This is the first time we set a logger
    log::set_boxed_logger(Box::new(logger)).unwrap();
    let last_arg = std::env::args().last().unwrap_or_else(|| String::from(""));
    if last_arg.eq_ignore_ascii_case("/debug") {
        log::set_max_level(LevelFilter::Debug);
    } else if last_arg.eq_ignore_ascii_case("/trace") {
        log::set_max_level(LevelFilter::Trace);
    } else {
        log::set_max_level(LevelFilter::Info);
    }

    info!("Application startup");
    panic::set_hook(Box::new(|panic_info| {
        error!("{}", panic_info);
        // FIXME: This kicks off a nested message loop, which is likely to repeat the panic
        show_error_message_box(panic_info.to_string().as_str());
    }));
    // SAFETY: We are sure that current logger is indeed a FileLogger
    let logger = unsafe { &*(log::logger() as *const dyn Log as *const FileLogger) };
    logger.init(&std::env::temp_dir()).unwrap();
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
