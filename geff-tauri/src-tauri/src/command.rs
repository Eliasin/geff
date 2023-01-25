use crate::app::{AppCommand, AppState, FrontendAppState};
use crate::parser::{self, GoalCommand};
use crate::parser::{command as parse_command, ControlCommand};
use geff_core::request::GoalRequest;
use geff_util::{get_selected_goal_id, Cursor, CursorAction};
use nom::Finish;
use std::ops::DerefMut;
use tauri::async_runtime::Mutex;

#[tauri::command]
pub async fn load(state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut state = state.lock().await;

    state
        .handle_command(AppCommand::LoadRequest)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Option<FrontendAppState>, String> {
    let app_state = state.lock().await;

    app_state.try_into_frontend()
}

#[tauri::command]
pub async fn cursor_action(
    state: tauri::State<'_, Mutex<AppState>>,
    cursor_action: CursorAction,
) -> Result<(), String> {
    let mut app_state = state.lock().await;

    app_state
        .handle_command(AppCommand::CursorAction(cursor_action))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

async fn handle_goal_command(app_state: &mut AppState, command: GoalCommand) -> anyhow::Result<()> {
    let selected_goal_id = if let AppState::Loaded {
        persistent_state: _,
        cursor,
        populated_goals,
        current_datetime: _,
    } = &mut *app_state
    {
        if let Cursor::SelectedGoal(Some(selected_goal)) = cursor {
            Some(get_selected_goal_id(selected_goal, populated_goals)?)
        } else {
            None
        }
    } else {
        None
    };

    match command {
        parser::GoalCommand::Create {
            name,
            effort_to_complete,
        } => {
            app_state
                .handle_command(AppCommand::GoalRequest(GoalRequest::Add {
                    name,
                    effort_to_complete,
                }))
                .await
        }
        parser::GoalCommand::Delete => {
            if let Some(selected_goal_id) = selected_goal_id {
                app_state
                    .handle_command(AppCommand::GoalRequest(GoalRequest::Delete(
                        selected_goal_id,
                    )))
                    .await
            } else {
                Ok(())
            }
        }
        parser::GoalCommand::Refine {
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
            } else {
                Ok(())
            }
        }
        parser::GoalCommand::AddEffort { effort } => {
            if let Some(selected_goal_id) = selected_goal_id {
                app_state
                    .handle_command(AppCommand::GoalRequest(GoalRequest::AddEffort {
                        goal_id: selected_goal_id,
                        effort,
                    }))
                    .await
            } else {
                Ok(())
            }
        }
        parser::GoalCommand::RemoveEffort { effort } => {
            if let Some(selected_goal_id) = selected_goal_id {
                app_state
                    .handle_command(AppCommand::GoalRequest(GoalRequest::RemoveEffort {
                        goal_id: selected_goal_id,
                        effort,
                    }))
                    .await
            } else {
                Ok(())
            }
        }
    }
}

#[tauri::command]
pub async fn app_command(
    state: tauri::State<'_, Mutex<AppState>>,
    handle: tauri::AppHandle,
    command: String,
) -> Result<(), String> {
    let mut app_state = state.lock().await;

    let (_, command) = parse_command(&command)
        .finish()
        .map_err(|e| format!("Failed to parse command: {e}"))?;

    match command {
        parser::Command::Display(command) => app_state
            .handle_command(AppCommand::DisplayCommand(command))
            .await
            .map_err(|e| e.to_string()),
        parser::Command::Goal(command) => handle_goal_command(app_state.deref_mut(), command)
            .await
            .map_err(|e| e.to_string()),
        parser::Command::Control(control_command) => match control_command {
            ControlCommand::Quit => {
                handle.exit(0);

                Ok(())
            }
        },
    }
}
