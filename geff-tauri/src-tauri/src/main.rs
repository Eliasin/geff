#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use geff_util::{LoadError, PersistentState};

#[tauri::command]
async fn load_local() -> Result<PersistentState<()>, LoadError> {
    PersistentState::<()>::load("geff-tauri").await
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![load_local])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
