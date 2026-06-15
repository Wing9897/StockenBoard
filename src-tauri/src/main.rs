// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(feature = "desktop")]
    stockenboard_lib::run();

    #[cfg(not(feature = "desktop"))]
    {
        eprintln!("The desktop binary requires the 'desktop' feature. Use --features desktop or build the server binary instead.");
        std::process::exit(1);
    }
}
