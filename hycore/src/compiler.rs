use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use hyinstr::{
    modules::{Module, parser::extend_module_from_string},
    types::TypeRegistry,
};

use crate::{
    base::{
        InstanceContext, ModuleKey,
        api::{ModuleCompileInfo, ModuleSourceType},
    },
    hyerror, hyinfo, hytrace,
    utils::error::{HyError, HyResult},
};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct CompiledModuleStorage {
    pub filenames: Vec<String>,
    pub module: Module,
    pub type_registry: TypeRegistry,
}

impl CompiledModuleStorage {
    /// Distinguishing magic bytes for compiled module storage files
    #[cfg(feature = "legacy_nozstd")]
    pub const MAGIC_BYTES: [u8; 8] = *b"\x80HYMODIR";
    #[cfg(not(feature = "legacy_nozstd"))]
    pub const MAGIC_BYTES: [u8; 8] = *b"\x7FHYMODIR";

    fn writer_header<W: std::io::Write>(
        &self,
        instance: &InstanceContext,
        writer: &mut W,
    ) -> std::io::Result<()> {
        // Write magic bytes
        writer.write_all(&Self::MAGIC_BYTES)?;

        // Write version requirement (using semver format)
        let version_req = semver::VersionReq {
            comparators: vec![semver::Comparator {
                op: semver::Op::Exact,
                major: instance.version.major,
                minor: Some(instance.version.minor),
                patch: Some(instance.version.patch),
                pre: instance.version.pre.clone(),
            }],
        };
        let mut version_req_str = version_req.to_string();

        // Write null-terminated version requirement string
        version_req_str.push('\0');
        let version_req_bytes = version_req_str.as_bytes();

        // Write version requirement string bytes
        writer.write_all(version_req_bytes)?;

        Ok(())
    }

    fn read_header<R: std::io::Read>(
        instance: &InstanceContext,
        reader: &mut R,
    ) -> std::io::Result<()> {
        // Read and verify magic bytes
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if magic != Self::MAGIC_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid magic bytes in compiled module storage",
            ));
        }

        // Read version requirement string until null terminator
        let mut version_req_bytes = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            reader.read_exact(&mut byte)?;
            if byte[0] == 0 {
                break;
            }
            version_req_bytes.push(byte[0]);
        }

        // Parse version requirement
        let version_req_str = String::from_utf8(version_req_bytes).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid UTF-8 in version requirement: {}", e),
            )
        })?;
        let version_req = semver::VersionReq::parse(&version_req_str).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid version requirement format: {}", e),
            )
        })?;

        // Check version compatibility
        if !version_req.matches(&instance.version) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Incompatible compiled module storage version: required {}, found {}",
                    version_req, instance.version
                ),
            ));
        }

        Ok(())
    }

    pub fn encode(&self, instance: &InstanceContext) -> HyResult<Vec<u8>> {
        // Serialize inner using borsh
        hytrace!(
            instance,
            "Serializing compiled module storage (module has {} functions)",
            self.module.functions.len()
        );
        let mut buf = Vec::new();
        let mut writer = &mut buf;

        self.writer_header(instance, &mut writer)
            .and_then(|_| {
                #[cfg(feature = "legacy_nozstd")]
                borsh::BorshSerialize::serialize(&self, &mut writer)?;

                #[cfg(not(feature = "legacy_nozstd"))]
                {
                    let mut zstd_writer = zstd::stream::write::Encoder::new(writer, 3).unwrap();
                    borsh::BorshSerialize::serialize(&self, &mut zstd_writer)?;
                    zstd_writer.finish()?;
                }
                Ok(())
            })
            .map_err(|e| {
                hyerror!(
                    instance,
                    "Failed to serialize compiled module storage header: {}",
                    e
                );
                HyError::Unknown(format!(
                    "Failed to serialize compiled module storage header: {}",
                    e
                ))
            })?;

        Ok(buf)
    }

    pub fn decode(instance: &InstanceContext, data: &[u8]) -> HyResult<Self> {
        hytrace!(
            instance,
            "Deserializing compiled module storage ({} bytes)",
            data.len()
        );

        let mut reader = data;
        Self::read_header(instance, &mut reader)
            .and_then(|_| {
                #[cfg(feature = "legacy_nozstd")]
                {
                    borsh::BorshDeserialize::deserialize_reader(&mut reader)
                }
                #[cfg(not(feature = "legacy_nozstd"))]
                {
                    let mut zstd_reader = zstd::stream::read::Decoder::new(reader).unwrap();
                    borsh::BorshDeserialize::deserialize_reader(&mut zstd_reader)
                }
            })
            .map_err(|e| {
                hyerror!(
                    instance,
                    "Failed to read compiled module storage header: {}",
                    e
                );
                HyError::Unknown(format!(
                    "Failed to read compiled module storage header: {}",
                    e
                ))
            })
    }
}

pub fn compile_sources(
    instance: &InstanceContext,
    compile_info: ModuleCompileInfo,
) -> HyResult<Vec<u8>> {
    let mut module = Module::default();

    // Notice, we used a fresh type registry for compilation, not the instance's registry
    // because we want to avoid polluting it with temporary types.
    let type_registry = TypeRegistry::new([0u8; 6]);
    let mut filenames = Vec::new();

    // Compile each source in the compile_info
    for source_info in compile_info.sources {
        hytrace!(
            instance,
            "Compiling source \"{}\"",
            source_info.filename.as_deref().unwrap_or("<unnamed>")
        );

        match source_info.source_type {
            ModuleSourceType::Assembly => {
                // Compile assembly source code into the module
                extend_module_from_string(&mut module, &type_registry, &source_info.data)
                    .inspect_err(|e| {
                        hyerror!(
                            instance,
                            "Failed to compile assembly source \"{}\": {}",
                            source_info.filename.as_deref().unwrap_or("<unnamed>"),
                            e
                        );
                    })?;
            }
        }

        if let Some(filename) = source_info.filename {
            filenames.push(filename);
        }
    }

    // Verify and type check the module
    hytrace!(instance, "Verifying compiled module");
    module.verify().inspect_err(|e| {
        hyerror!(instance, "Module verification failed: {}", e);
    })?;

    hytrace!(instance, "Type checking compiled module");
    for func in module.functions.values() {
        hytrace!(
            instance,
            "Type checking function '{}'",
            func.name
                .clone()
                .unwrap_or_else(|| format!("@{}", func.uuid))
        );
        func.type_check(&type_registry).inspect_err(|e| {
            hyerror!(
                instance,
                "Type check failed for function '{}': {}",
                func.name
                    .clone()
                    .unwrap_or_else(|| format!("@{}", func.uuid)),
                e
            );
        })?;
    }

    // Produce compiled module storage or further processing here
    let storage = CompiledModuleStorage {
        module,
        type_registry,
        filenames,
    };
    let encoded_storage = storage.encode(instance)?;

    // Information about the compiled module can be used here
    hyinfo!(
        instance,
        "Compiled successful, module has {} functions: {:?}",
        storage.module.functions.len(),
        storage
            .module
            .functions
            .values()
            .map(|x| x.name.clone().unwrap_or_else(|| format!("@{}", x.uuid)))
            .collect::<Vec<_>>()
    );
    hyinfo!(
        instance,
        "Produced {} bytes of compiled module storage",
        encoded_storage.len()
    );

    Ok(encoded_storage)
}

pub fn load_module(instance: &Arc<InstanceContext>, data: &[u8]) -> HyResult<ModuleKey> {
    let storage = CompiledModuleStorage::decode(instance, data)?;
    hytrace!(
        instance,
        "Loaded compiled module with {} functions from {} bytes",
        storage.module.functions.len(),
        data.len()
    );
    hytrace!(
        instance,
        "Module originally compiled from: {}",
        storage.filenames.join(", ")
    );

    // 1. Merge type registry, construct table mapping old to new type IDs
    hytrace!(
        instance,
        "Merging type registry ({} types) into instance's registry ({} types)",
        storage.type_registry.len(),
        instance.type_registry.len()
    );
    let mapping = instance.type_registry.merge_with(&storage.type_registry);

    // 2. Remap types in module using the mapping
    let mut module = storage.module;
    module.remap_types(&mapping);

    // 3. Add module to instance's module list
    instance.add_module(module)
}
