# Plugin development

WinIsland loads trusted Windows DLLs using ABI v2. The current `winisland-plugin-api` crate is `0.8`. ABI v1 packages and entry points are rejected. Plugins can provide contexts, media sources, widgets, translations, lyric transforms, settings pages, and persistent values.

> Plugins run inside WinIsland without a sandbox. A panic in an `extern "C"` callback can terminate the app.

## Guides

| Guide | What it covers |
|---|---|
| [Quickstart](/plugin-dev/quickstart) | Build, load, and package an ABI v2 plugin |
| [ABI and lifecycle](/plugin-dev/abi-lifecycle) | Descriptor validation, ownership, callbacks, shutdown, and migration |
| [Host services](/plugin-dev/services) | All eleven service tables, drawing, settings, and limits |
| [API reference](/plugin-dev/api) | One page per public service table, with methods, data contracts, and limits |
| [Packaging and installation](/plugin-dev/packaging) | `plugin.yml`, ZIPs, signing, installation, and updates |
| [API changelog](/api-changelog) | Historical published crate release notes |

See the [SDK README](https://github.com/WinIslandProject/WinIsland/tree/master/crates/winisland-plugin-api) and [ABI definitions](https://github.com/WinIslandProject/WinIsland/tree/master/crates/winisland-plugin-api/src/abi) for exact Rust signatures.

## Runtime model

```text
DLL exports winisland_plugin_entry_v2() -> static PluginDescriptorV2
    -> host validates ABI, capabilities, callbacks, and metadata
    -> host calls create(PluginCreateInfoV2) with token and PluginHostV2
    -> plugin queries versioned service tables and creates resources
    -> optional descriptor.on_tick runs on a plugin worker
    -> widget submits a complete draw list; host validates and replays it
    -> host calls shutdown(handle), then destroy(handle), then unloads DLL
```

`PluginDescriptorV2.capabilities` declares access to services. Every resource belongs to a host-issued `PluginToken`. Service tables use `PluginHostV2.query` and `IFACE_VERSION_1`, a separate version from `ABI_VERSION_2`.

| Capability | Table | Purpose |
|---|---|---|
| `CAP_CONTEXT` | `ContextApiV2` | Activity text |
| `CAP_MEDIA` | `MediaApiV2` | Now-playing source, cover, and controls |
| `CAP_I18N` | `I18nApiV2` | Translation bundles |
| `CAP_HOST_STATE` | `HostStateApiV2` | Media/theme snapshot and subscriptions |
| `CAP_WIDGET` | `WidgetApiV2` | Widgets and draw-list submission |
| `CAP_LYRICS` | `LyricsTransformApiV2` | Parsed lyric transforms |
| `CAP_SETTINGS` | `SettingsApiV2` | Declarative settings pages |
| `CAP_TEXT` | `TextApiV2` | Text measurement and font families |
| `CAP_IMAGE` | `ImageApiV2` | Images and current album art |
| `CAP_STORE` | `StoreApiV2` | Plugin-scoped persistent bytes |

`LogApiV2` is available without a capability bit. Declare only what the plugin uses.

## Development flow

1. Create a Rust `cdylib` with the `winisland-plugin-api` dependency shown in the [quickstart](/plugin-dev/quickstart).
2. Export `winisland_plugin_entry_v2` with a static `PluginDescriptorV2`.
3. Validate `PluginCreateInfoV2` in `create` and use SDK `Host::from_raw` or raw service tables.
4. Keep resource handles until `shutdown`; release them while host tables are valid.
5. Stop and join plugin workers before successful `shutdown`; free the opaque instance in `destroy`.
6. Package a ZIP with root-level `plugin.yml`, `abi-version: 2`, and the declared DLL. Drop it onto the island to install or update.

The SDK wraps common calls and builds draw lists. Advanced controls and settings changes use the raw ABI. Plugin drawing does not run on the render thread.

## Compatibility

Crate `0.8`, top-level `ABI_VERSION_2`, and service-table `IFACE_VERSION_1` are different version numbers. Check `struct_size` and `version` before reading a table. ABI v1 DLLs require source migration and repackaging; changing `plugin.yml` alone cannot convert one.
