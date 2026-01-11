#[cfg(feature = "pyo3")]
use pyo3::prelude::*;
use strum::FromRepr;

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

/// Function ptr type for log callbacks
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
                timepoint: chrono::Local::now().naive_local(),
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
