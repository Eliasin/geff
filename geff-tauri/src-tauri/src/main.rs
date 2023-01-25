#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
mod command;
mod parser;

use app::AppState;
use command::{app_command, cursor_action, fetch, load};
use tauri::async_runtime::Mutex;

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(AppState::Unloaded))
        .invoke_handler(tauri::generate_handler![
            app_command,
            load,
            fetch,
            cursor_action
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
