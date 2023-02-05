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

async fn handle_untargeted_goal_command(
    app_state: &mut AppState,
    command: GoalCommand,
) -> anyhow::Result<bool> {
    Ok(match command {
        parser::GoalCommand::Create {
            name,
            effort_to_complete,
        } => {
            app_state
                .handle_command(AppCommand::GoalRequest(GoalRequest::Create {
                    name,
                    effort_to_complete,
                }))
                .await?;

            true
        }
        _ => false,
    })
}

async fn handle_targeted_goal_command(
    app_state: &mut AppState,
    command: GoalCommand,
) -> anyhow::Result<bool> {
    let selected_goal_id = if let AppState::Loaded {
        persistent_state: _,
        cursor: Cursor::SelectedGoal(Some(selected_goal)),
        populated_goals,
        current_datetime: _,
    } = &mut *app_state
    {
        get_selected_goal_id(selected_goal, populated_goals)?
    } else {
        return Ok(false);
    };

    let command = match command {
        parser::GoalCommand::Create { .. } => return Ok(false),
        parser::GoalCommand::Delete => {
            AppCommand::GoalRequest(GoalRequest::Delete(selected_goal_id))
        }
        parser::GoalCommand::Refine {
            child_name,
            child_effort_to_complete,
            parent_effort_removed,
        } => AppCommand::GoalRequest(GoalRequest::Refine {
            parent_goal_id: selected_goal_id,
            parent_effort_removed,
            child_name,
            child_effort_to_complete,
        }),
        parser::GoalCommand::AddEffort { effort } => {
            AppCommand::GoalRequest(GoalRequest::AddEffort {
                goal_id: selected_goal_id,
                effort,
            })
        }
        parser::GoalCommand::RemoveEffort { effort } => {
            AppCommand::GoalRequest(GoalRequest::RemoveEffort {
                goal_id: selected_goal_id,
                effort,
            })
        }
        parser::GoalCommand::Focus => todo!(),
        parser::GoalCommand::Unfocus => todo!(),
        parser::GoalCommand::FocusSingle => todo!(),
        parser::GoalCommand::UnfocusSingle => todo!(),
        parser::GoalCommand::Rescope {
            new_effort_to_complete,
        } => AppCommand::GoalRequest(GoalRequest::Rescope {
            goal_id: selected_goal_id,
            new_effort_to_complete,
        }),
        parser::GoalCommand::Rename { new_name } => AppCommand::GoalRequest(GoalRequest::Rename {
            goal_id: selected_goal_id,
            new_name,
        }),
    };

    app_state.handle_command(command).await?;

    Ok(true)
}

async fn handle_goal_command(app_state: &mut AppState, command: GoalCommand) -> anyhow::Result<()> {
    if !handle_untargeted_goal_command(app_state, command.clone()).await? {
        // false return means action us unhandled and nothing was triggered
        handle_targeted_goal_command(app_state, command).await?;
    }

    Ok(())
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
            ControlCommand::Save => app_state
                .handle_command(AppCommand::SaveRequest)
                .await
                .map_err(|e| e.to_string()),
        },
    }
}
