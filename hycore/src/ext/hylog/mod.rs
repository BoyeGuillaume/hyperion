//! Cross-language logging primitives used by Hyperion instances and
//! extensions.

#[cfg(feature = "ext_hylog")]
mod r#impl;

#[cfg(feature = "pyo3")]
use pyo3::prelude::*;
use strum::FromRepr;

use crate::{define_py_opaque_object_loaders, utils::opaque::OpaqueObject};

/// Formats a message and forwards it to the runtime log callback registered on
/// an [`InstanceContext`](crate::base::InstanceContext).
#[macro_export]
macro_rules! hylog {
    (
        $instance:expr,
        $level:expr,
        $( $arg:tt )*
    ) => {
        {
            let msg = $crate::ext::hylog::LogMessageEXT {
                level: $level,
                timepoint: $crate::chrono::Local::now().naive_local(),
                message: format!($($arg)*),
                module: module_path!().to_string(),
                file: Some(file!().to_string()),
                line: Some(line!()),
                thread_name: std::thread::current().name().map(|s| s.to_string()),
            };
            let instance = &*$instance;
            (instance.ext.log_callback.read())(&instance, msg);

        }
    };
}

/// Emits a trace-level log entry routed through the logger extension.
#[macro_export]
macro_rules! hytrace {
    (
        $instance:expr,
        $( $arg:tt )*
    ) => {
        $crate::hylog!(
            $instance,
            $crate::ext::hylog::LogLevelEXT::Trace,
            $( $arg )*
        );
    };
}

/// Emits a debug-level log entry routed through the logger extension.
#[macro_export]
macro_rules! hydebug {
    (
        $instance:expr,
        $( $arg:tt )*
    ) => {
        $crate::hylog!(
            $instance,
            $crate::ext::hylog::LogLevelEXT::Debug,
            $( $arg )*
        );
    };
}

/// Emits an info-level log entry routed through the logger extension.
#[macro_export]
macro_rules! hyinfo {
    (
        $instance:expr,
        $( $arg:tt )*
    ) => {
        $crate::hylog!(
            $instance,
            $crate::ext::hylog::LogLevelEXT::Info,
            $( $arg )*
        );
    };
}

/// Emits a warning-level log entry routed through the logger extension.
#[macro_export]
macro_rules! hywarn {
    (
        $instance:expr,
        $( $arg:tt )*
    ) => {
        $crate::hylog!(
            $instance,
            $crate::ext::hylog::LogLevelEXT::Warn,
            $( $arg )*
        );
    };
}

/// Emits an error-level log entry routed through the logger extension.
#[macro_export]
macro_rules! hyerror {
    (
        $instance:expr,
        $( $arg:tt )*
    ) => {
        $crate::hylog!(
            $instance,
            $crate::ext::hylog::LogLevelEXT::Error,
            $( $arg )*
        );
    };
}

/// Logger levels supported by the logger extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, FromRepr)]
#[repr(u32)]
pub enum LogLevelEXT {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

#[cfg(feature = "pyo3")]
impl<'a, 'py> FromPyObject<'a, 'py> for LogLevelEXT {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, PyErr> {
        let level_int: u32 = obj.extract()?;
        if let Some(level) = LogLevelEXT::from_repr(level_int) {
            Ok(level)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid LogLevelEXT value: {}",
                level_int
            )))
        }
    }
}

#[cfg(feature = "pyo3")]
impl<'py> IntoPyObject<'py> for LogLevelEXT {
    type Target = pyo3::PyAny;

    type Output = pyo3::Bound<'py, pyo3::PyAny>;

    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, PyErr> {
        use pyo3::IntoPyObjectExt;

        Ok((self as usize).into_py_any(py)?.bind(py).to_owned())
    }
}

/// Message structure for the logger extension.
#[cfg_attr(feature = "pyo3", derive(IntoPyObject))]
pub struct LogMessageEXT {
    pub level: LogLevelEXT,
    pub timepoint: chrono::NaiveDateTime,
    pub message: String,
    pub module: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub thread_name: Option<String>,
}

/// Wrapper around user-supplied log callbacks so they can be stored in
/// `ExtList` and invoked from Rust.
pub struct LogCallbackEXT(pub Box<dyn Fn(LogMessageEXT) + Send + Sync>);

impl std::fmt::Debug for LogCallbackEXT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogCallbackEXT")
            .field(
                "0",
                &format!("{:p}", ::core::ptr::from_ref(self.0.as_ref())),
            )
            .finish()
    }
}

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
                use pyo3::IntoPyObjectExt;

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
#[derive(Debug)]
pub struct LogCreateInfoEXT {
    pub level: LogLevelEXT,
    pub callback: LogCallbackEXT,
}
impl OpaqueObject for LogCreateInfoEXT {}
define_py_opaque_object_loaders!("hypi.api.ext.hylog.LogCreateInfoEXT", LogCreateInfoEXT);
