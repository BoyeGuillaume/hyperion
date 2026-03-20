use hycore::{
    api::{
        ApplicationInfo, InstanceCreateInfo, constants::HYCORE_LOGGER_PLUGIN_NAME,
        hy_create_instance,
    },
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
    println!("Hello, world!");
    let _instance = hy_create_instance(InstanceCreateInfo {
        application_info: ApplicationInfo {
            application_name: "HyCore Example".into(),
            application_version: "0.1.0".parse().unwrap(),
            engine_name: None,
            engine_version: None,
        },
        enabled_plugins: vec![HYCORE_LOGGER_PLUGIN_NAME.into()],
        node_rank: 0,
        ext: (LoggerPluginCreateInfo {
            level: hycore::plugin::logger::LoggerLevel::Debug,
            sink_callback: Box::new(log_fn),
        },)
            .into(),
    })
    .expect("Failed to create instance");
}
