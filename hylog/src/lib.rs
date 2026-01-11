use std::sync::Weak;

use hycore::{
    base::{
        InstanceContext,
        ext::{PluginExt, PluginExtStatic},
    },
    define_plugin,
    magic::{HYPERION_LOGGER_NAME_EXT, HYPERION_LOGGER_UUID_EXT},
    utils::conf::OpaqueObject,
};
#[cfg(feature = "pyo3")]
use pyo3::{IntoPyObjectExt, prelude::*};
use semver::Version;
use uuid::Uuid;

pub const HYPERION_PY_NAME_LOG_CREATE_INFO_EXT: &str = "hypi.api.ext_hylog.LogCreateInfoEXT";

define_plugin!("=0.1.0",
    entry => logger_entrypoint,
    teardown => logger_teardown,
    plugins => [LogPluginEXT],
);

pub fn logger_entrypoint(_library_builder: hycore::base::ext::LibraryBuilderPtr) {
    // Register the LogCreateInfoEXT structure to be understood by Hycore's Python integration
    println!("Logger plugin entrypoint called.");
    #[cfg(feature = "pyo3")]
    if let Some(opaque_object_loader) = _library_builder.opaque_object_loader {
        println!("Registering LogCreateInfoEXT loader.");

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
        println!("Logger plugin teardown called.");

        opaque_object_loader
            .write()
            .remove(HYPERION_PY_NAME_LOG_CREATE_INFO_EXT);
    }
}

/// Logger levels supported by the logger extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "pyo3", pyclass(eq, eq_int))]
pub enum LogLevelEXT {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

/// Message structure for the logger extension.
#[cfg_attr(feature = "pyo3", derive(IntoPyObject))]
pub struct LogMessageEXT {
    pub level: LogLevelEXT,
    pub message: String,
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
}

impl PluginExtStatic for LogPluginEXT {
    const UUID: Uuid = HYPERION_LOGGER_UUID_EXT;

    fn new(_ext: &mut hycore::utils::conf::ExtList) -> Self {
        let version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        Self {
            version,
            instance: None,
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
        Ok(())
    }

    fn attach_to(&mut self, instance: Weak<InstanceContext>) {
        self.instance = Some(instance);
    }

    fn teardown(&mut self) {
        // Nothing to do
    }
}
