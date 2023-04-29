use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use serde::Serialize;

use super::{LoadError, PersistentState, SaveError};

#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(not(target_arch = "wasm32"))]
impl<C: std::fmt::Debug + Serialize + Clone + Default + DeserializeOwned> PersistentState<C> {
    #[cfg(target_os = "windows")]
    pub fn default_data_path<S: AsRef<str>>(app_name: S) -> Result<PathBuf, LoadError> {
        let appdata = PathBuf::from(
            std::env::var("APPDATA")
                .map_err(|e| LoadError::NoAppDataOrHomeDirectory(e.to_string()))?,
        );

        Ok(app_data
            .join("Roaming")
            .join(app_name.as_ref())
            .join("data"))
    }

    #[cfg(target_os = "linux")]
    pub fn default_data_path<S: AsRef<str>>(app_name: S) -> Result<PathBuf, LoadError> {
        let home = PathBuf::from(
            std::env::var("HOME")
                .map_err(|e| LoadError::NoAppDataOrHomeDirectory(e.to_string()))?,
        );
        Ok(home.join(format!(".{}", app_name.as_ref())))
    }

    #[cfg(target_os = "macos")]
    pub fn default_data_path<S: AsRef<str>>(app_name: S) -> Result<PathBuf, LoadError> {
        Ok(PathBuf::from(format!(
            "~Library/Application Support/{}/Data",
            app_name.as_ref()
        )))
    }

    pub fn data_path<S: AsRef<str>>(app_name: S) -> Result<PathBuf, LoadError> {
        std::env::var("GEFF_CORE_ICED_DATA_PATH")
            .map(|p| Ok(PathBuf::from(p)))
            .unwrap_or(Self::default_data_path(app_name))
    }

    pub async fn save_to_file<P: AsRef<Path>>(&self, p: P) -> Result<(), SaveError> {
        use tokio::fs;

        fs::write(
            p,
            rmp_serde::to_vec(self).map_err(|e| SaveError::SerializeError(e.to_string()))?,
        )
        .await
        .map_err(|e| SaveError::WriteError(e.to_string()))
    }

    pub async fn load<P: AsRef<Path>>(profile_data_path: P) -> Result<Self, LoadError> {
        use tokio::fs;

        if !profile_data_path.as_ref().exists() {
            fs::create_dir_all(
                profile_data_path
                    .as_ref()
                    .parent()
                    .expect("profile data path to have parent"),
            )
            .await
            .map_err(|e| {
                LoadError::ProfileDataCreation(
                    profile_data_path.as_ref().to_path_buf(),
                    e.to_string(),
                )
            })?;

            let default_data = rmp_serde::encode::to_vec(&Self::default())
                .expect("default data type to be serializable");

            fs::File::create(&profile_data_path)
                .await
                .map_err(|e| {
                    LoadError::ProfileDataCreation(
                        profile_data_path.as_ref().to_path_buf(),
                        e.to_string(),
                    )
                })?
                .write_all(&default_data)
                .await
                .map_err(|e| {
                    LoadError::FailureToWriteDefaultData(
                        profile_data_path.as_ref().to_path_buf(),
                        e.to_string(),
                    )
                })?;
        }

        let mut data_file = fs::File::open(profile_data_path.as_ref())
            .await
            .map_err(|e| {
                LoadError::ProfileDataFileRead(
                    profile_data_path.as_ref().to_path_buf(),
                    e.to_string(),
                )
            })?;

        let mut profile_bytes = vec![];
        data_file
            .read_to_end(&mut profile_bytes)
            .await
            .map_err(|e| {
                LoadError::ProfileDataFileRead(
                    profile_data_path.as_ref().to_path_buf(),
                    e.to_string(),
                )
            })?;

        rmp_serde::decode::from_slice(&profile_bytes).map_err(|e| {
            LoadError::MalformedProfileDataFile(
                profile_data_path.as_ref().to_path_buf(),
                e.to_string(),
            )
        })
    }

    pub fn blocking_load<P: AsRef<Path>>(profile_data_path: P) -> Result<Self, LoadError> {
        use std::fs;

        if !profile_data_path.as_ref().exists() {
            fs::create_dir_all(
                profile_data_path
                    .as_ref()
                    .parent()
                    .expect("profile data path to have parent"),
            )
            .map_err(|e| {
                LoadError::ProfileDataCreation(
                    profile_data_path.as_ref().to_path_buf(),
                    e.to_string(),
                )
            })?;

            let default_data = rmp_serde::encode::to_vec(&Self::default())
                .expect("default data type to be serializable");

            fs::File::create(&profile_data_path)
                .map_err(|e| {
                    LoadError::ProfileDataCreation(
                        profile_data_path.as_ref().to_path_buf(),
                        e.to_string(),
                    )
                })?
                .write_all(&default_data)
                .map_err(|e| {
                    LoadError::FailureToWriteDefaultData(
                        profile_data_path.as_ref().to_path_buf(),
                        e.to_string(),
                    )
                })?;
        }

        let mut data_file = fs::File::open(profile_data_path.as_ref()).map_err(|e| {
            LoadError::ProfileDataFileRead(profile_data_path.as_ref().to_path_buf(), e.to_string())
        })?;

        let mut profile_bytes = vec![];
        data_file.read_to_end(&mut profile_bytes).map_err(|e| {
            LoadError::ProfileDataFileRead(profile_data_path.as_ref().to_path_buf(), e.to_string())
        })?;

        rmp_serde::decode::from_slice(&profile_bytes).map_err(|e| {
            LoadError::MalformedProfileDataFile(
                profile_data_path.as_ref().to_path_buf(),
                e.to_string(),
            )
        })
    }
}
