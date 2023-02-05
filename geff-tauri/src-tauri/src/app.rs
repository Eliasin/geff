use geff_core::goal::{GoalId, PopulatedGoal};
use geff_core::request::{GoalRequest, GoalRequestHandler};
use geff_core::{DateTime, Utc};
use geff_util::{get_selected_goal_id, Cursor, CursorAction, PersistentState};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CommandlineDisplayConfig {
    #[serde(rename = "fontSizePixels")]
    font_size_pixels: u32,
    #[serde(rename = "backgroundColor")]
    background_color: String,
    #[serde(rename = "fontColor")]
    font_color: String,
}

impl Default for CommandlineDisplayConfig {
    fn default() -> Self {
        Self {
            font_size_pixels: 14,
            background_color: "gray".to_string(),
            font_color: "black".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CommandlineDisplayCommand {
    ChangeFontSize(u32),
    ChangeBackgroundColor(String),
    ChangeFontColor(String),
}

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    Commandline(CommandlineDisplayCommand),
}

impl From<CommandlineDisplayCommand> for DisplayCommand {
    fn from(value: CommandlineDisplayCommand) -> Self {
        DisplayCommand::Commandline(value)
    }
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct DisplayConfig {
    commandline: CommandlineDisplayConfig,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Config {
    display: DisplayConfig,
}

#[derive(Clone, Debug)]
pub enum AppCommand {
    GoalRequest(GoalRequest),
    CursorAction(CursorAction),
    DisplayCommand(DisplayCommand),
    LoadRequest,
    SaveRequest,
}

pub enum AppState {
    Loaded {
        persistent_state: PersistentState<Config>,
        cursor: Cursor,
        populated_goals: Vec<PopulatedGoal>,
        current_datetime: DateTime<Utc>,
    },
    Unloaded,
    Error(String),
}

#[derive(Deserialize, Serialize, Clone)]
pub struct FrontendAppState {
    #[serde(rename = "populatedGoals")]
    pub populated_goals: Vec<PopulatedGoal>,
    #[serde(rename = "selectedGoalId")]
    pub selected_goal_id: Option<GoalId>,
    #[serde(rename = "focusedGoals")]
    pub focused_goals: HashSet<GoalId>,
    pub config: Config,
}

impl AppState {
    pub fn try_into_frontend(&self) -> Result<Option<FrontendAppState>, String> {
        let selected_goal_id = if let AppState::Loaded {
            persistent_state: _,
            cursor: Cursor::SelectedGoal(Some(selected_goal)),
            populated_goals,
            current_datetime: _,
        } = self
        {
            Some(get_selected_goal_id(selected_goal, populated_goals).map_err(|e| e.to_string())?)
        } else {
            None
        };

        Ok(
            if let AppState::Loaded {
                persistent_state,
                cursor: _,
                populated_goals,
                current_datetime: _,
            } = self
            {
                Some(FrontendAppState {
                    populated_goals: populated_goals.clone(),
                    selected_goal_id,
                    focused_goals: persistent_state.profile.focused_goals().clone(),
                    config: persistent_state.config.clone(),
                })
            } else if let AppState::Error(e) = self {
                Err(e.to_string())?
            } else {
                None
            },
        )
    }

    pub async fn handle_command(&mut self, command: AppCommand) -> anyhow::Result<()> {
        if let AppState::Loaded {
            persistent_state,
            cursor,
            populated_goals,
            current_datetime,
        } = self
        {
            match command {
                AppCommand::LoadRequest => {
                    let config_data_path = match PersistentState::<Config>::data_path("geff-tauri")
                    {
                        Ok(config_data_path) => config_data_path,
                        Err(e) => {
                            *self = AppState::Error(e.to_string());
                            return Ok(());
                        }
                    };
                    let persistent_state =
                        match PersistentState::<Config>::load(config_data_path).await {
                            Ok(persistent_state) => persistent_state,
                            Err(e) => {
                                *self = AppState::Error(e.to_string());
                                return Ok(());
                            }
                        };
                    let populated_goals = persistent_state.profile.populate_goals();

                    *self = AppState::Loaded {
                        persistent_state,
                        cursor: Default::default(),
                        populated_goals,
                        current_datetime: Utc::now(),
                    };
                }
                AppCommand::GoalRequest(goal_request) => {
                    persistent_state
                        .profile
                        .with_datetime(*current_datetime)
                        .handle_request(goal_request);
                    *populated_goals = persistent_state.profile.populate_goals();
                }
                AppCommand::CursorAction(cursor_action) => {
                    cursor.handle_action(cursor_action, populated_goals)?;
                }
                AppCommand::DisplayCommand(DisplayCommand::Commandline(command)) => {
                    let CommandlineDisplayConfig {
                        font_size_pixels,
                        background_color,
                        font_color,
                    } = &mut persistent_state.config.display.commandline;

                    match command {
                        CommandlineDisplayCommand::ChangeFontSize(fs) => *font_size_pixels = fs,
                        CommandlineDisplayCommand::ChangeBackgroundColor(color) => {
                            *background_color = color
                        }
                        CommandlineDisplayCommand::ChangeFontColor(color) => *font_color = color,
                    }
                }
                AppCommand::SaveRequest => {
                    let config_data_path = match PersistentState::<Config>::data_path("geff-tauri")
                    {
                        Ok(config_data_path) => config_data_path,
                        Err(e) => {
                            *self = AppState::Error(e.to_string());
                            return Ok(());
                        }
                    };
                    match persistent_state.save_to_file(config_data_path).await {
                        Ok(config_data_path) => config_data_path,
                        Err(e) => {
                            *self = AppState::Error(e.to_string());
                            return Ok(());
                        }
                    };
                }
            };

            Ok(())
        } else {
            if let AppCommand::LoadRequest = command {
                let config_data_path = match PersistentState::<Config>::data_path("geff-tauri") {
                    Ok(config_data_path) => config_data_path,
                    Err(e) => {
                        *self = AppState::Error(e.to_string());
                        return Ok(());
                    }
                };
                let persistent_state = match PersistentState::<Config>::load(config_data_path).await
                {
                    Ok(persistent_state) => persistent_state,
                    Err(e) => {
                        *self = AppState::Error(e.to_string());
                        return Ok(());
                    }
                };
                let populated_goals = persistent_state.profile.populate_goals();

                *self = AppState::Loaded {
                    persistent_state,
                    cursor: Default::default(),
                    populated_goals,
                    current_datetime: Utc::now(),
                };
            }

            Ok(())
        }
    }
}
