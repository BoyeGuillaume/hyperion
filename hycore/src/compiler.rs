use std::{path::Path, sync::Arc};

use borsh::{BorshDeserialize, BorshSerialize};
use hyinstr::{
    modules::{
        Module,
        parser::{extend_module_from_paths, extend_module_from_strings},
    },
    types::TypeRegistry,
};
use smallvec::{SmallVec, smallvec};

use crate::{
    base::{
        InstanceContext, ModuleKey,
        api::{ModuleCompileFlags, ModuleCompileInfo, ModuleSourceType},
    },
    hyerror, hyinfo, hytrace, hywarn,
    utils::error::{HyError, HyResult},
};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct CompiledModuleStorage {
    pub filenames: Vec<String>,
    pub module: Module,
    pub type_registry: TypeRegistry,
}

struct CompiledModuleStorageHeaderFlags {
    zstd_enabled: bool,
}

impl CompiledModuleStorage {
    /// Distinguishing magic bytes for compiled module storage files
    pub const MAGIC_BYTES_NOZSTD: [u8; 8] = *b"\x80HYMODIR";
    pub const MAGIC_BYTES_ZSTD: [u8; 8] = *b"\x7FHYMODIR";

    fn writer_header<W: std::io::Write>(
        &self,
        instance: &InstanceContext,
        writer: &mut W,
        zstd_enabled: bool,
    ) -> std::io::Result<()> {
        // Write magic bytes
        if zstd_enabled {
            writer.write_all(&Self::MAGIC_BYTES_ZSTD)?;
        } else {
            writer.write_all(&Self::MAGIC_BYTES_NOZSTD)?;
        }

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
    ) -> std::io::Result<CompiledModuleStorageHeaderFlags> {
        // Read and verify magic bytes
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        let zstd_enabled = if magic == Self::MAGIC_BYTES_ZSTD {
            true
        } else if magic == Self::MAGIC_BYTES_NOZSTD {
            false
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid magic bytes in compiled module storage",
            ));
        };

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

        Ok(CompiledModuleStorageHeaderFlags { zstd_enabled })
    }

    pub fn encode(&self, instance: &InstanceContext, zstd_enabled: bool) -> HyResult<Vec<u8>> {
        // Serialize inner using borsh
        hytrace!(
            instance,
            "Serializing compiled module storage (module has {} functions)",
            self.module.functions.len()
        );
        let mut buf = Vec::new();
        let mut writer = &mut buf;

        self.writer_header(instance, &mut writer, zstd_enabled)
            .and_then(|_| {
                if zstd_enabled {
                    hytrace!(
                        instance,
                        "Compressing compiled module storage with zstd encoder (level 3)"
                    );
                    let mut zstd_writer = zstd::stream::write::Encoder::new(writer, 3).unwrap();
                    borsh::BorshSerialize::serialize(&self, &mut zstd_writer)?;
                    zstd_writer.finish()?;
                } else {
                    hytrace!(
                        instance,
                        "Not compressing compiled module storage, writing directly"
                    );
                    borsh::BorshSerialize::serialize(&self, &mut writer)?;
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
            .and_then(|header_flags| {
                if header_flags.zstd_enabled {
                    hytrace!(
                        instance,
                        "Compiled module storage is zstd-compressed, decompressing with zstd decoder"
                    );
                    let mut zstd_reader = zstd::stream::read::Decoder::new(reader).unwrap();
                    borsh::BorshDeserialize::deserialize_reader(&mut zstd_reader)
                } else {
                    hytrace!(
                        instance,
                        "Compiled module storage is not compressed, deserializing directly"
                    );
                    borsh::BorshDeserialize::deserialize_reader(&mut reader)
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

    // Either each source contains source code as a string, or a filename + base path is provided to read the source code
    // from disk
    let some_sources_missing_data = compile_info
        .sources
        .iter()
        .any(|source| source.data.is_none());
    let all_sources_missing_data = compile_info
        .sources
        .iter()
        .all(|source| source.data.is_none());

    if compile_info
        .sources
        .iter()
        .any(|source| source.data.is_none() && source.filename.is_none())
    {
        hyerror!(
            instance,
            "Cannot compile sources: some sources are missing both data and filename"
        );
        return Err(HyError::Unknown(
            "Cannot compile sources: some sources are missing both data and filename".to_string(),
        ));
    }

    if some_sources_missing_data != all_sources_missing_data {
        hyerror!(
            instance,
            "Cannot compile sources: some sources are missing data while others have data, this is not allowed"
        );
        return Err(HyError::Unknown(
            "Cannot compile sources: some sources are missing data while others have data, this is not allowed".to_string(),
        ));
    }

    // Compile each source in the compile_info,
    // Note: this approach does not allow to mix source type but
    // if in the future i ever get to this point, changing it wouldn't
    // be too hard, simply add just-compiled function (prior to referece checking) in a sort of list
    // for each compilation type and call it a day
    if all_sources_missing_data {
        // Read source code from disk using the provided base path and filenames
        let base_path = if let Some(base_path) = compile_info.base_path.as_ref() {
            hytrace!(
                instance,
                "Compiling sources from disk with provided base path '{}'",
                base_path
            );
            let base_path = std::path::PathBuf::from(base_path);
            base_path.canonicalize().map_err(|e| {
                hyerror!(
                    instance,
                    "Failed to canonicalize base path '{}': {}",
                    base_path.display(),
                    e
                );
                HyError::Unknown(format!(
                    "Failed to canonicalize base path '{}': {}",
                    base_path.display(),
                    e
                ))
            })?
        } else {
            let base_path = std::path::PathBuf::from(std::env::current_dir().map_err(|e| {
                hyerror!(
                    instance,
                    "Failed to get current working directory for base path: {}",
                    e
                );
                HyError::Unknown(format!(
                    "Failed to get current working directory for base path: {}",
                    e
                ))
            })?);
            hywarn!(
                instance,
                "No base path provided for compiling sources from disk, using current working directory '{}'",
                base_path.display()
            );
            base_path
        };

        let mut initial_source_infos = Vec::new();
        for source_info in compile_info.sources {
            assert!(source_info.source_type == ModuleSourceType::Assembly);
            let filename = std::path::PathBuf::from(source_info.filename.as_ref().unwrap());
            if filename.is_absolute() {
                initial_source_infos.push(filename);
            } else {
                let full_path = base_path.join(&filename).canonicalize().map_err(|e| {
                    hyerror!(
                        instance,
                        "Failed to canonicalize path for source file '{}': {}",
                        filename.display(),
                        e
                    );
                    HyError::Unknown(format!(
                        "Failed to canonicalize path for source file '{}': {}",
                        filename.display(),
                        e
                    ))
                })?;
                initial_source_infos.push(full_path);
            }
        }

        // Call the compilation function for each source, with error handling that includes the filename
        hytrace!(
            instance,
            "Compiling sources from disk with base path '{}'",
            base_path.display()
        );
        extend_module_from_paths(
            &mut module,
            &type_registry,
            initial_source_infos.into_iter(),
            Some(|path: &Path| {
                hytrace!(
                    instance,
                    "Reading source file for compilation from path '{}'",
                    path.display()
                );
                filenames.push(path.to_string_lossy().to_string());
            }),
        )
        .inspect_err(|e| {
            hyerror!(
                instance,
                "Failed to compile sources from disk with base path '{}': {}",
                base_path.display(),
                e
            );
        })
        .map_err(|e| {
            HyError::Unknown(format!(
                "Failed to compile sources from disk with base path '{}': {}",
                base_path.display(),
                e
            ))
        })?;
    } else {
        // Compile source code directly from the provided strings
        hytrace!(instance, "Compiling sources from provided strings");
        let mut contents: SmallVec<String, 2> = smallvec!();
        for source_info in compile_info.sources {
            assert!(source_info.source_type == ModuleSourceType::Assembly);

            if let Some(filename) = source_info.filename {
                filenames.push(filename);
            }

            contents.push(source_info.data.as_deref().unwrap().to_string());
        }

        extend_module_from_strings(
            &mut module,
            &type_registry,
            contents.iter().map(|s| s.as_str()),
        )
        .inspect_err(|e| {
            hyerror!(
                instance,
                "Failed to compile sources from provided strings: {}",
                e
            );
        })
        .map_err(|e| {
            HyError::Unknown(format!(
                "Failed to compile sources from provided strings: {}",
                e
            ))
        })?;
    }
    // for source_info in compile_info.sources {
    //     hytrace!(
    //         instance,
    //         "Compiling source \"{}\"",
    //         source_info.filename.as_deref().unwrap_or("<unnamed>")
    //     );

    //     match source_info.source_type {
    //         ModuleSourceType::Assembly => {
    //             // Compile assembly source code into the module
    //             extend_module_from_string(&mut module, &type_registry, &source_info.data)
    //                 .inspect_err(|e| {
    //                     hyerror!(
    //                         instance,
    //                         "Failed to compile assembly source \"{}\": {}",
    //                         source_info.filename.as_deref().unwrap_or("<unnamed>"),
    //                         e
    //                     );
    //                 })?;
    //         }
    //     }

    //     if let Some(filename) = source_info.filename {
    //         filenames.push(filename);
    //     }
    // }

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
    let encoded_storage = storage.encode(
        instance,
        compile_info
            .flags
            .contains(ModuleCompileFlags::ZSTD_COMPRESSED),
    )?;

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
