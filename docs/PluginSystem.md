# Hyperion Plugin System

The `hycore::base::ext` module is the backbone of Hyperion’s extensibility story. It exposes
the trait contracts, lifecycle hooks, and hosting utilities that enable user-defined DLLs to
participate in an instance. This document augments the in-line documentation with a narrative
walkthrough so you can design, ship, and troubleshoot extensions confidently.

## Runtime anatomy

The major moving pieces live in [hycore/src/base/ext.rs](hycore/src/base/ext.rs):

- **Metadata (`HyperionMetaInfo`)** records every known extension (name, UUID, path). The
    `load_or_generate()` helper caches the last `HY_LD_PATH` snapshot (falling back to an
    `extensions/` directory next to `meta.toml` when the env var is unset) and repopulates the
    metadata file whenever the discovery paths change or a requested plugin is missing by scanning
    every `hy*` shared object exposed via the descriptor symbol described below.
- **Loader (`load_plugin_by_name`)** handles the full lifecycle: open the dynamic library, run the
    compatibility check, and invoke the plugin-specific constructor exported by the shared object.
- **Traits** define the extensibility surface:
  - `PluginExt` describes runtime behavior (UUID, version, name, `attach_to`, `initialize`,
        `teardown`).
  - `PluginExtStatic` adds the compile-time UUID constant and the `new(ext: &mut ExtList)` factory
        used by the loader.
- **Macros**. A single [`define_plugin!`](hycore/src/base/ext.rs) invocation now generates the
    compatibility, entrypoint, loader, descriptor, and teardown symbols Hyperion expects. This keeps
    plugin crates declarative and prevents symbol mismatches while giving the host a uniform way to
    interrogate DLL metadata before instantiation.
- **Library management**. `PluginExtWrapper` and `LibraryWrapper` ensure that the shared object
    remains loaded for as long as any plugin value exists. The wrapper also triggers the teardown
    symbol when the library drops, giving plugins a chance to release global resources.
- **Host services**. `LibraryBuilderPtr` exposes whitelisted host APIs (currently the Python opaque
    loader registry) to entrypoint/teardown callbacks, while `ExtList` ferries per-instance
    configuration objects into plugin constructors.

## Automatic discovery

Hyperion keeps the TOML metadata file but now treats it as a cache fed by an optional discovery
stage:

1. Set `HY_LD_PATH` (path-separated) to directories containing your shared objects. Only filenames
    beginning with `hy` or `libhy` and ending in `.so`, `.dylib`, or `.dll` are considered. When the
    env var is omitted, Hyperion defaults to the `extensions/` folder colocated with `meta.toml`.
2. `HyperionMetaInfo::load_or_generate()` compares the persisted `env_ld_path` snapshot with the
    current environment. When the value changes—or when a requested extension is missing—the cache is
    invalidated.
3. Every candidate DLL is dlopened briefly so the host can call the standardized
    `__hyext_fn_describe` symbol. The descriptors returned by the plugin crate are converted into
    fresh `ExtMetaInfo` entries (UUID, name, canonical path) and written back to `meta.toml`.
4. Subsequent runs short-circuit unless `HY_LD_PATH` changes again, so hand-authored metadata can
    coexist with automatically discovered entries.

Using the `hy_`/`libhy` prefix dramatically narrows the search set and lets discovery scale even
when the directories also host unrelated native libraries.

## Lifecycle: from metadata to running plugin

1. **Discovery**. `InstanceContext::create` calls `HyperionMetaInfo::load_or_generate`, which reads
    the cached TOML file (overridable via `HY_CONFIG_PATH`) and refreshes it using `HY_LD_PATH`
    (defaulting to the sibling `extensions/` directory) whenever the cache is stale or a requested
    extension is missing.
2. **Preloading (optional)**. When Python bindings need to register opaque objects before instance
     creation, the host calls `preload_plugins`, which keeps `LibraryWrapper`s alive via
     `PluginPreloadGuard`.
3. **dlopen**. `load_so_lib` canonicalizes the path, takes a global lock to avoid platform-specific
     races, and loads the shared object through `libloading`.
4. **Entrypoint**. The generated `__hyext_fn_entrypoint` receives a `LibraryBuilderPtr`, allowing
     the plugin to register Python loaders or other global services.
5. **Compatibility check**. The generated `__hyext_fn_compatibility_check` returns a
     `semver::VersionReq`; the host compares it to `env!("CARGO_PKG_VERSION")` and aborts with
     `HyError::CompatibilityCheckFailed` if the requirement is not satisfied.
6. **Instantiation**. The generated `__hyext_fn_loader` matches the requested UUID, calls
     `PluginExtStatic::new`, and returns a boxed `PluginExt`. Any configuration supplied through
     `ExtList` (e.g., logger callbacks) can be consumed at this point.
7. **Attach**. Once all plugins are instantiated, `InstanceContext` builds an `Arc` and calls
     `PluginExt::attach_to` for each plugin, providing a `Weak` pointer they can upgrade during
     `initialize`.
8. **Initialize**. Each plugin receives a final `initialize()` call where per-instance state is
     registered (event hooks, logging callbacks, etc.).
9. **Steady state**. The plugin runs alongside the host. Logging helpers such as
     [hycore/src/ext/hylog.rs](hycore/src/ext/hylog.rs) show how macros bridge host code and plugin
     callbacks.
10. **Tear-down**. Dropping `InstanceContext` triggers `PluginExt::teardown` on every plugin. When
        the last strong reference to the shared object is gone, `LibraryWrapper::drop` invokes the
        generated `__hyext_fn_teardown` symbol so plugins can unregister global resources.

## Configuration channels

Extensions frequently need structured configuration from the host application. Hyperion provides two
mechanisms:

- **`ExtList`** ([hycore/src/utils/conf.rs](hycore/src/utils/conf.rs)) holds boxed `OpaqueObject`
    values. The list is populated via Rust or Python before `InstanceContext::create` is invoked. The
    logger plugin, for example, looks up `LogCreateInfoEXT` during `PluginExtStatic::new`.
- **Python opaque loaders** enable high-level bindings to push dataclasses across the FFI boundary.
    Each extension may register a loader inside its entrypoint using `LibraryBuilderPtr` so that the
    Python runtime can `extract()` strongly-typed objects.

## Building a plugin from scratch

Below is a condensed workflow for writing a new DLL/`cdylib` plugin that exports multiple concrete
extensions.

1. **Scaffold the crate**:

     ```toml
     [package]
     name = "my_hy_plugin"
     edition = "2024"

     [lib]
     crate-type = ["cdylib"]

     [dependencies]
     hycore = { path = "../../hycore" }
     semver = { workspace = true }
     uuid = { workspace = true, features = ["v4"] }
     ```

2. **Describe each plugin type**:

     ```rust
     use hycore::{
             base::ext::{PluginExt, PluginExtStatic},
             define_plugin,
     };
     use semver::Version;
     use uuid::{Uuid, uuid};

     pub struct FooPlugin {
             version: Version,
     }

     impl PluginExtStatic for FooPlugin {
             const UUID: Uuid = uuid!("cdb726aa-8656-486f-a5b5-ff09f37a83fb");
             const NAME: &'static str = "__EXT_FOO";
             const DESCRIPTION: &'static str = "Provides foo-related features";

             fn new(_ext: &mut hycore::utils::conf::ExtList) -> Self {
                     Self { version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap() }
             }
     }

     impl PluginExt for FooPlugin {
             fn uuid(&self) -> Uuid { Self::UUID }
             fn version(&self) -> &Version { &self.version }
             fn name(&self) -> &str { Self::NAME }
             fn description(&self) -> &str { Self::DESCRIPTION }
             fn attach_to(&mut self, _instance: std::sync::Weak<hycore::base::InstanceContext>) {}
             fn initialize(&self) -> hycore::utils::error::HyResult<()> { Ok(()) }
             fn teardown(&mut self) {}
     }
     ```

3. **Generate the required symbols**:

     ```rust
     define_plugin!(
             ">=0.1",
             entry => plugin_entrypoint,
             teardown => plugin_teardown,
             plugins => [FooPlugin],
     );

     pub fn plugin_entrypoint(_builder: hycore::base::ext::LibraryBuilderPtr) {}
     pub fn plugin_teardown(_builder: hycore::base::ext::LibraryBuilderPtr) {}
     ```

4. **Register metadata** so the host knows where to find the shared object:

     ```toml
     [[ext]]
     uuid = "cdb726aa-8656-486f-a5b5-ff09f37a83fb"
     name = "__EXT_FOO"
     path = "/abs/path/to/libmy_hy_plugin.so"
     ```

    When `HY_LD_PATH` is set, the host writes these entries automatically after probing
    `__hyext_fn_describe`, so manual snippets remain optional for day-to-day development.

5. **Consume from the host**:

     ```rust
    use hycore::base::{ext::load_plugin_by_name, meta::HyperionMetaInfo};

    let required = vec!["__EXT_FOO".to_string()];
    let meta = HyperionMetaInfo::load_or_generate(&required)?;
    let mut ext_list = hycore::utils::conf::ExtList(vec![]);
    let plugin = unsafe { load_plugin_by_name(&meta, "__EXT_FOO", &mut ext_list)? };
    println!("Loaded {}", plugin.name());
     ```

In production, pass the same `enabled_extensions` vector you feed into `InstanceCreateInfo` so the
cache refresh logic can ensure every requested plugin is discoverable. If `HY_LD_PATH` is unset, the
loader still scans the default `extensions/` directory next to `meta.toml`.

## Case study: the logger extension

The built-in logger (`hylog` crate) showcases most patterns discussed above:

- Custom configuration travels through `LogCreateInfoEXT`, an `OpaqueObject` that Python bindings
    know how to serialize. The plugin stores the user callback and minimum log level.
- During `initialize`, the plugin writes a function pointer into
    `InstanceStateEXT::log_callback`, allowing host macros such as `hyinfo!` or `hywarn!` to funnel
    messages through the plugin.
- `teardown` simply restores the default callback so future instances can run without logging.
- See [hylog/src/lib.rs](hylog/src/lib.rs) for the fully annotated source.

## Python workflow

Python bindings mirror the Rust structures so that scripts can spin up instances without touching
unsafe code:

```python
from hypi.api import (
        ApplicationInfo,
        InstanceCreateInfo,
        InstanceEXT,
        Version,
        create_instance,
)
from hypi.api.ext_hylog import LogCreateInfoEXT, LogLevelEXT

def py_logger(msg):
        print(f"[{msg.level.name}] {msg.module}: {msg.message}")

create_info = InstanceCreateInfo(
        application_info=ApplicationInfo(
                application_name="Notebook",
                application_version=Version.parse("0.1.0"),
                engine_name="Hyperion",
                engine_version=Version.parse("0.1.1"),
        ),
        enabled_extensions=[InstanceEXT.LOGGER.value],
        ext=[LogCreateInfoEXT(level=LogLevelEXT.DEBUG, callback=py_logger)],
)

instance = create_instance(create_info)
```

When `create_instance` crosses the FFI boundary, the dataclasses become the struct types defined in
[hycore/src/base/api.rs](hycore/src/base/api.rs), and the logger plugin finds its configuration via
`ExtList::take_ext`.

## Troubleshooting and best practices

- **Stabilize UUIDs**. Treat a UUID change as a completely different plugin.
- **Log aggressively**. The macros in [hycore/src/ext/hylog.rs](hycore/src/ext/hylog.rs) are cheap;
    use them to annotate every stage of load/initialize/teardown.
- **Validate `HY_LD_PATH`**. Keep the directories accurate and prefixed libraries in place so the
    metadata cache does not churn; the stored `env_ld_path` snapshot is a quick way to diagnose what
    the host saw last.
- **Handle errors deterministically**. Bubble errors through `HyResult` so hosts can surface them to
    end users. Avoid panicking inside plugin constructors.
- **Version consciously**. Encode real compatibility expectations in the `define_plugin!`
    `compat` string. Semantic ranges such as `"=0.1.1"`, `"^0.1"`, or `">=1.0,<2.0"` are all valid.
- **Keep entrypoints idempotent**. Hosts may preload plugins before instance creation. Limit
    entrypoint work to lightweight registration steps and store state in the instance rather than
    globals whenever possible.
- **Use `ExtList` sparingly**. Remove configuration objects once consumed so other plugins do not
    accidentally grab them.

Armed with this reference and the inline Rustdoc/docstrings contributed across the codebase, you can
confidently extend Hyperion with bespoke functionality tailored to your workloads.
