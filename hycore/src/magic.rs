/// Name of the extension loader function.
pub const EXT_LOADER_FN_NAME: &str = "__hyext_fn_loader";

/// Name of the extension compatibility version check function.
pub const EXT_COMPATIBILITY_CHECK_FN_NAME: &str = "__hyext_fn_compatibility_check";

/// Name of the environment variable containing the path to the Hy configuration file.
/// If not set, defaults to
///  (1) on Linux and macOS: `$XDG_CONFIG_HOME/hyperion/meta.toml` or `$HOME/.config/hyperion/meta.toml`
///  (2) on Windows: `%APPDATA%\hyperion\meta.toml`
pub const ENV_META_CONFIG_PATH: &str = "HY_CONFIG_PATH";
