#![windows_subsystem = "windows"]

mod config;
mod hotkeys;
mod image_processing;
mod windows_app;

fn main() {
    windows_app::run();
}
