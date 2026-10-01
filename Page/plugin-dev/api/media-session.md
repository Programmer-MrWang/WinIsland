# Media Session API

`MediaSessionApiV2` lists and controls existing native SMTC sessions and plugin media sources. It was added in crate `0.9.0`; declare `CAP_MEDIA_SESSION`, query `IFACE_MEDIA_SESSION`, or use `host.media_sessions()?`. To publish a new source owned by your plugin, use [Media API](/plugin-dev/api/media).

## Pause the current session

This helper also needs `CAP_EVENTS`. It subscribes before queuing the request and returns both the subscription handle and request sequence. Retain the handle and use the sequence to correlate results in your instance state.

```rust
use winisland_plugin_api::sdk::{CallbackResource, Error, Host};
use winisland_plugin_api::*;

fn pause_current(host: &Host) -> Result<(CallbackResource, u64), Error> {
    let log = host.log();
    let results = host.events()?.subscribe(
        EVENT_RESULT,
        WidgetId::INVALID,
        move |event| {
            log.write(2, &format!("Request {}: status {}", event.sequence, event.code));
        },
    )?;
    let sequence = host.media_sessions()?.send(0, SESSION_PAUSE, 0)?;
    Ok((results, sequence))
}
```

The callback reports all results delivered to this plugin, not only the pause request. Avoid waiting for it synchronously inside another plugin callback.

## Methods and snapshots

Raw calls take `context, token` first and return `PluginStatus`.

| SDK method | Raw method and remaining parameters | Behavior |
|---|---|---|
| `list()` | `list(*mut MediaSessionV2, capacity, *mut required)` | Returns a copied session snapshot. |
| `send(session, command, position_ms)` | `send(u64, u32, u64, *mut sequence)` | Queues an operation and returns its sequence. |

For raw `list`, a null buffer and zero capacity returns `Ok` with the required element count; an undersized nonnull buffer returns `LimitExceeded`. The SDK handles allocation and retries. Refresh after `EVENT_MEDIA`; notifications are signals to re-read, not complete session payloads.

`MediaSessionV2` includes a runtime `id`, flags, `available_controls`, duration/position in milliseconds, host-relative monotonic `sampled_at_seconds`, and NUL-terminated UTF-8 `source`, `title`, `artist`, and `album`. Flags are `SESSION_PLAYING`, `SESSION_CURRENT`, and `SESSION_PLUGIN`. The control mask uses the existing `MEDIA_CONTROL_TOGGLE_PLAY`, `MEDIA_CONTROL_PREVIOUS`, `MEDIA_CONTROL_NEXT`, and `MEDIA_CONTROL_SEEK` bits; it is not a mask of `SESSION_*` command numbers. Play and pause do not have separate capability bits.

Native discovery follows the host's SMTC enable setting. An empty list can mean no player exposes a session or native discovery is disabled; plugin sources are included independently. IDs are transient: re-enumerate after a session disappears and do not save them across process restarts.

## Commands and selection

| Command | Meaning |
|---|---|
| `SESSION_TOGGLE` | Toggle playing state |
| `SESSION_PLAY` / `SESSION_PAUSE` | Request an explicit playing state |
| `SESSION_PREVIOUS` / `SESSION_NEXT` | Skip a track |
| `SESSION_SEEK` | Seek to the absolute `position_ms` value |
| `SESSION_SELECT` | Select the session used by the island |

For playback operations, session ID `0` means the currently selected/automatic source. `send(0, SESSION_SELECT, 0)` restores automatic selection. An explicit selection can change the island's displayed media and default controls; selecting a source does not itself start playback. Selection is released when its owning plugin unloads. Use `position_ms = 0` for commands other than seek.

`send()` returning success means accepted for processing. Match `EVENT_RESULT.sequence` with the returned sequence and read the status from `code`. A source can disappear after enumeration (`StaleHandle`), reject an unsupported operation (`InvalidArgument`), or fail in the platform (`IoError`). Refresh state after completion instead of assuming playback already changed. For plugin sources, explicit play/pause uses the existing toggle callback when a state change is needed; completion confirms callback dispatch, not completion of the plugin's external player work.

[All plugin APIs](/plugin-dev/api)
