use std::env::VarError;
use std::fmt::Debug;
use std::path::PathBuf;

use geff_core::goal::{GoalEvent, GoalId, PopulatedGoal};
use geff_core::profile::goal_traversal::{traverse_populated_goal_children, GoalChildIndexPath};
use geff_core::profile::Profile;

use serde::de::DeserializeOwned;

use serde::{Deserialize, Serialize};

#[allow(unused)]
#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize)]
pub enum LoadError {
    #[error("APP_DATA or $t HOME directory not found: {0}")]
    NoAppDataOrHomeDirectory(String),
    #[error("Failed to create profile data at {0}: {1}")]
    ProfileDataCreation(PathBuf, String),
    #[error("Failed to read profile data at {0}: {1}")]
    ProfileDataFileRead(PathBuf, String),
    #[error("Profile data at {0} is malformed: {1}")]
    MalformedProfileDataFile(PathBuf, String),
    #[error("Failed to write default data to new file at {0}: {1}")]
    FailureToWriteDefaultData(PathBuf, String),
}

impl From<VarError> for LoadError {
    fn from(value: VarError) -> Self {
        LoadError::NoAppDataOrHomeDirectory(value.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PersistentState<C>
where
    C: std::fmt::Debug + Serialize + Clone + Default,
{
    pub profile: Profile,
    pub goal_event_history: Vec<GoalEvent>,
    pub config: C,
}

impl<C: std::fmt::Debug + Serialize + Clone + Default + DeserializeOwned> From<PersistentState<C>>
    for (Profile, Vec<GoalEvent>, C)
{
    fn from(value: PersistentState<C>) -> Self {
        (value.profile, value.goal_event_history, value.config)
    }
}

#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(not(target_arch = "wasm32"))]
impl<C: std::fmt::Debug + Serialize + Clone + Default + DeserializeOwned> PersistentState<C> {
    #[cfg(target_os = "windows")]
    fn default_data_path<S: AsRef<str>>(app_name: S) -> Result<PathBuf, LoadError> {
        let appdata = PathBuf::from(std::env::var("APPDATA")?);

        Ok(app_data
            .join("Roaming")
            .join(app_name.as_ref())
            .join("data"))
    }

    #[cfg(target_os = "linux")]
    fn default_data_path<S: AsRef<str>>(app_name: S) -> Result<PathBuf, LoadError> {
        let home = PathBuf::from(std::env::var("HOME")?);
        Ok(home.join(format!(".{}", app_name.as_ref())))
    }

    #[cfg(target_os = "macos")]
    fn default_data_path<S: AsRef<str>>(app_name: S) -> Result<PathBuf, LoadError> {
        Ok(PathBuf::from(format!(
            "~Library/Application Support/{}/Data",
            app_name.as_ref()
        )))
    }

    pub async fn load<S: AsRef<str>>(app_name: S) -> Result<Self, LoadError> {
        use tokio::fs;

        let profile_data_path = std::env::var("GEFF_CORE_ICED_DATA_PATH")
            .map(PathBuf::from)
            .unwrap_or(Self::default_data_path(app_name)?);

        if !profile_data_path.exists() {
            fs::create_dir_all(
                profile_data_path
                    .parent()
                    .expect("profile data path to have parent"),
            )
            .await
            .map_err(|e| {
                LoadError::ProfileDataCreation(profile_data_path.clone(), e.to_string())
            })?;

            let default_data = rmp_serde::encode::to_vec(&Self::default())
                .expect("default data type to be serializable");

            fs::File::create(&profile_data_path)
                .await
                .map_err(|e| {
                    LoadError::ProfileDataCreation(profile_data_path.clone(), e.to_string())
                })?
                .write_all(&default_data)
                .await
                .map_err(|e| {
                    LoadError::FailureToWriteDefaultData(profile_data_path.clone(), e.to_string())
                })?;
        }

        let mut data_file = fs::File::open(profile_data_path.clone())
            .await
            .map_err(|e| {
                LoadError::ProfileDataFileRead(profile_data_path.clone(), e.to_string())
            })?;

        let mut profile_bytes = vec![];
        data_file
            .read_to_end(&mut profile_bytes)
            .await
            .map_err(|e| {
                LoadError::ProfileDataFileRead(profile_data_path.clone(), e.to_string())
            })?;

        rmp_serde::decode::from_slice(&profile_bytes).map_err(|e| {
            LoadError::MalformedProfileDataFile(profile_data_path.clone(), e.to_string())
        })
    }

    pub fn blocking_load<S: AsRef<str>>(app_name: S) -> Result<Self, LoadError> {
        use std::fs;

        let profile_data_path = std::env::var("GEFF_CORE_ICED_DATA_PATH")
            .map(PathBuf::from)
            .unwrap_or(Self::default_data_path(app_name)?);

        if !profile_data_path.exists() {
            fs::create_dir_all(
                profile_data_path
                    .parent()
                    .expect("profile data path to have parent"),
            )
            .map_err(|e| {
                LoadError::ProfileDataCreation(profile_data_path.clone(), e.to_string())
            })?;

            let default_data = rmp_serde::encode::to_vec(&Self::default())
                .expect("default data type to be serializable");

            fs::File::create(&profile_data_path)
                .map_err(|e| {
                    LoadError::ProfileDataCreation(profile_data_path.clone(), e.to_string())
                })?
                .write_all(&default_data)
                .map_err(|e| {
                    LoadError::FailureToWriteDefaultData(profile_data_path.clone(), e.to_string())
                })?;
        }

        let mut data_file = fs::File::open(profile_data_path.clone()).map_err(|e| {
            LoadError::ProfileDataFileRead(profile_data_path.clone(), e.to_string())
        })?;

        let mut profile_bytes = vec![];
        data_file.read_to_end(&mut profile_bytes).map_err(|e| {
            LoadError::ProfileDataFileRead(profile_data_path.clone(), e.to_string())
        })?;

        rmp_serde::decode::from_slice(&profile_bytes).map_err(|e| {
            LoadError::MalformedProfileDataFile(profile_data_path.clone(), e.to_string())
        })
    }
}

#[cfg(target_arch = "wasm32")]
impl<C: std::fmt::Debug + Serialize + Clone + Default + DeserializeOwned> PersistentState<C> {
    pub async fn load() -> Self {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedGoal {
    root_goal_index: usize,
    child_index_path: GoalChildIndexPath,
}

impl SelectedGoal {
    pub fn selected_index(&mut self) -> &mut usize {
        match self.child_index_path.last_mut() {
            Some(last_child_index) => last_child_index,
            None => &mut self.root_goal_index,
        }
    }

    pub fn pop_child(&mut self) -> Option<usize> {
        self.child_index_path.pop()
    }

    pub fn push_child(&mut self, index: usize) {
        self.child_index_path.push(index);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CursorError {
    #[error("root index of selected goal does not exist: {0:?}")]
    InvalidRootIndex(SelectedGoal),
    #[error("attempted to visit nonexistent child index {child_index} in goal {goal:?}")]
    InvalidGoalChild {
        goal: PopulatedGoal,
        child_index: usize,
    },
    #[error("error attempting to traverse to selected goal at {0:?}")]
    TraversalError(SelectedGoal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cursor {
    SelectedGoal(Option<SelectedGoal>),
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor::SelectedGoal(None)
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash, Debug)]
pub enum CursorAction {
    #[serde(rename = "up")]
    Up,
    #[serde(rename = "down")]
    Down,
    #[serde(rename = "in")]
    In,
    #[serde(rename = "out")]
    Out,
}

pub fn selected_goal_siblings<'a>(
    selected_goal: &SelectedGoal,
    goals: &'a Vec<PopulatedGoal>,
) -> Result<&'a Vec<PopulatedGoal>, CursorError> {
    if let Some((_last, before_last)) = selected_goal.child_index_path.split_last() {
        let mut current = goals
            .get(selected_goal.root_goal_index)
            .ok_or(CursorError::InvalidRootIndex(selected_goal.clone()))?;

        for index in before_last {
            current = current
                .children
                .get(*index)
                .ok_or(CursorError::InvalidGoalChild {
                    goal: current.clone(),
                    child_index: *index,
                })?;
        }

        Ok(&current.children)
    } else {
        Ok(goals)
    }
}

pub fn get_selected_goal<'a>(
    selected_goal: &SelectedGoal,
    goals: &'a [PopulatedGoal],
) -> Result<&'a PopulatedGoal, CursorError> {
    let mut current = goals
        .get(selected_goal.root_goal_index)
        .ok_or(CursorError::InvalidRootIndex(selected_goal.clone()))?;

    for index in &selected_goal.child_index_path {
        current = current
            .children
            .get(*index)
            .ok_or(CursorError::InvalidGoalChild {
                goal: current.clone(),
                child_index: *index,
            })?;
    }

    Ok(current)
}

pub fn get_selected_goal_id(
    selected_goal: &SelectedGoal,
    goals: &[PopulatedGoal],
) -> Result<GoalId, CursorError> {
    let selected_goal_data = get_selected_goal(selected_goal, goals)?;
    Ok(selected_goal_data.id)
}

impl Cursor {
    pub fn handle_action(
        &mut self,
        action: CursorAction,
        goals: &Vec<PopulatedGoal>,
    ) -> Result<(), CursorError> {
        use CursorAction::*;

        match self {
            Cursor::SelectedGoal(selected_goal_index_path) => {
                match selected_goal_index_path.as_mut() {
                    Some(selected_goal) => match action {
                        Down => {
                            let sibling_goals = selected_goal_siblings(selected_goal, goals)?;

                            let selected_goal_index = selected_goal.selected_index();
                            if sibling_goals.len() > (*selected_goal_index) + 1 {
                                *selected_goal_index += 1;
                            }

                            Ok(())
                        }
                        Up => {
                            let selected_goal_index = selected_goal.selected_index();
                            if *selected_goal_index > 0 {
                                *selected_goal_index -= 1;
                            }
                            Ok(())
                        }
                        In => {
                            let root_goal = goals
                                .get(selected_goal.root_goal_index)
                                .ok_or(CursorError::InvalidRootIndex(selected_goal.clone()))?;

                            let selected_goal_data = traverse_populated_goal_children(
                                root_goal,
                                &selected_goal.child_index_path,
                            )
                            .ok_or(CursorError::TraversalError(selected_goal.clone()))?;

                            if !selected_goal_data.children.is_empty() {
                                selected_goal.push_child(0);
                            }

                            Ok(())
                        }
                        Out => {
                            if selected_goal.pop_child().is_none() {
                                *selected_goal_index_path = None;
                            }
                            Ok(())
                        }
                    },
                    None => {
                        if !goals.is_empty() {
                            *self = Cursor::SelectedGoal(Some(SelectedGoal {
                                root_goal_index: 0,
                                child_index_path: vec![],
                            }));
                        }

                        Ok(())
                    }
                }
            }
        }
    }
}
