# Plugin quickstart

This example builds a complete ABI v2 DLL that publishes one context. It validates the host input, keeps the resource alive, releases it during `shutdown`, and frees the opaque instance in `destroy`.

## Prerequisites

- Windows 10 version 2004 or later, or Windows 11
- Stable Rust with the `x86_64-pc-windows-msvc` target
- Visual Studio C++ build tools and Windows SDK
- A WinIsland build with ABI v2 support

## Create a library

```powershell
cargo new --lib hello-winisland-plugin
cd hello-winisland-plugin
```

Use this `Cargo.toml`. Until an ABI v2 crate is published to the registry, use the repository source shown here. After publication, a matching `winisland-plugin-api = "0.8"` release can replace the Git dependency.

```toml
[package]
name = "hello-winisland-plugin"
version = "0.1.0"
edition = "2024"
authors = ["Example Author"]
description = "Minimal WinIsland ABI v2 plugin"
repository = "https://github.com/example/hello-winisland-plugin"

[lib]
name = "hello_winisland_plugin"
crate-type = ["cdylib"]

[dependencies]
winisland-plugin-api = { git = "https://github.com/WinIslandProject/WinIsland" }
```

The package ID, name, version, author, and description must agree with the descriptor and the packaged manifest. The repository URL supplies `github-link`.

## Implement `src/lib.rs`

```rust
use std::ffi::c_void;
use winisland_plugin_api::abi::{
    ABI_VERSION_2, CAP_CONTEXT, PluginCreateInfoV2, PluginDescriptorV2,
    PluginHandleV2, PluginStatus,
};
use winisland_plugin_api::sdk::{Host, Resource};
use winisland_plugin_api::PluginMetadataC;

struct Instance {
    context: Option<Resource>,
}

static DESCRIPTOR: PluginDescriptorV2 = PluginDescriptorV2 {
    struct_size: std::mem::size_of::<PluginDescriptorV2>() as u32,
    abi_version: ABI_VERSION_2,
    capabilities: CAP_CONTEXT,
    metadata: PluginMetadataC::new(
        "hello-winisland-plugin",
        "hello-winisland-plugin",
        env!("CARGO_PKG_VERSION"),
        "Example Author",
        "Minimal WinIsland ABI v2 plugin",
    ),
    create: Some(create),
    shutdown: Some(shutdown),
    destroy: Some(destroy),
    on_tick: None,
};

unsafe extern "C" fn create(
    info: *const PluginCreateInfoV2,
    out_handle: *mut PluginHandleV2,
) -> PluginStatus {
    if info.is_null() || out_handle.is_null() {
        return PluginStatus::InvalidArgument;
    }
    // SAFETY: WinIsland supplies a readable create-info header.
    let info = unsafe { &*info };
    if info.struct_size < std::mem::size_of::<PluginCreateInfoV2>() as u32
        || info.abi_version != ABI_VERSION_2
        || info.plugin_token == winisland_plugin_api::PluginToken::INVALID
    {
        return PluginStatus::UnsupportedVersion;
    }
    // SAFETY: The host table remains allocated throughout this instance's lifetime.
    let host = match unsafe { Host::from_raw(info.host_api, info.plugin_token) } {
        Ok(host) => host,
        Err(_) => return PluginStatus::InvalidArgument,
    };
    let context = match host
        .context()
        .and_then(|api| api.create("Hello WinIsland", "ABI v2 plugin is running"))
    {
        Ok(context) => context,
        Err(_) => return PluginStatus::Internal,
    };
    let instance = Box::new(Instance {
        context: Some(context),
    });
    // SAFETY: WinIsland treats this pointer as opaque until destroy.
    unsafe { out_handle.write(Box::into_raw(instance).cast::<c_void>()) };
    PluginStatus::Ok
}

unsafe extern "C" fn shutdown(handle: PluginHandleV2) -> PluginStatus {
    if handle.is_null() {
        return PluginStatus::InvalidArgument;
    }
    // SAFETY: This handle was created above and has not been destroyed.
    let instance = unsafe { &mut *handle.cast::<Instance>() };
    drop(instance.context.take());
    PluginStatus::Ok
}

unsafe extern "C" fn destroy(handle: PluginHandleV2) {
    if !handle.is_null() {
        // SAFETY: WinIsland calls destroy once after successful shutdown.
        unsafe { drop(Box::from_raw(handle.cast::<Instance>())) };
    }
}

/// # Safety
/// WinIsland calls this exported symbol using the ABI v2 entry signature.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn winisland_plugin_entry_v2() -> *const PluginDescriptorV2 {
    &DESCRIPTOR
}
```

`PluginStatus` is a numeric status; it does not carry an error string. Log diagnostic details through `LogApiV2` when needed. Do not unwind through any exported C callback.

## Build and load

```powershell
cargo check
cargo clippy -- -D warnings
cargo build --release
```

The DLL is `target/release/hello_winisland_plugin.dll`. During local development, place it in the root of WinIsland's plugin directory and restart the app. Root-level DLLs are manual installations and have no `plugin.yml`. Remove a manual DLL before installing a packaged copy with the same ID.

For a distributable ZIP, follow [Packaging and installation](/plugin-dev/packaging). Set `abi-version: 2` and make `entry` equal to the DLL filename. You can also drop the ZIP onto the island while WinIsland is running.

## Extend the example

- Use [Host services](/plugin-dev/services) for Media, Widgets, Settings, Images, Store, and lyrics.
- Use [ABI and lifecycle](/plugin-dev/abi-lifecycle) before adding callbacks or threads.
- See the [SDK widget example](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/examples/minimal_widget.rs) for `DrawListBuilder` usage.
