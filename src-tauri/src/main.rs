//! Desktop binary entry: delegates to the library `run()`.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    git_pincer_desktop_lib::run()
}
