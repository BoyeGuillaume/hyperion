use crate::ext::ExtList;

pub mod constants;

#[derive(Debug, Clone, Copy)]
pub struct VersionInfo {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct ApplicationInfo<'a> {
    pub application_name: &'a str,
    pub application_version: VersionInfo,
    pub engine_name: &'a str,
    pub engine_version: VersionInfo,
}

#[derive(Debug)]
pub struct InstanceCreateInfo<'a> {
    /// Information about the library to distinguish for distinguishing between usecases and for debugging purposes.
    pub application_info: &'a ApplicationInfo<'a>,

    /// A list of enabled plugins. You can check the [`constants`] module for the available (default) plugins. Additional
    /// library and module may add their own plugins
    pub enabled_plugins: &'a [&'a str],

    /// Only the 6 least significant bytes of the node rank are used, everything else is ignored and should be set to 0
    pub node_rank: u32,

    /// Extension point
    pub ext: ExtList,
}
