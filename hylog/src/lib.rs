use std::sync::Weak;

use hycore::{
    base::{
        InstanceContext,
        ext::{PluginExt, PluginExtStatic},
    },
    define_plugin,
    ext::hylog::{LogLevelEXT, LogMessageEXT},
    magic::{HYPERION_LOGGER_NAME_EXT, HYPERION_LOGGER_UUID_EXT},
    utils::conf::OpaqueObject,
};
#[cfg(feature = "pyo3")]
use pyo3::{IntoPyObjectExt, prelude::*};
use semver::Version;
use uuid::Uuid;

pub const HYPERION_PY_NAME_LOG_CREATE_INFO_EXT: &str = "hypi.api.ext_hylog.LogCreateInfoEXT";

define_plugin!("=0.1.1",
    entry => logger_entrypoint,
    teardown => logger_teardown,
    plugins => [LogPluginEXT],
);

pub fn logger_entrypoint(_library_builder: hycore::base::ext::LibraryBuilderPtr) {
    // Register the LogCreateInfoEXT structure to be understood by Hycore's Python integration
    #[cfg(feature = "pyo3")]
    if let Some(opaque_object_loader) = _library_builder.opaque_object_loader {
        opaque_object_loader.write().insert(
            HYPERION_PY_NAME_LOG_CREATE_INFO_EXT.to_string(),
            |obj: pyo3::Borrowed<'_, '_, pyo3::PyAny>| -> pyo3::PyResult<Box<dyn hycore::utils::conf::OpaqueObject>> {
                let info: LogCreateInfoEXT = obj.extract()?;
                Ok(Box::new(info))
            },
        );
    }
}

pub fn logger_teardown(_library_builder: hycore::base::ext::LibraryBuilderPtr) {
    // Teardown logic for the logger extension can be added here
    #[cfg(feature = "pyo3")]
    if let Some(opaque_object_loader) = _library_builder.opaque_object_loader {
        opaque_object_loader
            .write()
            .remove(HYPERION_PY_NAME_LOG_CREATE_INFO_EXT);
    }
}

pub struct LogCallbackEXT(pub Box<dyn Fn(LogMessageEXT) + Send + Sync>);

#[cfg(feature = "pyo3")]
impl<'a, 'py> FromPyObject<'a, 'py> for LogCallbackEXT {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if !obj.is_callable() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Expected a callable object for LogCallbackEXT",
            ));
        }
        let obj = obj.to_owned().unbind();

        let box_fn = Box::new(move |msg: LogMessageEXT| {
            Python::attach(|py| -> PyResult<()> {
                // Call the Python callable with the LogMessageEXT
                let py_msg = msg.into_py_any(py)?;
                obj.bind(py).call1((py_msg,))?;
                Ok(())
            })
            .unwrap();
        });
        Ok(LogCallbackEXT(box_fn))
    }
}

/// Creation information for the logger extension.
#[cfg_attr(feature = "pyo3", derive(FromPyObject))]
pub struct LogCreateInfoEXT {
    pub level: LogLevelEXT,
    pub callback: LogCallbackEXT,
}
impl OpaqueObject for LogCreateInfoEXT {}

pub struct LogPluginEXT {
    version: Version,
    instance: Option<Weak<InstanceContext>>,
    callback: Option<LogCallbackEXT>,
    min_level: LogLevelEXT,
}

impl PluginExtStatic for LogPluginEXT {
    const UUID: Uuid = HYPERION_LOGGER_UUID_EXT;

    fn new(ext: &mut hycore::utils::conf::ExtList) -> Self {
        let version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

        // Find the LogCreateInfoEXT in the ext list
        let mut callback = None;
        let mut min_level = LogLevelEXT::Trace;

        if let Some(create_info) = ext.take_ext::<LogCreateInfoEXT>() {
            min_level = create_info.level;
            callback = Some(create_info.callback);
        }

        Self {
            version,
            instance: None,
            min_level,
            callback,
        }
    }
}

impl PluginExt for LogPluginEXT {
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
        let instance = self.instance.as_ref().unwrap().upgrade().unwrap();
        instance.ext.restore_default_log_callback();
    }
}

impl LogPluginEXT {
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
