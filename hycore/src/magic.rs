use uuid::{Uuid, uuid};

/// Name of the environment variable containing the path to the Hy configuration file.
/// If not set, defaults to
///  (1) on Linux and macOS: `$HOME/.cache/hyperion/meta.toml`
///  (2) on Windows: `%APPDATA%\hyperion\meta.toml`
pub const ENV_HOME_PATH: &str = "HY_HOME";

/// Stdandard extension constants
/// Canonical plugin name registered for the built-in logger extension.
pub const HYPERION_LOGGER_NAME_EXT: &str = "__EXT_hyperion_logger";

/// UUID baked into the logger extension binary and recorded inside metadata files.
pub const HYPERION_LOGGER_UUID_EXT: Uuid = uuid!("20189b61-7279-46fa-9ba2-5f0360bf5b51");
