use hycore::{
    api::{
        ApplicationInfo, InstanceCreateInfo, ModuleCompileInfo, ModuleCompileInfoFlags,
        ModuleCompileInfoSourceDescriptor,
        constants::{HYCORE_LOGGER_PLUGIN_NAME, HYCORE_REMOTE_PLUGIN_NAME},
        hy_compile_module, hy_create_instance, hy_load_compiled_module, hy_run,
    },
    ext::ExtList,
    plugin::logger::{LoggerPluginCreateInfo, LoggerRecord},
};

fn log_fn(record: LoggerRecord) {
    println!(
        "{} [{}:{}] [{}] {} -- {}",
        record.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
        record.file.as_deref().unwrap_or("<unknown>"),
        record.line.unwrap_or(0),
        record.module_path.as_deref().unwrap_or("<unknown>"),
        record.level,
        record.message
    );
}

fn main() {
    let mut instance = hy_create_instance(InstanceCreateInfo {
        application_info: ApplicationInfo {
            application_name: "HyCore Example".into(),
            application_version: "0.1.0".parse().unwrap(),
            engine_name: None,
            engine_version: None,
        },
        enabled_plugins: vec![
            HYCORE_LOGGER_PLUGIN_NAME.into(),
            HYCORE_REMOTE_PLUGIN_NAME.into(),
        ],
        node_rank: 0,
        ext: (LoggerPluginCreateInfo {
            level: hycore::plugin::logger::LoggerLevel::Debug,
            sink_callback: Box::new(log_fn),
        },)
            .into(),
    })
    .expect("Failed to create instance");

    let compiled_data = hy_compile_module(
        &instance,
        ModuleCompileInfo {
            base_path: None,
            source_descriptors: vec![ModuleCompileInfoSourceDescriptor {
                data: None,
                filename: Some("examples/library/all.hyir".into()),
            }],
            flags: ModuleCompileInfoFlags::ZSTD_COMPRESSION,
            ext: ExtList::new(),
        },
    )
    .unwrap();

    let _module = hy_load_compiled_module(&mut instance, &compiled_data).unwrap();

    hy_run(&mut instance);

    // hy_destroy_module(&mut instance, module).unwrap();
}
