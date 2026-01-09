# Hyperion Plugin System

This document explains how Hyperion discovers, loads, and interacts with external plugins. It captures the conventions introduced in the latest plugin work so that you can build compatible extensions, ship them, and consume them from Hyperion-based tools.

## High-level architecture

- **Discovery**: Plugin metadata lives in a TOML file (usually `~/.config/hyperion/meta.toml`). Each `[[ext]]` entry specifies a UUID, a human-friendly name, and the absolute or relative path to the compiled dynamic library. Set the `HY_CONFIG_PATH` environment variable to override the lookup path.
- **Loading**: `load_plugin_by_name()` deserializes the metadata using `HyperionMetaInfo`, locates the library via `libloading`, runs a compatibility check, and then asks the plugin-specific loader to build the concrete type. The plugin value is wrapped in `PluginExtWrapper` to keep the `Library` alive for as long as the extension instance exists.
- **Extensibility surface**: Plugins implement two traits. `PluginExt` exposes runtime metadata (UUID, version, name, description). `PluginExtStatic` adds the static UUID constant and a `new()` constructor that the loader uses to instantiate the plugin.
- **Version gating**: Every plugin exports a compatibility function that returns a `semver::VersionReq`. Hyperion compares it against the library version supplied by the host. Loading aborts if the requirement does not match, producing `HyError::CompatibilityCheckFailed`.

## Module reference: `hycore/src/base/ext.rs`

- **Macros**: `define_plugin_compatibility!` emits the `__hyext_fn_compatibility_check` symbol that returns a `VersionReq`, while `define_plugin_loader!` wires UUIDs to concrete `PluginExtStatic` implementors and exposes the `__hyext_fn_loader` entry point expected by the host.
- **Traits**: `PluginExt` defines the runtime contract (UUID, semantic version, name, description). `PluginExtStatic` extends it with the compile-time UUID constant and a `new()` constructor used solely by the loader.
- **Wrapper**: `PluginExtWrapper` keeps an `Arc<Library>` next to the boxed plugin to guarantee the shared object stays loaded for the lifetime of the plugin instance.
- **Loader**: `load_plugin_by_name()` glues together metadata lookup, symbol resolution, semantic version checks, and instantiation. Failures bubble up as the `HyError` variants defined in `hycore/src/utils/error.rs`.

## Writing a plugin

1. **Create a `cdylib` crate**. In `Cargo.toml`, set `crate-type = ["cdylib"]` and depend on `hycore`, `semver`, and `uuid`.
2. **Define the plugin type**. Implement both `PluginExt` and `PluginExtStatic` so Hyperion can describe and instantiate your plugin.
3. **Expose the required entry points** using the provided macros:

   ```rust
   use hycore::{
       base::ext::{PluginExt, PluginExtStatic},
       define_plugin_compatibility, define_plugin_loader,
   };
   use semver::Version;
   use uuid::{Uuid, uuid};

   define_plugin_compatibility!(">=0.1.0");
   define_plugin_loader!(MyPlugin);

   pub struct MyPlugin {
       version: Version,
   }

   impl PluginExtStatic for MyPlugin {
       const UUID: Uuid = uuid!("a8af402c-7892-4b7f-9aa1-ca4b9bd47c94");

       fn new() -> Self {
           Self { version: Version::parse("0.2.3").unwrap() }
       }
   }

   impl PluginExt for MyPlugin {
       fn uuid(&self) -> Uuid { Self::UUID }
       fn version(&self) -> &Version { &self.version }
       fn name(&self) -> &str { "__EXT_PLUGIN_EXAMPLE" }
       fn description(&self) -> &str { "An example plugin extension." }
   }
   ```

   `define_plugin_compatibility!` emits the `__hyext_fn_compatibility_check` symbol, while `define_plugin_loader!` publishes `__hyext_fn_loader` capable of spawning one or more plugin types.
4. **Build the dynamic library** via `cargo build --release`. Record the resulting `.so`, `.dylib`, or `.dll` path.

See the complete reference implementation in [examples/hycore-plugin/plugin/src/lib.rs](examples/hycore-plugin/plugin/src/lib.rs).

## Registering plugins

Add, edit, or generate the metadata file read by `HyperionMetaInfo::load_from_toml()`:

```toml
[[ext]]
uuid = "a8af402c-7892-4b7f-9aa1-ca4b9bd47c94"
path = "/absolute/path/to/libmy_plugin.so"
name = "__EXT_PLUGIN_EXAMPLE"
```

Key field semantics:

- `uuid`: Must match `PluginExtStatic::UUID`. Hyperion verifies this at load time.
- `path`: Any path libloading can open. Relative paths are resolved from the process working directory.
- `name`: Used by `load_plugin_by_name()` and for user-facing listings.

Use `HyperionMetaInfo::save_to_toml()` to write the file programmatically or keep one committed alongside your application (see [examples/hycore-plugin/main/test_plugin.toml](examples/hycore-plugin/main/test_plugin.toml)).

## Loading from host applications

A host integrates the plugin runtime with a handful of calls:

```rust
use hycore::base::{ext::load_plugin_by_name, meta::HyperionMetaInfo};
use semver::Version;

let meta = HyperionMetaInfo::load_from_toml("./test_plugin.toml".as_ref())?;
let host_version = Version::parse("0.1.0")?;
let plugin = load_plugin_by_name(&meta, "__EXT_PLUGIN_EXAMPLE", host_version)?;
println!("Loaded {} v{}", plugin.name(), plugin.version());
```

This routine will:

- Display any `HyError::ExtensionLoadError` if the shared object cannot be opened or does not export the expected symbols.
- Reject the plugin when `compatibility_req.matches(host_version)` evaluates to `false`.
- Keep the shared object pinned in memory via `PluginExtWrapper`, so references to vtables stay valid for the life of `plugin`.

## Best practices and troubleshooting

- **Stabilize UUIDs**: Treat the UUID as the plugin’s identity. Generating a new UUID implies a distinct plugin.
- **Follow semantic versioning**: The compatibility check can express ranges like `"^0.1"` or `"<=1.2"`; choose requirements that reflect real API expectations.
- **Bundle diagnostics**: Implement `description()` with actionable text (e.g., prerequisites, expected host modules).
- **Graceful updates**: Ship a new shared object, update the metadata entry (or distribute multiple entries with different names), and allow hosts to pick the appropriate plugin at runtime.
- **Security**: `load_plugin_by_name()` executes arbitrary code from the shared object constructors. Only load trusted libraries and consider sandboxing when integrating third-party plugins.

With these conventions, plugin authors can safely extend Hyperion, and host applications gain a consistent way to discover and orchestrate new capabilities without recompiling the core runtime.
