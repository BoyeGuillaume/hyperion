use std::sync::Weak;

use hycore::{
    base::{
        InstanceContext,
        ext::{PluginExt, PluginExtStatic},
    },
    magic::{HYPERION_LOGGER_NAME_EXT, HYPERION_LOGGER_UUID_EXT},
};
use semver::Version;
use uuid::Uuid;

pub struct LogPlugin {
    version: Version,
    instance: Option<Weak<InstanceContext>>,
}

impl PluginExtStatic for LogPlugin {
    const UUID: Uuid = HYPERION_LOGGER_UUID_EXT;

    fn new(_ext: &mut hycore::utils::conf::ExtList) -> Self {
        let version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        Self {
            version,
            instance: None,
        }
    }
}

impl PluginExt for LogPlugin {
    fn uuid(&self) -> uuid::Uuid {
        Self::UUID
    }

    fn version(&self) -> &semver::Version {
        &self.version
    }

    fn name(&self) -> &str {
        HYPERION_LOGGER_NAME_EXT
    }

    fn description(&self) -> &str {
        "Hyperion Logger Extension"
    }

    fn initialize(&self) -> hycore::utils::error::HyResult<()> {
        Ok(())
    }

    fn attach_to(&mut self, instance: Weak<InstanceContext>) {
        self.instance = Some(instance);
    }

    fn teardown(self) {
        // Nothing to do
    }
}
