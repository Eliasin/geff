#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
mod command;
mod parser;

use app::AppState;
use command::invoke_handler;
use tauri::async_runtime::Mutex;

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(AppState::Unloaded))
        .invoke_handler(invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
