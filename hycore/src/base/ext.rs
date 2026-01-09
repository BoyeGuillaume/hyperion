use std::{ops::Deref, sync::Arc};

use libloading::Library;
use semver::{Version, VersionReq};
use uuid::Uuid;

use crate::{
    base::meta::HyperionMetaInfo,
    magic::{EXT_COMPATIBILITY_CHECK_FN_NAME, EXT_LOADER_FN_NAME},
    utils::error::{HyError, HyResult},
};

/// Prototype of the extension loader function. Function name should be [`EXT_LOADER_FN_NAME`].
pub type ExtLoaderFn = unsafe fn(Uuid) -> HyResult<Box<dyn PluginExt>>;

/// Prototype of the extension compatibility version check function. Function name should be
/// [`EXT_COMPATIBILITY_CHECK_FN_NAME`].
pub type ExtCompatibilityCheckFn = unsafe fn() -> VersionReq;

/// Macro to define the compatibility check function for a plugin extension.
#[macro_export]
macro_rules! define_plugin_compatibility {
    (
        $compat:literal
    ) => {
        #[unsafe(no_mangle)]
        pub fn __hyext_fn_compatibility_check() -> semver::VersionReq {
            semver::VersionReq::parse($compat).unwrap()
        }
    };
}

/// Macro that defines loader function for a plugin extension.
#[macro_export]
macro_rules! define_plugin_loader {
    (
        $( $plugin_ty:ty ),+
        $(,)?
    ) => {
        #[unsafe(no_mangle)]
        pub unsafe fn __hyext_fn_loader(uuid: uuid::Uuid) -> hycore::utils::error::HyResult<Box<dyn hycore::base::ext::PluginExt>> {
            match uuid {
                $(
                    <$plugin_ty as hycore::base::ext::PluginExtStatic>::UUID => {
                        let plugin = <$plugin_ty as hycore::base::ext::PluginExtStatic>::new();
                        assert_eq!(plugin.uuid(), <$plugin_ty as hycore::base::ext::PluginExtStatic>::UUID, "Plugin UUID does not match the expected UUID");
                        return Ok(Box::new(plugin));
                    },
                )+
                _ => {
                    Err(hycore::utils::error::HyError::ExtensionNotFound(uuid.to_string()))
                },
            }
        }
    };
}

pub trait PluginExt: Send + Sync {
    fn uuid(&self) -> Uuid;

    fn version(&self) -> &Version;

    fn name(&self) -> &str;

    fn description(&self) -> &str;
}

pub trait PluginExtStatic: PluginExt {
    const UUID: Uuid;

    fn new() -> Self;
}

/// Wrapper around a dynamically loaded plugin extension.
///
/// Prevents the library from being unloaded while the extension is in use.
pub struct PluginExtWrapper {
    ext: Box<dyn PluginExt>,
    /// SAFETY: Drop order ensures that the library is not unloaded before the extension is dropped.
    ///
    /// DO NOT CHANGE THE ORDER OF FIELDS!
    _lib: Arc<libloading::Library>,
}

impl Deref for PluginExtWrapper {
    type Target = dyn PluginExt;

    fn deref(&self) -> &Self::Target {
        &*self.ext
    }
}

pub fn load_plugin_by_name(
    meta_info: &HyperionMetaInfo,
    name: &str,
    library_version: Version,
) -> HyResult<PluginExtWrapper> {
    // Find the extension meta info by UUID
    let ext_info = meta_info
        .ext
        .iter()
        .find(|ext| ext.name == name)
        .ok_or(HyError::ExtensionNotFound(name.to_string()))?;

    // Load the dynamic library
    unsafe {
        let library = Library::new(&ext_info.path).map_err(|e| HyError::ExtensionLoadError {
            source: e,
            file: ext_info.path.clone(),
            name: ext_info.name.clone(),
        })?;

        // Get the compatibility check function
        let compat_check_fn: libloading::Symbol<ExtCompatibilityCheckFn> = library
            .get(EXT_COMPATIBILITY_CHECK_FN_NAME.as_bytes())
            .map_err(|e| HyError::ExtensionLoadError {
                source: e,
                file: ext_info.path.clone(),
                name: ext_info.name.clone(),
            })?;

        // Check compatibility
        let compat_req = compat_check_fn();
        if !compat_req.matches(&library_version) {
            return Err(HyError::CompatibilityCheckFailed {
                file: ext_info.path.clone(),
                name: ext_info.name.clone(),
                version: library_version,
                req: compat_req,
            });
        }

        // Get the loader function
        let loader_fn: libloading::Symbol<ExtLoaderFn> = library
            .get(EXT_LOADER_FN_NAME.as_bytes())
            .map_err(|e| HyError::ExtensionLoadError {
                source: e,
                file: ext_info.path.clone(),
                name: ext_info.name.clone(),
            })?;

        // Load the extension
        let ext = loader_fn(ext_info.uuid)?;

        Ok(PluginExtWrapper {
            _lib: Arc::new(library),
            ext,
        })
    }
}
