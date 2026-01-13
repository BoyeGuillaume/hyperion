use hyinstr::{
    modules::{Module, parser::extend_module_from_string},
    types::TypeRegistry,
};
use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    base::{
        InstanceContext,
        api::{ModuleCompileInfo, ModuleSourceType},
    },
    hyerror, hytrace,
    utils::error::{HyError, HyResult},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledModuleStorageInternal {
    pub filenames: Vec<String>,
    pub module: Module,
}

impl CompiledModuleStorageInternal {
    pub fn encode(&self, instance: &InstanceContext) -> HyResult<Vec<u8>> {
        // Serialize inner using dlhn
        let mut buf = Vec::new();
        let mut serializer = serde_cbor::Serializer::new(&mut buf);
        self.serialize(&mut serializer).map_err(|e| {
            hyerror!(
                instance,
                "Failed to serialize compiled module storage: {}",
                e
            );
            HyError::Unknown(format!(
                "Failed to serialize compiled module storage: {}",
                e
            ))
        })?;

        // Secondly, wrap in CompiledModuleStorage
        let storage = CompiledModuleStorage {
            magic: CompiledModuleStorage::MAGIC_BYTES,
            version_req: semver::VersionReq {
                comparators: vec![semver::Comparator {
                    op: semver::Op::Exact,
                    major: instance.version.major,
                    minor: Some(instance.version.minor),
                    patch: Some(instance.version.patch),
                    pre: instance.version.pre.clone(),
                }],
            },
            data: buf,
        };

        // Serialize the storage using serde_cbor for stability
        hytrace!(
            instance,
            "Serializing compiled module storage wrapper with version requirement {}",
            storage.version_req
        );
        let mut buf = Vec::new();
        serde_cbor::to_writer(&mut buf, &storage).map_err(|e| {
            hyerror!(
                instance,
                "Failed to serialize compiled module storage wrapper: {}",
                e
            );
            HyError::Unknown(format!(
                "Failed to serialize compiled module storage wrapper: {}",
                e
            ))
        })?;

        Ok(buf)
    }

    pub fn decode(&self, instance: &InstanceContext, data: &[u8]) -> HyResult<Self> {
        hytrace!(
            instance,
            "Deserializing compiled module storage ({} bytes)",
            data.len()
        );

        // Deserialize the storage using serde_cbor
        let storage: CompiledModuleStorage = serde_cbor::from_slice(data).map_err(|e| {
            hyerror!(
                instance,
                "Failed to deserialize compiled module storage wrapper: {}",
                e
            );
            HyError::Unknown(format!(
                "Failed to deserialize compiled module storage wrapper: {}",
                e
            ))
        })?;

        // Check magic bytes
        if storage.magic != CompiledModuleStorage::MAGIC_BYTES {
            hyerror!(instance, "Invalid magic bytes in compiled module storage");
            return Err(HyError::Unknown(
                "Invalid magic bytes in compiled module storage".to_string(),
            ));
        }

        // Check version compatibility
        if !storage.version_req.matches(&instance.version) {
            hyerror!(
                instance,
                "Incompatible compiled module storage version: required {}, found {}",
                storage.version_req,
                instance.version
            );
            return Err(HyError::Unknown(format!(
                "Incompatible compiled module storage version: required {}, found {}",
                storage.version_req, instance.version
            )));
        }

        // Deserialize inner using dlhn
        let compiled_module: Self =
            serde_cbor::from_slice(storage.data.as_slice()).map_err(|e| {
                hyerror!(
                    instance,
                    "Failed to deserialize compiled module storage: {}",
                    e
                );
                HyError::Unknown(format!(
                    "Failed to deserialize compiled module storage: {}",
                    e
                ))
            })?;

        Ok(compiled_module)
    }
}

/// Compiled IR module storage format.
///
/// This is used to store compiled-module metadata alongside the serialized module data.
/// This should be kept stable across all versions of Hyperion. No modifications should be made
/// to this structure that would break compatibility with previously stored modules.
///
/// See [`CompiledModuleStorage::MAGIC_BYTES`] for the magic bytes used to identify compiled module storage files.
/// See [`CompiledModuleStorageInternal`] for the internal representation used during (de)serialization.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledModuleStorage {
    pub magic: [u8; 8],
    pub version_req: semver::VersionReq,
    pub data: Vec<u8>,
}

impl CompiledModuleStorage {
    /// Magic bytes used to identify compiled module storage files.
    pub const MAGIC_BYTES: [u8; 8] = *b"\0HYCOMP\0";
}

pub fn compile_sources(
    instance: &InstanceContext,
    compile_info: ModuleCompileInfo,
) -> HyResult<Vec<u8>> {
    let mut module = Module::default();
    let type_registry = TypeRegistry::new([0u8; 6]);
    let mut filenames = Vec::new();

    // Compile each source in the compile_info
    for source_info in compile_info.sources {
        hytrace!(
            instance,
            "Compiling source \"{}\"",
            source_info.filename.as_deref().unwrap_or("<unnamed>")
        );

        if let Some(filename) = source_info.filename {
            filenames.push(filename);
        }

        match source_info.source_type {
            ModuleSourceType::Assembly => {
                // Compile assembly source code into the module
                extend_module_from_string(&mut module, &type_registry, &source_info.data)?;
            }
        }
    }

    // Produce compiled module storage or further processing here
    let storage = CompiledModuleStorageInternal { module, filenames };
    let encoded_storage = storage.encode(instance)?;

    // Information about the compiled module can be used here
    info!(
        "Compiled successful, module has {} functions: {:?}",
        storage.module.functions.len(),
        storage
            .module
            .functions
            .values()
            .map(|x| x.name.clone().unwrap_or_else(|| format!("@{}", x.uuid)))
            .collect::<Vec<_>>()
    );
    info!(
        "Produced {} bytes of compiled module storage",
        encoded_storage.len()
    );

    Ok(encoded_storage)
}
