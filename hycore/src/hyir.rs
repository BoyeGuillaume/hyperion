use std::{
    borrow::Cow,
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow, bail};
use borsh::{BorshDeserialize, BorshSerialize};
use hyinstr::{
    modules::{Module, parser::extend_module_from_callbacks},
    types::TypeRegistry,
};

use crate::{
    HyResult,
    api::{ModuleCompileInfo, ModuleCompileInfoFlags},
    hyinfo, hytrace, hywarn,
    instance::{Instance, core::ModuleHandle},
};

// This struct represents the on-disk format of a compiled module, which includes the module itself,
#[derive(BorshSerialize, BorshDeserialize)]
struct CompiledModuleStorage {
    filenames: Vec<String>,
    module: Module,
    type_registry: TypeRegistry,
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
        writer: &mut W,
        header: CompiledModuleStorageHeaderFlags,
    ) -> std::io::Result<()> {
        // Write magic bytes
        if header.zstd_enabled {
            writer.write_all(&Self::MAGIC_BYTES_ZSTD)?;
        } else {
            writer.write_all(&Self::MAGIC_BYTES_NOZSTD)?;
        }

        // Write version requirement (using semver format)
        let version = Instance::library_version();
        let version_req = semver::VersionReq {
            comparators: vec![semver::Comparator {
                op: semver::Op::Exact,
                major: version.major,
                minor: Some(version.minor),
                patch: Some(version.patch),
                pre: version.pre.clone(),
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

    fn read_header<R: std::io::Read>(reader: &mut R) -> HyResult<CompiledModuleStorageHeaderFlags> {
        // Read and verify magic bytes
        let mut magic = [0u8; 8];
        reader
            .read_exact(&mut magic)
            .with_context(|| "Failed to read magic bytes")?;
        let zstd_enabled = if magic == Self::MAGIC_BYTES_ZSTD {
            true
        } else if magic == Self::MAGIC_BYTES_NOZSTD {
            false
        } else {
            anyhow::bail!(
                "Invalid magic bytes in compiled module storage. Expected either {:?} or {:?}, found {:?}",
                Self::MAGIC_BYTES_NOZSTD,
                Self::MAGIC_BYTES_ZSTD,
                magic
            );
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
        let version_req_str = String::from_utf8(version_req_bytes)
            .with_context(|| "Failed to parse version requirement string")?;
        let version_req = semver::VersionReq::parse(&version_req_str).with_context(|| {
            format!(
                "Invalid version requirement string: \"{}\"",
                version_req_str
            )
        })?;

        // Check version compatibility
        let version = Instance::library_version();
        if !version_req.matches(&version) {
            bail!(
                "Incompatible compiled module storage version: expected {}, but got {}",
                version_req,
                version
            );
        }

        Ok(CompiledModuleStorageHeaderFlags { zstd_enabled })
    }

    pub fn encode(&self, instance: &Instance, zstd_enabled: bool) -> HyResult<Vec<u8>> {
        // Serialize inner using borsh
        hytrace!(
            instance;
            "Serializing compiled module storage (module has {} functions)",
            self.module.functions.len()
        );
        let mut buf = Vec::new();
        let mut writer = &mut buf;

        self.writer_header(
            &mut writer,
            CompiledModuleStorageHeaderFlags { zstd_enabled },
        )
        .and_then(|_| {
            if zstd_enabled {
                hytrace!(
                    instance;
                    "Compressing compiled module storage with zstd encoder (level 3)"
                );
                let mut zstd_writer = zstd::stream::write::Encoder::new(writer, 3).unwrap();
                borsh::BorshSerialize::serialize(&self, &mut zstd_writer)?;
                zstd_writer.finish()?;
            } else {
                hytrace!(
                    instance;
                    "Not compressing compiled module storage, writing directly"
                );
                borsh::BorshSerialize::serialize(&self, &mut writer)?;
            }

            Ok(())
        })
        .with_context(|| "Failed to serialize compiled module storage header")?;

        Ok(buf)
    }

    pub fn decode(instance: &Instance, data: &[u8]) -> HyResult<Self> {
        hytrace!(
            instance;
            "Deserializing compiled module storage ({} bytes)",
            data.len()
        );

        let mut reader = data;
        Self::read_header(&mut reader).and_then(|header_flags| {
            if header_flags.zstd_enabled {
                hytrace!(
                    instance;
                    "Compiled module storage is zstd-compressed, decompressing with zstd decoder"
                );
                let mut zstd_reader = zstd::stream::read::Decoder::new(reader).unwrap();
                Ok(borsh::BorshDeserialize::deserialize_reader(&mut zstd_reader)?)
            } else {
                hytrace!(
                    instance;
                    "Compiled module storage is not compressed, deserializing directly"
                );
                Ok(borsh::BorshDeserialize::deserialize_reader(&mut reader)?)
            }
        })
        .with_context(|| "Failed to deserialize compiled module storage")
    }
}

pub fn compile_sources(instance: &Instance, compile_info: ModuleCompileInfo) -> HyResult<Vec<u8>> {
    let type_registry = TypeRegistry::new(instance.type_registry().node_id());
    let mut module = Module::default();

    if compile_info
        .source_descriptors
        .iter()
        .any(|source| source.data.is_none() && source.filename.is_none())
    {
        anyhow::bail!("Cannot compile sources: some sources are missing both data and filename");
    }

    // Determine the base path for resolving source files when compiling from disk, if needed
    let base_path = if let Some(base_path) = compile_info.base_path.as_ref() {
        hytrace!(
            instance;
            "Compiling sources from disk with provided base path '{}'",
            base_path.display()
        );
        base_path.canonicalize().map_err(|e| {
            anyhow::anyhow!(
                "Failed to canonicalize base path '{}'. {}",
                base_path.display(),
                e
            )
        })?
    } else {
        let base_path = std::path::PathBuf::from(std::env::current_dir().map_err(|e| {
            anyhow::anyhow!(
                "Failed to get current working directory for base path. {}",
                e
            )
        })?);
        hywarn!(
            instance;
            "No base path provided for compiling sources from disk, using current working directory '{}'",
            base_path.display()
        );
        base_path
    };

    // Compile each source in the compile_info
    #[derive(PartialEq, Eq)]
    enum SourceInfoType {
        Data(String, Option<PathBuf>),
        Filename(PathBuf),
    }

    impl std::fmt::Display for SourceInfoType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SourceInfoType::Data(_, None) => write!(f, "<source data>"),
                SourceInfoType::Data(_, Some(path)) | SourceInfoType::Filename(path) => {
                    write!(f, "{}", path.display())
                }
            }
        }
    }

    let mut initial_source_infos = Vec::new();
    for source_info in compile_info.source_descriptors.into_iter() {
        let full_path = if let Some(filename) = source_info.filename {
            if filename.is_absolute() {
                Some(filename)
            } else {
                Some(base_path.join(&filename).canonicalize().map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to canonicalize path for source file '{}': {}",
                        filename.display(),
                        e
                    )
                })?)
            }
        } else {
            None
        };

        if let Some(data) = source_info.data {
            initial_source_infos.push(SourceInfoType::Data(data, full_path));
        } else if let Some(filename) = full_path {
            initial_source_infos.push(SourceInfoType::Filename(filename));
        } else {
            unreachable!(
                "Already checked that all sources have either data or filename, so this branch should never be hit"
            )
        }
    }

    // Call the compilation function for each source, with error handling that includes the filename
    let mut filenames = HashSet::new();
    let merger = |current: &SourceInfoType, path: String| {
        // Retrieve the path of the base path of the current source
        let current_path = match current {
            SourceInfoType::Data(_, Some(path)) | SourceInfoType::Filename(path) => path,
            SourceInfoType::Data(_, None) => {
                return Err(hyinstr::utils::Error::IllegalState(format!(
                    "Cannot use include directives in source {} because no filename is associated with it",
                    current
                )));
            }
        };

        // Resolve the included path relative to the current source's base path
        let included_path = current_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&path)
            .canonicalize()
            .map_err(|e| {
                hyinstr::utils::Error::FileNotFound(format!(
                    "Failed to resolve included path '{}' relative to '{}': {}",
                    path,
                    current_path.display(),
                    e
                ))
            })?;

        // Finally, convert the included path to a string and return it
        Ok(SourceInfoType::Filename(included_path))
    };

    // For debugging purposes, create a string representation of the initial source infos to include in error messages
    let initial_source_infos_fmt = initial_source_infos
        .iter()
        .map(|info| info.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    extend_module_from_callbacks(
        &mut module,
        &type_registry,
        initial_source_infos.into_iter(),
        |source_info| match source_info {
            SourceInfoType::Data(data, path_buf) => {
                if let Some(path_buf) = path_buf {
                    hytrace!(
                        instance;
                        "Compiling source from provided data with associated filename '{}'",
                        path_buf.display()
                    );
                    if !filenames.insert(path_buf.clone()) {
                        return Ok(None);
                    }
                } else {
                    hytrace!(
                        instance;
                        "Compiling source from provided data with no associated filename"
                    );
                }

                let data = data.clone();
                Ok(Some(Cow::Owned(data)))
            }
            SourceInfoType::Filename(path_buf) => {
                if !filenames.insert(path_buf.clone()) {
                    return Ok(None);
                }

                // Read the source file content and return it as a Cow
                hytrace!(
                    instance;
                    "Reading source file for compilation from path '{}'",
                    path_buf.display()
                );
                let content = std::fs::read_to_string(&path_buf).map_err(|e| {
                    hyinstr::utils::Error::FileNotFound(format!(
                        "Failed to read the file {}: {}",
                        path_buf.display(),
                        e
                    ))
                })?;

                Ok(Some(Cow::Owned(content)))
            }
        },
        Some(merger),
    )
    .with_context(|| {
        format!(
            "Failed to compile sources with initial source info: {}",
            initial_source_infos_fmt
        )
    })?;

    // Verify and type check the module
    hytrace!(instance; "Verifying compiled module");
    module.verify().with_context(|| {
        anyhow!(
            "Module verification failed for compiled module with sources: {:?}",
            filenames
        )
    })?;

    hytrace!(instance; "Type checking compiled module");
    for func in module.functions.values() {
        hytrace!(
            instance;
            "Type checking function '{}'",
            func.name
                .clone()
                .unwrap_or_else(|| format!("@{}", func.uuid))
        );
        func.type_check(&type_registry).with_context(|| {
            anyhow!(
                "In function '{}'",
                func.name
                    .clone()
                    .unwrap_or_else(|| format!("@{}", func.uuid))
            )
        })?;
    }

    hytrace!(instance; "Type checking compiled global");
    for global in module.globals.values() {
        hytrace!(
            instance;
            "Type checking global '{}'",
            global.name
                .clone()
                .unwrap_or_else(|| format!("@{}", global.uuid))
        );
        global.type_check(&type_registry).with_context(|| {
            anyhow!(
                "In global '{}'",
                global
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("@{}", global.uuid))
            )
        })?;
    }

    // Produce compiled module storage or further processing here
    let storage = CompiledModuleStorage {
        module,
        type_registry,
        filenames: filenames
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    };
    let encoded_storage = storage.encode(
        instance,
        compile_info
            .flags
            .contains(ModuleCompileInfoFlags::ZSTD_COMPRESSION),
    )?;

    // Information about the compiled module can be used here
    hyinfo!(
        instance;
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
        instance;
        "Produced {} bytes of compiled module storage",
        encoded_storage.len()
    );

    Ok(encoded_storage)
}

pub fn load_compiled_module(instance: &mut Instance, data: &[u8]) -> HyResult<ModuleHandle> {
    let storage = CompiledModuleStorage::decode(instance, data).with_context(|| {
        format!("Failed to load module, probably due to corruption or version mismatch")
    })?;
    hytrace!(
        instance;
        "Loaded compiled module with {} functions from {} bytes",
        storage.module.functions.len(),
        data.len()
    );
    hytrace!(
        instance;
        "Module originally compiled from: {:?}",
        storage.filenames
    );

    // 1. Merge type registry, construct table mapping old to new type IDs
    hytrace!(
        instance;
        "Merging type registry ({} types) into instance's registry ({} types)",
        storage.type_registry.len(),
        instance.type_registry().len()
    );
    let mapping = instance.type_registry().merge_with(&storage.type_registry);

    // 2. Remap types in module using the mapping
    let mut module = storage.module;
    module.remap_types(&mapping);

    // 3. Add module to instance's module list
    instance.add_module(module).with_context(|| {
        format!("Failed to load module, probably due to corruption or version mismatch")
    })
}

// pub fn load_module(instance: &mut Instance, data: &[u8]) -> HyResult<ModuleKey> {
//     //     let storage = CompiledModuleStorage::decode(instance, data)?;
//     //     hytrace!(
//     //         instance,
//     //         "Loaded compiled module with {} functions from {} bytes",
//     //         storage.module.functions.len(),
//     //         data.len()
//     //     );
//     //     hytrace!(
//     //         instance,
//     //         "Module originally compiled from: {}",
//     //         storage.filenames.join(", ")
//     //     );

//     //     // 1. Merge type registry, construct table mapping old to new type IDs
//     //     hytrace!(
//     //         instance,
//     //         "Merging type registry ({} types) into instance's registry ({} types)",
//     //         storage.type_registry.len(),
//     //         instance.type_registry.len()
//     //     );
//     //     let mapping = instance.type_registry.merge_with(&storage.type_registry);

//     //     // 2. Remap types in module using the mapping
//     //     let mut module = storage.module;
//     //     module.remap_types(&mapping);

//     //     // 3. Add module to instance's module list
//     //     instance.add_module(module)
// }
