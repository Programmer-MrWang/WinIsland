# Plugin development

WinIsland plugins are Windows DLLs loaded through ABI v2. A plugin asks WinIsland for a service, then uses that service to add content such as activity text, a media source, a widget, or a settings page. The current `winisland-plugin-api` crate is `0.11`; ABI v1 DLLs cannot be loaded by the current host.

> Plugins run inside WinIsland without a sandbox. A panic in an `extern "C"` callback can terminate the app.

## Guides

| Guide | What it covers |
|---|---|
| [Quickstart](/plugin-dev/quickstart) | Build, load, and package an ABI v2 plugin |
| [ABI and lifecycle](/plugin-dev/abi-lifecycle) | Descriptor validation, ownership, callbacks, shutdown, and migration |
| [Host services](/plugin-dev/services) | All eighteen service tables, drawing, settings, and limits |
| [API reference](/plugin-dev/api) | One page per public service table, with methods, data contracts, and limits |
| [Packaging and installation](/plugin-dev/packaging) | `plugin.yml`, ZIPs, signing, installation, and updates |
| [API changelog](/api-changelog) | Historical published crate release notes |

See the [SDK README](https://github.com/WinIslandProject/WinIsland/tree/master/crates/winisland-plugin-api) and [ABI definitions](https://github.com/WinIslandProject/WinIsland/tree/master/crates/winisland-plugin-api/src/abi) for exact Rust signatures.

## Start with what you want to build

| I want to… | Start here | What to expect |
|---|---|---|
| Put a short status or alert on the island | [Context API](/plugin-dev/api/context) | Publish text, then update or remove it. |
| Supply a song or playback controls | [Media API](/plugin-dev/api/media) | Publish track data; controls need a command callback. |
| Draw my own content in the expanded island | [Widget API](/plugin-dev/api/widget) | Submit a complete drawing for a grid widget. |
| Add an interactive page or compact panel | [Surface](/plugin-dev/api/surface), [Input](/plugin-dev/api/input), and [Events](/plugin-dev/api/events) | Draw content, define hit regions, and retain event subscriptions. |
| Add a tray command or shortcut | [Command API](/plugin-dev/api/command) | Register a callback with an optional tray entry and hotkey. |
| Update only while visible | [Events API](/plugin-dev/api/events) | Use a visible-only timer or control animation ticks. |
| Control an existing player | [Media Session API](/plugin-dev/api/media-session) | List sessions, select one, send operations, and observe completion. |
| Add options to WinIsland settings | [Settings API](/plugin-dev/api/settings) and [Store API](/plugin-dev/api/store) | Describe controls and save their values separately. |
| React to the current song or theme | [Host State API](/plugin-dev/api/host-state) | Read a snapshot or subscribe to changes. |
| Change displayed lyric text | [Lyrics Transform API](/plugin-dev/api/lyrics-transform) | Transform parsed lines before display. |

Build the [one-context example](/plugin-dev/quickstart) first if you have not loaded a plugin before. The [API reference](/plugin-dev/api) covers the other services and their exact call contracts.

## Three ideas to keep in mind

1. **Capability:** a bit in the plugin descriptor declaring which service you intend to use. Log is the only service here without a capability bit.
2. **Service table:** the set of functions WinIsland provides for that capability. The SDK wraps common calls; raw tables expose the full API.
3. **Resource:** something you create, such as a context, widget, or settings page. Keep its ID or SDK wrapper while it is needed, then release it before shutdown completes.

Crate 0.9 adds custom pages, compact surfaces, background/foreground layers, input regions, commands, timers, and media session control. Combine Surface or Widget with Input and Events for interactive UI. These are defined host extension points: they do not replace arbitrary host internals or intercept input outside the plugin's regions. The host owns native Windows integration, so these features do not require a Windows binding dependency in each plugin.

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
| `CAP_ACTIVITY` | `ActivityApiV2` | Drawn activities, priority, page navigation, and visibility holds |
| `CAP_SYSTEM` | `SystemApiV2` | Local date/time, lunar conversion, and UI language |
| `CAP_MEDIA` | `MediaApiV2` | Now-playing source, cover, and controls |
| `CAP_I18N` | `I18nApiV2` | Translation bundles |
| `CAP_HOST_STATE` | `HostStateApiV2` | Media/theme snapshot and subscriptions |
| `CAP_WIDGET` | `WidgetApiV2` | Widgets and draw-list submission |
| `CAP_SURFACE` | `SurfaceApiV2` | Expanded pages, compact content, and island layers |
| `CAP_INPUT` | `InputApiV2` | Pointer, keyboard, IME, and file-drop regions |
| `CAP_EVENTS` | `EventsApiV2` | Subscriptions, timers, island state, and animation scheduling |
| `CAP_COMMAND` | `CommandApiV2` | Commands, tray entries, and global hotkeys |
| `CAP_MEDIA_SESSION` | `MediaSessionApiV2` | Discover, select, and control existing media sessions |
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

Crate `0.11`, top-level `ABI_VERSION_2`, and service-table `IFACE_VERSION_1` are different version numbers. The new tables are additive; existing ABI v2 layouts remain unchanged. New capability bits still require an updated host: an older ABI v2 host may reject the descriptor before `create`. `Host::supports(IFACE_*)` checks table availability, not capability permission, and cannot bypass that load-time check. Check `struct_size` and `version` before reading a table. ABI v1 DLLs require source migration and repackaging; changing `plugin.yml` alone cannot convert one.
