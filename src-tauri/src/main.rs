// Prevents an additional console window on Windows in release builds, do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    omoio_lib::run();
}
