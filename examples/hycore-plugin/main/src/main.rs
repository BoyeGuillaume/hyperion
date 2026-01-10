use std::path::PathBuf;

use hycore::{
    base::{ext::load_plugin_by_name, meta::HyperionMetaInfo},
    utils::conf::ExtList,
};
use semver::Version;

fn main() {
    let path = PathBuf::from("./test_plugin.toml");
    println!("Loading meta info from {:?}", path);
    let meta_info = HyperionMetaInfo::load_from_toml(&path).unwrap();
    let version = Version::parse("0.1.0").unwrap();

    println!("Available plugins:");
    for ext in &meta_info.ext {
        println!("- {} (UUID: {}) at {}", ext.name, ext.uuid, ext.path);
    }

    let plugin = unsafe {
        load_plugin_by_name(
            &meta_info,
            "__EXT_PLUGIN_EXAMPLE",
            version,
            &mut ExtList(vec![]),
        )
        .unwrap()
    };
    println!(
        "Loaded plugin: {} (UUID: {}, Version: {})",
        plugin.name(),
        plugin.uuid(),
        plugin.version()
    );

    // let mut meta_info = HyperionMetaInfo::default();
    // for _ in 0..3 {
    //     let plugin = ExtMetaInfo {
    //         uuid: Uuid::new_v4(),
    //         version: Version::parse("0.2.51").unwrap(),
    //         compatible_version: VersionReq::parse(">=0.1.0").unwrap(),
    //         path: "./some/plugin/path".to_string(),
    //         name: "Example Plugin".to_string(),
    //     };

    //     meta_info.ext.push(plugin);
    // }

    // meta_info.save_to_toml("./test.toml").unwrap();
}
