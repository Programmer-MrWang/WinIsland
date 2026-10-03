# Host services

ABI v2 exposes eighteen versioned service tables through `PluginHostV2.query`. This page is a tour; the [API reference](/plugin-dev/api) documents each table's methods and data contract. The SDK `Host` wrapper covers common operations; the raw tables in `winisland_plugin_api::abi` expose every function. Each raw table starts with `TablePrefix` and currently uses `IFACE_VERSION_1`. Validate required function slots before calling them. Except for Log, declare the matching `CAP_*` bit in `PluginDescriptorV2`.

Think of a service as an entry point into one part of WinIsland. A capability bit says your plugin needs it; querying the table gives you its functions; calling `create` or `register` gives you an owned resource. Keep the returned SDK object or raw ID for as long as the content should exist.

| Capability | Raw table | SDK access | Main operations |
|---|---|---|---|
| `CAP_CONTEXT` | `ContextApiV2` | `host.context()?` | Create, update, release activity text |
| `CAP_ACTIVITY` | `ActivityApiV2` | `host.activities()?` | Drawn activities, priority, page navigation, and visibility holds |
| `CAP_SYSTEM` | `SystemApiV2` | `host.system()?` | Local date/time, lunar conversion, and UI language |
| `CAP_MEDIA` | `MediaApiV2` | `host.media()?` | Publish a media source and read current title |
| `CAP_I18N` | `I18nApiV2` | `host.i18n()?` (query only) | Register and release translation bundles |
| `CAP_HOST_STATE` | `HostStateApiV2` | `host.host_state()?` | Read or subscribe to media/theme state |
| `CAP_WIDGET` | `WidgetApiV2` | `host.widgets()?` | Create, update, release, draw, and size widgets |
| `CAP_SURFACE` | `SurfaceApiV2` | `host.surfaces()?` | Expanded pages, compact content, and island layers |
| `CAP_INPUT` | `InputApiV2` | `host.input()?` | Pointer, keyboard, IME, and file-drop regions |
| `CAP_EVENTS` | `EventsApiV2` | `host.events()?` | Subscriptions, timers, island state, and animation scheduling |
| `CAP_COMMAND` | `CommandApiV2` | `host.commands()?` | Commands, tray entries, and global hotkeys |
| `CAP_MEDIA_SESSION` | `MediaSessionApiV2` | `host.media_sessions()?` | Discover, select, and control existing media sessions |
| `CAP_LYRICS` | `LyricsTransformApiV2` | `host.lyrics()?` | Register/release a lyric transformer |
| `CAP_SETTINGS` | `SettingsApiV2` | `host.settings()?` | Create/update/release a settings page |
| `CAP_TEXT` | `TextApiV2` | `host.text()?` | Measure text and query font family |
| `CAP_IMAGE` | `ImageApiV2` | `host.images()?` | Decode, upload, use album art, release |
| `CAP_STORE` | `StoreApiV2` | `host.store()?` | Get, set, delete plugin-scoped bytes |
| None | `LogApiV2` | `host.log()` | Write plugin log messages |

## Activities and system data

[Activity](/plugin-dev/api/activity) binds a `SURFACE_COMPACT_MAIN` and an optional `SURFACE_PAGE`. The host compares activities with native music, timers, and legacy text by priority and update time. Use `ACTIVITY_KEEP_VISIBLE` for an eligible ongoing task that should hold off inactivity auto-hide. Expiry hides the activity without freeing its handle; release it when done. Context remains the simple text option.

[System](/plugin-dev/api/system) supplies local date/time, optional Chinese lunar conversion, and the current WinIsland UI language. It requires no resource handles and lets built-in plugins use public services instead of importing host date or language implementations.

## Context and Media

`ContextDataV2` has a priority, compact flag, timeout, title, body, and compact text. Zero timeout keeps it until release. Update the same resource rather than creating a new one on each refresh. The SDK's `host.context()?.create(title, body)` returns an owned `Resource`.

Use Context for a short-lived message such as “Export complete” or a persistent activity such as “Timer running.” Use Media when the plugin itself supplies the active track. `MediaSourceDataV2` carries title, artist, album, duration/position in milliseconds, playing flag, optional PNG/JPEG cover bytes, declared controls, and an optional command callback. The SDK `create_source(title, artist)` publishes metadata without controls; use `MediaApiV2` directly for cover, timeline, or controls. Releasing or disabling a plugin media source lets WinIsland fall back to another source or SMTC. Keep callback data valid until release is safe.

## Widget drawing

`host.widgets()?.create(WidgetSpec::new("key").span(2, 1))` creates a layout-managed widget. Stable keys identify placement across restarts. `Widget::logical_size()` returns the current logical dimensions; skip a frame if it returns `(0, 0)`. The size follows the expanded grid even while the island collapses.

Build a complete list with `DrawListBuilder::new(Size::new(width, height))`, add commands such as `fill_round_rect`, `text`, `text_runs`, or `image`, then call `widget.submit(list.finish())`. The draw protocol supports clips, transforms, alpha, shapes, gradients, strokes, shadows, images, and UTF-8 text with a font-family field. The host copies the list and validates it before replay. Submission success does not guarantee display; malformed frames can eventually disable that widget. No plugin callback runs on the render thread.

The draw list is limited to 4 MiB, 4096 commands, and 64 KiB of text per command. `Widget::request_redraw` is best effort. `PluginDescriptorV2.on_tick` receives `WidgetId` and elapsed seconds on a plugin worker.

## Interactive surfaces and scheduling

[Surface](/plugin-dev/api/surface) adds expanded pages, compact left/right content, and background/foreground layers. It reuses the validated drawing protocol and shares widget resource quotas. [Input](/plugin-dev/api/input) assigns logical hit regions to a surface or grid widget; [Events](/plugin-dev/api/events) delivers pointer, keyboard, IME, and dropped-file callbacks on the plugin worker.

Events also provides visibility/resize notifications, island state, timers, and per-target animation control. Disable continuous animation for static content and submit only when needed. Keep subscription and timer handles in the instance, cancel them before releasing their targets, and avoid capturing the owning instance strongly in its own callbacks.

[Command](/plugin-dev/api/command) registers named actions with optional tray entries and global hotkeys. [Media Session](/plugin-dev/api/media-session) enumerates native and plugin media, selects sources, and sends playback requests. Subscribe to `EVENT_RESULT` before sending requests; immediate success confirms queuing, and the later event reports the operation status.

## Lyrics, i18n, and Host State

A lyric transformer runs after lyrics are parsed. It uses a size query and then a write pass. Preserve the same Unicode character count on word-synchronised lines to keep timing boundaries. The SDK `host.lyrics()?.register` accepts a `Send + Sync` transformation closure and retains its callback storage while registered.

`I18nApiV2` registers a language-tagged bundle of key/value pairs. `HostStateApiV2.get` returns media title, artist, playing state, and light/dark theme. `subscribe` registers a callback; release the subscription before unloading.

## Settings, Text, Image, Store, and Log

`SettingsApiV2` accepts a declarative `SettingsPageDataV2` with a stable page key, title, optional icon, and items: section, group, label, switch, select, stepper, and button. Its `on_change` callback can accept or reject a user action. SDK `create_page` uses the title as its section heading; `create_label_page` supplies separate section text; use the raw table for interactive items and callbacks. Persist values explicitly with Store, then repopulate item values on create.

`TextApiV2.measure` accepts `TextStyleV2` with size, weight, italic flag, and UTF-8 font family; `font_family` reports host families. The SDK `measure` convenience call uses weight 400 and upright style. `ImageApiV2` can decode PNG/JPEG/WebP, upload RGBA pixels, or capture current album art; image IDs remain owned until release. A captured album-art handle keeps its image after the cover changes.

`StoreApiV2` stores bytes under plugin-local keys across restarts; a value is limited to 1 MiB. `LogApiV2.write` uses a numeric level and has no capability gate. SDK `LogApi::write` and `Widget::request_redraw` discard errors; use raw tables if the status matters.

For a setting such as “show seconds,” read its Store value during `create`, put that value into the Settings page, and update Store after `on_change` accepts a new choice. Recreating the page alone does not persist the choice. For a widget that shows the song title, combine Host State or Media with Text/Image and Widget; Widget does not supply content on its own.

## Resource limits and errors

Current per-plugin resource limits are 64 Activities, 64 Contexts, 4 Media sources (32 MiB total), 16 i18n bundles (4 MiB), 8 Widgets and Surfaces combined (4 MiB), 4 lyric transformers, 1 Settings page (2 MiB), 64 Images (64 MiB), 16 Host State subscriptions, 128 Events subscriptions/timers combined, and 64 Commands. The host rejects stale or foreign handles and quota overflows using `PluginStatus`. Releases should occur before successful shutdown; the host revokes remaining resources afterward.
