# Command API

`CommandApiV2` registers plugin commands, optional tray entries and global hotkeys, and executes host or plugin commands. It was added in crate `0.9.0`; declare `CAP_COMMAND`, query `IFACE_COMMAND`, or use `host.commands()?`. Native tray and hotkey integration stays in the host.

## Register a command

```rust
use winisland_plugin_api::sdk::{CallbackResource, Error, Host};
use winisland_plugin_api::*;

fn register_ping(host: &Host) -> Result<CallbackResource, Error> {
    let log = host.log();
    host.commands()?.register(CommandSpecV2 {
        key: str_to_fixed("ping"),
        title: str_to_fixed("My plugin: Ping"),
        flags: COMMAND_ENABLED | COMMAND_TRAY,
        hotkey_modifiers: MOD_CONTROL | MOD_ALT,
        hotkey_key: u32::from('P'),
        ..Default::default()
    }, move |event| {
        log.write(2, &format!("Ping received: {} bytes", event.data.len()));
    })
}
```

Keep the returned handle in the instance. For plugin ID `my-plugin`, this registers `my-plugin.ping`. The callback receives `EVENT_COMMAND` on the plugin worker. It does not require an Events subscription; `CAP_EVENTS` is needed only if the plugin also subscribes to completion notifications.

## Methods

Raw calls take `context, token` first and return `PluginStatus`.

| SDK method | Raw method and remaining parameters | Behavior |
|---|---|---|
| `register(spec, handler)` | `register(*const CommandSpecV2, *mut ResourceId)` | Creates an owned command and callback. |
| `handle.set_enabled(bool)` | `set_enabled(ResourceId, u8)` | Enables/disables this command; this helper is only valid for command handles. |
| `list()` | `list(*mut CommandInfoV2, capacity, *mut required)` | Lists built-ins and registered plugin commands, including disabled commands. |
| `execute(id, data)` | `execute(Utf8Slice, ByteSlice, *mut sequence)` | Queues an enabled command and returns a request sequence. |
| `handle.cancel()` | `release(ResourceId)` | Removes the command, callback, tray entry, and hotkey binding. |

Keys contain 1–63 ASCII letters, digits, `_`, `-`, or `.` and are prefixed with the plugin ID. The nonempty title fits in 255 UTF-8 bytes; use `str_to_fixed` and sized defaults. Duplicate IDs are rejected. `COMMAND_ENABLED` and `COMMAND_TRAY` are the only flags. Up to 64 commands may be registered per plugin.

Set `hotkey_key = 0` for no hotkey; otherwise use an ASCII letter or digit with any of `MOD_CONTROL`, `MOD_ALT`, `MOD_SHIFT`, and `MOD_SUPER`. Binding happens later on the UI thread. Registration success does not guarantee that Windows accepted the shortcut. Binding failure is logged and reported as `EVENT_RESULT` with `code = PluginStatus::IoError.code() as u32` and `sequence = command_handle.id().get()`.

## Execute and observe

An execution payload is copied and limited to 64 KiB; the command ID is limited to 160 UTF-8 bytes. Unknown or disabled commands return `InvalidArgument`. `execute()` success only confirms queuing. Subscribe to `EVENT_RESULT` with `WidgetId::INVALID` before execution and match its `sequence`. The command callback's `data` carries the payload. Completion means the callback returned, not that work it started elsewhere has finished. Tray and hotkey callbacks have no caller request to complete.

For raw `list`, a null buffer and zero capacity returns `Ok` with the element count. A too-small nonnull output returns `LimitExceeded`. The SDK retries if the list grows between passes.

| Built-in ID | Action |
|---|---|
| `winisland.expand` | Expand the island |
| `winisland.collapse` | Collapse the island |
| `winisland.settings` | Open settings |
| `winisland.media.toggle` | Toggle playback |
| `winisland.media.next` | Next track |
| `winisland.media.previous` | Previous track |

Built-ins ignore payload bytes. For targeting a particular player or observing its operation result, use [Media Session](/plugin-dev/api/media-session). Release returns `LimitExceeded` while the command callback is executing; follow the [callback lifetime rules](/plugin-dev/api/events). Unloading a plugin removes its registrations.

[All plugin APIs](/plugin-dev/api)
