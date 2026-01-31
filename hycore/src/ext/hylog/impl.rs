use crate::{
    base::InstanceContext,
    ext::{
        PluginEXT, StaticPluginEXT,
        hylog::{LogCallbackEXT, LogCreateInfoEXT, LogLevelEXT, LogMessageEXT},
    },
    magic::{HYPERION_LOGGER_NAME_EXT, HYPERION_LOGGER_UUID_EXT},
    register,
    utils::{error::HyResult, opaque::OpaqueList},
};
use std::sync::Weak;
use uuid::Uuid;

/// Concrete logger plugin implementation backed by [`LogCreateInfoEXT`]
/// configuration and optional Python callbacks.
pub struct LogPluginEXT {
    instance: Option<Weak<InstanceContext>>,
    callback: Option<LogCallbackEXT>,
    min_level: LogLevelEXT,
}
register!(plugin LogPluginEXT);

impl StaticPluginEXT for LogPluginEXT {
    const UUID: Uuid = HYPERION_LOGGER_UUID_EXT;
    const NAME: &'static str = HYPERION_LOGGER_NAME_EXT;
    const DESCRIPTION: &'static str = "Hyperion Logger Extension";

    fn new(ext: &mut OpaqueList) -> Self {
        // Find the LogCreateInfoEXT in the ext list
        let mut callback = None;
        let mut min_level = LogLevelEXT::Trace;

        if let Some(create_info) = ext.take_ext::<LogCreateInfoEXT>() {
            min_level = create_info.level;
            callback = Some(create_info.callback);
        }

        Self {
            instance: None,
            min_level,
            callback,
        }
    }
}

impl PluginEXT for LogPluginEXT {
    fn initialize(&self) -> HyResult<()> {
        let instance = self.instance.as_ref().unwrap().upgrade().unwrap();
        let mut log_handle = instance.ext.log_callback.write();

        // Attach the log_handle to this extension's log_message function
        *log_handle = Self::log_message;

        Ok(())
    }

    fn attach_to(&mut self, instance: Weak<InstanceContext>) {
        self.instance = Some(instance);
    }

    fn teardown(&mut self) {
        // Restore default logger callback
    }
}

impl LogPluginEXT {
    /// Dispatches a [`LogMessageEXT`] to the registered callback (or stdout) if
    /// the message level is enabled.
    pub fn log_message(instance_context: &InstanceContext, message: LogMessageEXT) {
        if let Some(logger_ext) = instance_context.get_plugin_ext::<LogPluginEXT>() {
            if message.level < logger_ext.min_level {
                return;
            }

            if let Some(callback) = &logger_ext.callback {
                (callback.0)(message);
            } else {
                println!(
                    "[{:?}] {} - {}",
                    message.level, message.timepoint, message.message
                );
            }
        }
    }
}
