# Hyperion Plugin Runtime

The modern Hyperion plugin surface is centered on the `hycore::ext` module. It exposes the traits,
registration helpers, and inventory lookups that let downstream crates declare extensions without
writing any `dlopen` boilerplate.

This document complements the inline Rustdoc comments and walks through how to author, register, and
consume extensions in the new architecture.

## Runtime primitives

`hycore/src/ext/mod.rs` contains the host-facing pieces:

- **Traits**
  - `StaticPluginEXT` encodes compile-time metadata (`UUID`, `NAME`, `DESCRIPTION`) and provides the
    `new(&mut OpaqueList)` constructor used when an extension is instantiated.
  - `PluginEXT` describes runtime hooks (`attach_to`, `initialize`, `teardown`) that the host calls as
    an instance is built and destroyed.
  - `DynPluginEXT` is the object-safe supertrait used to store heterogeneous plugins inside an
    `InstanceContext`.
- **Registry**
  - `PluginRegistry` is an inventory entry describing how to construct a concrete plugin.
  - `define_plugin!(MyPlugin)` expands to a static `PluginRegistry` submission. It automatically wires
    the loader closure so the host can ask for the plugin by name.
- **Loader**
  - `load_plugin_by_name(name, ext_list)` is the single entrypoint `InstanceContext::create` uses to
    turn user-supplied strings into boxed plugin values.
- **Opaque configuration**
  - `utils::opaque::OpaqueList` is how extensions receive typed configuration at startup. Each plugin
    extracts the structs it understands and leaves the rest untouched.

The legacy dynamic loading code (`hycore::base::ext`) has been removed. Inventory submissions make
plugins feel like first-class Rust components while still supporting dynamic composition.

## Instance lifecycle

1. **Create info**. Callers assemble `api::InstanceCreateInfo`, filling the application metadata,
   enabled extension names, and optional opaque config objects. Python bindings expose the same type
   via `hypi.api.InstanceCreateInfo`.
2. **Plugin instantiation**. `InstanceContext::create` iterates over the requested extension names and
   calls `load_plugin_by_name`. The loader finds the matching `PluginRegistry` entry and runs its
   constructor.
3. **Attachment**. Once all plugins exist, Hyperion builds the shared `Arc<InstanceContext>` and calls
   `PluginEXT::attach_to` so extensions can store a `Weak` pointer.
4. **Initialization**. With the `Arc` finalized, each plugin receives `initialize()`. This is where
   per-instance state (log handlers, callbacks, caches) should be registered.
5. **Steady state**. Extensions run side-by-side with core modules. They can call back into the host
   using the instance pointer provided earlier.
6. **Teardown**. Dropping the `InstanceContext` drains the `extensions` map and invokes
   `PluginEXT::teardown` in reverse order of insertion. Use this hook to release resources or detach
   callbacks.

## Building a plugin

Below is a minimal example showing how to implement and register a plugin crate.

```rust
use hycore::{
    define_plugin,
    ext::{PluginEXT, StaticPluginEXT},
    utils::{error::HyResult, opaque::OpaqueList},
};
use uuid::{Uuid, uuid};
use std::sync::Weak;

pub struct FooPlugin {
    instance: Option<Weak<hycore::base::InstanceContext>>,
}

define_plugin!(FooPlugin);

impl StaticPluginEXT for FooPlugin {
    const UUID: Uuid = uuid!("cdb726aa-8656-486f-a5b5-ff09f37a83fb");
    const NAME: &'static str = "__EXT_FOO";
    const DESCRIPTION: &'static str = "Provides foo-related features";

    fn new(_ext: &mut OpaqueList) -> Self {
        Self { instance: None }
    }
}

impl PluginEXT for FooPlugin {
    fn attach_to(&mut self, instance: Weak<hycore::base::InstanceContext>) {
        self.instance = Some(instance);
    }

    fn initialize(&self) -> HyResult<()> {
        // Instance has been fully constructed at this point.
        Ok(())
    }

    fn teardown(&mut self) {
        // Clean up global state if needed.
    }
}
```

Any crate that links `hycore` can submit a plugin like this. Once compiled, the plugin simply needs to
be included in the binary or dependency graph; the inventory table ensures `load_plugin_by_name`
locates it.

## Logger extension case study

The built-in logger under `hycore::ext::hylog` demonstrates the full flow:

- `LogCreateInfoEXT` is an opaque configuration object (also exposed to Python) that carries the log
  level and callback.
- `LogPluginEXT` implements the traits and registers itself via `define_plugin!(LogPluginEXT)`.
- During `initialize`, the plugin installs `log_message` into `InstanceStateEXT::log_callback` so all
  `hytrace!`, `hyinfo!`, etc. invocations reach the user callback.

Refer to `hycore/src/ext/hylog/impl.rs` for a comprehensive reference implementation.

## Python bindings

`python/python/hypi/api` mirrors the Rust structs with Pydantic dataclasses, keeping the experience
consistent across languages. The `create_instance` helper converts those dataclasses back into the C
ABI structs that `hycore::base::api` understands.

Extensions that expect opaque configuration types should call the `define_py_opaque_object_loaders!`
macro to register Python loaders. The logger plugin already registers `LogCreateInfoEXT` so scripts can
write:

```python
from hypi.api import (
    ApplicationInfo,
    InstanceCreateInfo,
    InstanceEXT,
    Version,
    create_instance,
)
from hypi.api.ext.hylog import LogCreateInfoEXT, LogLevelEXT

create_info = InstanceCreateInfo(
    application_info=ApplicationInfo(
        application_name="Notebook",
        application_version=Version.parse("0.1.0"),
        engine_name="Hyperion",
        engine_version=Version.parse("0.1.1"),
    ),
    enabled_extensions=[InstanceEXT.LOGGER.value],
    ext=[LogCreateInfoEXT(level=LogLevelEXT.DEBUG, callback=lambda msg: print(msg.message))],
)

instance = create_instance(create_info)
```

## Best practices

- **Stabilize UUIDs**: Treat UUID changes as breaking; hosts rely on them as stable identifiers.
- **Fail fast**: Prefer returning `HyResult` errors over panicking. The host surfaces these failures to
  callers.
- **Use `OpaqueList` judiciously**: Remove configuration objects once consumed to avoid accidental
  reuse by other extensions.
- **Keep `initialize` lightweight**: Heavy setup should be deferred or cached lazily to minimize
  startup time.
