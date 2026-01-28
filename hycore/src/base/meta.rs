//! Helpers for locating and persisting Hyperion metadata on disk.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    magic::ENV_HOME_PATH,
    utils::error::{HyError, HyResult},
};

#[derive(Default, Serialize, Deserialize)]
/// Representation of the `meta.toml` manifest used to seed plugin discovery.
pub struct HyMetaConfig {}

impl HyMetaConfig {
    /// Path to the `HY_HOME` directory
    pub fn hy_home() -> PathBuf {
        let path = {
            if let Ok(home_path) = std::env::var(ENV_HOME_PATH) {
                PathBuf::from(home_path)
            } else {
                #[cfg(target_os = "windows")]
                {
                    if let Ok(appdata) = std::env::var("APPDATA") {
                        PathBuf::from(appdata).join("hyperion")
                    } else {
                        panic!("Neither HY_HOME nor APPDATA environment variables are set");
                    }
                }
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                {
                    if let Ok(home) = std::env::var("HOME") {
                        PathBuf::from(home).join(".cache").join("hyperion")
                    } else {
                        panic!("Neither HY_HOME nor HOME environment variables are set");
                    }
                }
            }
        };

        // Ensure the directory exists
        if let Err(e) = fs::create_dir_all(&path) {
            panic!(
                "Failed to create HY_HOME directory at {}: {}",
                path.display(),
                e
            );
        }
        path
    }

    /// Get the path to the config file, using environment variable or default
    pub fn hy_meta_config_path() -> PathBuf {
        Self::hy_home().join("meta.toml")
    }

    /// Load HyperionMetaInfo from a TOML string.
    pub fn load_from_toml(path: &Path) -> HyResult<Self> {
        let toml_str = fs::read_to_string(path).map_err(HyError::IoError)?;

        toml::from_str(&toml_str).map_err(|e| HyError::ManifestParseError {
            source: e,
            file: toml_str.to_string(),
        })
    }

    /// Save HyperionMetaInfo to a TOML file.
    pub fn save_to_toml(&self, path: &Path) -> HyResult<()> {
        let toml_str = toml::to_string(self).map_err(|e| {
            HyError::Unknown(format!(
                "Failed during serialization of TOML to path `{}`: {}",
                path.display(),
                e
            ))
        })?;

        // Attempt to create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(HyError::IoError)?;
        }

        // Write the TOML string to the specified path
        fs::write(path, toml_str).map_err(HyError::IoError)
    }
}
