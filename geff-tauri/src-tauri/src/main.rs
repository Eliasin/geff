#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
mod parser;

use app::{AppCommand, AppState, FrontendAppState};
use geff_core::request::GoalRequest;
use geff_util::{get_selected_goal_id, Cursor};
use nom::Finish;
use parser::command as parse_command;
use tauri::async_runtime::Mutex;

#[tauri::command]
async fn load(state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut state = state.lock().await;

    state
        .handle_command(AppCommand::LoadRequest)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn fetch(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Option<FrontendAppState>, String> {
    let app_state = state.lock().await;

    app_state.try_into_frontend()
}

// The `Result` in the return is needed to work around a problem in tauri
#[tauri::command]
async fn app_command(
    state: tauri::State<'_, Mutex<AppState>>,
    command: String,
) -> Result<(), String> {
    let mut app_state = state.lock().await;

    let (_, command) = parse_command(&command)
        .finish()
        .map_err(|e| format!("Failed to parse command: {e}"))?;

    let selected_goal_id = if let AppState::Loaded {
        persistent_state: _,
        cursor,
        populated_goals,
        current_datetime: _,
    } = &mut *app_state
    {
        if let Cursor::SelectedGoal(Some(selected_goal)) = cursor {
            Some(get_selected_goal_id(selected_goal, populated_goals).map_err(|e| e.to_string())?)
        } else {
            None
        }
    } else {
        None
    };

    match command {
        parser::Command::Create {
            name,
            effort_to_complete,
        } => {
            app_state
                .handle_command(AppCommand::GoalRequest(GoalRequest::Add {
                    name,
                    effort_to_complete,
                }))
                .await
                .map_err(|e| e.to_string())?;
        }
        parser::Command::Delete => {
            if let Some(selected_goal_id) = selected_goal_id {
                app_state
                    .handle_command(AppCommand::GoalRequest(GoalRequest::Delete(
                        selected_goal_id,
                    )))
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        parser::Command::Refine {
            child_name,
            child_effort_to_complete,
            parent_effort_removed,
        } => {
            if let Some(selected_goal_id) = selected_goal_id {
                app_state
                    .handle_command(AppCommand::GoalRequest(GoalRequest::Refine {
                        parent_goal_id: selected_goal_id,
                        parent_effort_removed,
                        child_name,
                        child_effort_to_complete,
                    }))
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        parser::Command::AddEffort { effort } => {
            if let Some(selected_goal_id) = selected_goal_id {
                app_state
                    .handle_command(AppCommand::GoalRequest(GoalRequest::AddEffort {
                        goal_id: selected_goal_id,
                        effort,
                    }))
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        parser::Command::RemoveEffort { effort } => {
            if let Some(selected_goal_id) = selected_goal_id {
                app_state
                    .handle_command(AppCommand::GoalRequest(GoalRequest::RemoveEffort {
                        goal_id: selected_goal_id,
                        effort,
                    }))
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    };

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(AppState::Unloaded))
        .invoke_handler(tauri::generate_handler![app_command, load, fetch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
