use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    magic::ENV_META_CONFIG_PATH,
    utils::error::{HyError, HyResult},
};

#[derive(Serialize, Deserialize)]
pub struct ExtMetaInfo {
    pub uuid: Uuid,
    pub path: String,
    pub name: String,
}

#[derive(Default, Serialize, Deserialize)]
pub struct HyperionMetaInfo {
    pub ext: Vec<ExtMetaInfo>,
}

impl HyperionMetaInfo {
    /// Get the default path to the Hy configuration file.
    pub fn default_path() -> PathBuf {
        // Check if the environment variable is set
        if let Ok(config_path) = std::env::var(ENV_META_CONFIG_PATH) {
            return config_path.into();
        }

        // Fallback to default paths based on OS
        let mut path = PathBuf::new();

        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                path.push(appdata);
            }

            // Fallback to current directory if APPDATA is not set
            path.push("hyperion");
            path.push("meta.toml");
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
                path.push(xdg_config_home);
            } else if let Ok(home) = std::env::var("HOME") {
                path.push(home);
                path.push(".config");
            } else {
                // Fallback to current directory if HOME is not set
            }

            path.push("hyperion");
            path.push("meta.toml");
        }

        path
    }

    /// Load HyperionMetaInfo from a TOML string.
    pub fn load_from_toml(path: &Path) -> HyResult<Self> {
        let toml_str = std::fs::read_to_string(path).map_err(|e| HyError::IoError(e))?;

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
            std::fs::create_dir_all(parent).map_err(|e| HyError::IoError(e))?;
        }

        // Write the TOML string to the specified path
        std::fs::write(path, toml_str).map_err(|e| HyError::IoError(e))
    }
}
