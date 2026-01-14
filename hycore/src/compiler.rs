use borsh::{BorshDeserialize, BorshSerialize};
use hyinstr::{
    modules::{Module, parser::extend_module_from_string},
    types::TypeRegistry,
};
use log::info;

use crate::{
    base::{
        InstanceContext,
        api::{ModuleCompileInfo, ModuleSourceType},
    },
    hyerror, hytrace,
    utils::error::{HyError, HyResult},
};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompiledModuleStorage {
    pub filenames: Vec<String>,
    pub module: Module,
}

impl CompiledModuleStorage {
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
                let mut zstd_writer = zstd::stream::write::Encoder::new(writer, 3).unwrap();
                borsh::BorshSerialize::serialize(&self, &mut zstd_writer)?;
                zstd_writer.finish()?;
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

    pub fn decode(&self, instance: &InstanceContext, data: &[u8]) -> HyResult<Self> {
        hytrace!(
            instance,
            "Deserializing compiled module storage ({} bytes)",
            data.len()
        );

        let mut reader = &data[..];
        Self::read_header(instance, &mut reader)
            .and_then(|_| {
                let mut zstd_reader = zstd::stream::read::Decoder::new(reader).unwrap();
                borsh::BorshDeserialize::deserialize_reader(&mut zstd_reader)
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
    let storage = CompiledModuleStorage { module, filenames };
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
