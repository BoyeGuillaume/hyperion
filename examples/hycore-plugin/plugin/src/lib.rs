use std::sync::Weak;

use hycore::{
    base::{
        InstanceContext,
        ext::{PluginExt, PluginExtStatic},
    },
    define_plugin_compatibility, define_plugin_loader,
};
use semver::Version;
use uuid::{Uuid, uuid};

define_plugin_compatibility!(">=0.1.0");
define_plugin_loader!(Plugin);

pub struct Plugin {
    version: Version,
    instance: Option<Weak<InstanceContext>>,
}

impl PluginExtStatic for Plugin {
    const UUID: Uuid = uuid!("a8af402c-7892-4b7f-9aa1-ca4b9bd47c94");

    fn new() -> Self {
        Self {
            version: Version::parse("0.2.3").unwrap(),
            instance: None,
        }
    }
}

impl PluginExt for Plugin {
    fn uuid(&self) -> uuid::Uuid {
        Self::UUID
    }

    fn version(&self) -> &semver::Version {
        &self.version
    }

    fn name(&self) -> &str {
        "__EXT_PLUGIN_EXAMPLE"
    }

    fn description(&self) -> &str {
        "An example plugin extension for Hycore."
    }

    fn initialize(
        &mut self,
        instance: std::sync::Weak<hycore::base::InstanceContext>,
    ) -> hycore::utils::error::HyResult<()> {
        self.instance = Some(instance);
        Ok(())
    }

    fn teardown(self) {
        // Clean up resources if needed
    }
}
