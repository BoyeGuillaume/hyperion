//! Error types shared across the runtime and extension ecosystem.

use strum::EnumDiscriminants;
use thiserror::Error;

/// Unified error enumeration for Hyperion.
#[derive(Debug, Error, EnumDiscriminants)]
#[strum_discriminants(name(HyErrorType))]
pub enum HyError {
    #[error("I/O error: {0}")]
    #[from(std::io::Error)]
    IoError(std::io::Error),

    #[error("Failed to parse manifest file '{file}': {source}")]
    ManifestParseError {
        source: toml::de::Error,
        file: String,
    },

    #[error("An unknown error occurred: {0}")]
    Unknown(String),

    #[error("Plugin with name '{0}' not found")]
    PluginNotFound(String),

    #[error("UTF-8 conversion error: {0}")]
    #[from(std::str::Utf8Error)]
    Utf8Error(std::str::Utf8Error),

    #[error("{0}")]
    HyInstrError(#[from] hyinstr::utils::Error),

    #[error("Failed to find {context} with key '{key}'")]
    KeyNotFound { key: String, context: String },

    #[error("Duplicated {context} with key '{key}'")]
    DuplicatedKey { key: String, context: String },
}

/// Convenience alias for fallible operations returning [`HyError`].
pub type HyResult<T> = Result<T, HyError>;
