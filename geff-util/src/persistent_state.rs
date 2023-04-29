use std::fmt::Debug;
use std::path::PathBuf;

use geff_core::goal::GoalEvent;
use geff_core::profile::Profile;

use serde::de::DeserializeOwned;

use serde::{Deserialize, Serialize};

mod native;
mod wasm;

#[allow(unused)]
#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize)]
pub enum LoadError {
    #[error("APP_DATA or $HOME directory not found: {0}")]
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

#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize)]
pub enum SaveError {
    #[error("Failed to save config file due to IO write error: {0}")]
    WriteError(String),
    #[error("Failed to serialize config: {0}")]
    SerializeError(String),
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
