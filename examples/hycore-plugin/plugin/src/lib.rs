use hycore::{
    base::ext::{PluginExt, PluginExtStatic},
    define_plugin_compatibility, define_plugin_loader,
};
use semver::Version;
use uuid::{Uuid, uuid};

define_plugin_compatibility!(">=0.1.0");
define_plugin_loader!(Plugin);

pub struct Plugin {
    version: Version,
}

impl PluginExtStatic for Plugin {
    const UUID: Uuid = uuid!("a8af402c-7892-4b7f-9aa1-ca4b9bd47c94");

    fn new() -> Self {
        Self {
            version: Version::parse("0.2.3").unwrap(),
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
}
