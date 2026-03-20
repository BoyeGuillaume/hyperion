use pyo3::{intern, prelude::*, types::PyString};

#[doc(hidden)]
#[macro_export]
macro_rules! ensure_pyo3_qualname {
    (
        $obj:expr,
        $expected:expr
    ) => {{
        let objtype = $obj.get_type();
        let qualname = objtype.qualname()?;
        if qualname != $expected {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Expected an object of type '{}', but got {}",
                $expected, qualname
            )));
        }
    }};
}

use crate::{
    api::{ApplicationInfo, VersionInfo},
    ext::ExtList,
};

impl<'a, 'py> FromPyObject<'a, 'py> for VersionInfo {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        // Expect a string in the format "major.minor.patch"
        let s = obj.extract::<&str>()?;
        s.parse::<VersionInfo>()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    }
}

impl<'py> IntoPyObject<'py> for VersionInfo {
    type Target = PyString;
    type Output = Bound<'py, PyString>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let s = self.to_string();
        Ok(PyString::intern(py, &s))
    }
}

// Application Info
impl<'a, 'py> FromPyObject<'a, 'py> for ApplicationInfo<'static> {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        ensure_pyo3_qualname!(obj, "ApplicationInfo");

        let py = obj.py();
        let application_name = obj
            .getattr(intern!(py, "application_name"))?
            .extract::<String>()?;

        let application_version: VersionInfo =
            obj.getattr(intern!(py, "application_version"))?.extract()?;

        let engine_name = obj
            .getattr(intern!(py, "engine_name"))?
            .extract::<Option<String>>()?;

        let engine_version = obj
            .getattr(intern!(py, "engine_version"))?
            .extract::<Option<VersionInfo>>()?;

        Ok(ApplicationInfo {
            application_name: application_name.into(),
            application_version,
            engine_name: engine_name.map(Into::into),
            engine_version,
        })
    }
}

// Instance Create Info
impl<'py, 'a> FromPyObject<'a, 'py> for crate::api::InstanceCreateInfo<'a> {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        ensure_pyo3_qualname!(obj, "InstanceCreateInfo");

        let py = obj.py();
        let application_info: ApplicationInfo<'static> = obj
            .getattr(intern!(py, "application_info"))?
            .extract::<ApplicationInfo>()?;

        let enabled_plugins = obj
            .getattr(intern!(py, "enabled_plugins"))?
            .extract::<Vec<String>>()?;

        let node_rank = obj.getattr(intern!(py, "node_rank"))?.extract::<u64>()?;
        let ext = obj.getattr(intern!(py, "ext"))?.extract::<ExtList>()?;

        Ok(crate::api::InstanceCreateInfo {
            application_info,
            enabled_plugins,
            node_rank,
            ext,
        })
    }
}
