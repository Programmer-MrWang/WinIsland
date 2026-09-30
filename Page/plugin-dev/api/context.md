# Context API

`ContextApiV2` publishes an activity or alert as text that WinIsland can show in the compact island. Declare `CAP_CONTEXT` and query `IFACE_CONTEXT`. The SDK offers `host.context()?.create(title, body)` for a default medium-priority context; use the raw table for priority, timeout, or compact text.

## Methods

The parameters below follow the common `context, token` arguments. Each method returns `PluginStatus`.

| Method | Parameters | Result |
|---|---|---|
| `create` | `*const ContextDataV2`, `*mut ResourceId` | Publishes a context and writes its owned ID. |
| `update` | `ResourceId`, `*const ContextDataV2` | Replaces the data and refreshes its update time. |
| `release` | `ResourceId` | Removes the context. |

## Context data

`ContextDataV2` contains `struct_size`, `priority`, `flags`, `timeout_ms`, `title`, `body`, and `compact_text`. Priorities are `PRIORITY_LOW` (playback), `PRIORITY_MEDIUM` (activity), and `PRIORITY_HIGH` (alert). `CONTEXT_FLAG_SHOW_COMPACT` enables compact display. A zero timeout keeps the context until release. `title` must be nonempty; `compact_text` falls back to `title` when empty.

The fixed buffers hold at most 255 title bytes, 511 body bytes, and 127 compact-text bytes plus NUL. The host accepts only known priority and flag values. Update the same resource as its text changes instead of repeatedly creating new contexts. The current quota is 64 contexts per plugin.

See the [quickstart example](/plugin-dev/quickstart) and [ABI definition](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/types/v2/context.rs).

[All plugin APIs](/plugin-dev/api)
