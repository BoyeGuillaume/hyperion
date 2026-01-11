use semver::{Version, VersionReq};
use thiserror::Error;

#[derive(Debug, Error)]
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

    #[error("Extension with name '{0}' not found")]
    ExtensionNotFound(String),

    #[error("Failed to load extension '{name}' from file '{file}': {source}")]
    ExtensionLoadError {
        source: libloading::Error,
        file: String,
        name: String,
    },

    #[error("Symbol '{symbol}' not found in '{file}' for extension '{name}'")]
    ExtensionLoadErrorSymbolNotFound {
        file: String,
        name: String,
        symbol: &'static str,
    },

    #[error(
        "Compability check failed for extension '{name}' from file '{file}'. Required: {req}, found: {version}"
    )]
    CompatibilityCheckFailed {
        file: String,
        name: String,
        version: Version,
        req: VersionReq,
    },

    #[error("UTF-8 conversion error: {0}")]
    #[from(std::str::Utf8Error)]
    Utf8Error(std::str::Utf8Error),
}

pub type HyResult<T> = Result<T, HyError>;
