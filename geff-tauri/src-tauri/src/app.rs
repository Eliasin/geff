use geff_core::goal::{GoalId, PopulatedGoal};
use geff_core::request::{GoalRequest, GoalRequestHandler};
use geff_core::{DateTime, Utc};
use geff_util::{get_selected_goal_id, Cursor, CursorAction, PersistentState};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Deserialize, Serialize, Clone)]
pub enum AppCommand {
    GoalRequest(GoalRequest),
    CursorAction(CursorAction),
    LoadRequest,
}

pub enum AppState {
    Loaded {
        persistent_state: PersistentState<()>,
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
}

impl AppState {
    pub fn try_into_frontend(&self) -> Result<Option<FrontendAppState>, String> {
        let selected_goal_id = if let AppState::Loaded {
            persistent_state: _,
            cursor,
            populated_goals,
            current_datetime: _,
        } = self
        {
            if let Cursor::SelectedGoal(Some(selected_goal)) = cursor {
                Some(
                    get_selected_goal_id(selected_goal, populated_goals)
                        .map_err(|e| e.to_string())?,
                )
            } else {
                None
            }
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
                })
            } else if let AppState::Error(e) = self {
                Err(e.to_string())?
            } else {
                None
            },
        )
    }

    pub async fn handle_command(&mut self, command: AppCommand) -> anyhow::Result<()> {
        use AppCommand::*;

        match command {
            LoadRequest => {
                let persistent_state = match PersistentState::<()>::load("geff-tauri").await {
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

                Ok(())
            }
            GoalRequest(goal_request) => {
                if let AppState::Loaded {
                    persistent_state,
                    cursor: _,
                    populated_goals,
                    current_datetime,
                } = self
                {
                    persistent_state
                        .profile
                        .with_datetime(*current_datetime)
                        .handle_request(goal_request);
                    *populated_goals = persistent_state.profile.populate_goals();
                }

                Ok(())
            }
            CursorAction(cursor_action) => {
                if let AppState::Loaded {
                    cursor,
                    populated_goals,
                    ..
                } = self
                {
                    cursor.handle_action(cursor_action, populated_goals)?;
                }

                Ok(())
            }
        }
    }
}
