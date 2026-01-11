use std::sync::Weak;

use hycore::{
    base::{
        InstanceContext,
        ext::{PluginExt, PluginExtStatic},
    },
    define_plugin,
};
use semver::Version;
use uuid::{Uuid, uuid};

define_plugin!(">=0.1.0",
    entry => plugin_entrypoint,
    teardown => plugin_teardown,
    plugins => [Plugin],
);

pub fn plugin_entrypoint(_library_builder: hycore::base::ext::LibraryBuilderPtr) {
    // Entry point logic for the plugin can be added here
    println!("Plugin entrypoint called.");
}

pub fn plugin_teardown(_library_builder: hycore::base::ext::LibraryBuilderPtr) {
    // Teardown logic for the plugin can be added here
    println!("Plugin teardown called.");
}

pub struct Plugin {
    version: Version,
    instance: Option<Weak<InstanceContext>>,
}

impl PluginExtStatic for Plugin {
    const UUID: Uuid = uuid!("a8af402c-7892-4b7f-9aa1-ca4b9bd47c94");

    fn new(_ext: &mut hycore::utils::conf::ExtList) -> Self {
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

    fn attach_to(&mut self, instance: Weak<InstanceContext>) {
        self.instance = Some(instance);
    }

    fn initialize(&self) -> hycore::utils::error::HyResult<()> {
        Ok(())
    }

    fn teardown(&mut self) {
        // Clean up resources if needed
    }
}
