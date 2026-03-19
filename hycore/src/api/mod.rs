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

#[derive(Debug, Clone, Copy)]
pub struct InstanceCreateInfo<'a> {
    pub application_info: &'a ApplicationInfo<'a>,
    pub enabled_plugins: &'a [&'a str],
    // Only the 6 least significant bytes of the node rank are used, everything else is ignored and should be set to 0
    pub node_rank: u32,
}
