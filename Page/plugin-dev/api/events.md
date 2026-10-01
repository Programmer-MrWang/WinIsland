# Events API

`EventsApiV2` supplies subscriptions, timers, island state, and animation scheduling. It was added in crate `0.9.0`; declare `CAP_EVENTS`, query `IFACE_EVENTS`, or use `host.events()?`. Callbacks run on the plugin worker, never on the render thread.

## A visible-only timer

Keep the returned `CallbackResource` until the timer is no longer needed. The target must be an existing widget or surface owned by this plugin.

```rust
use winisland_plugin_api::sdk::{CallbackResource, Error, Host};
use winisland_plugin_api::*;

fn start_timer(host: &Host, target: WidgetId) -> Result<CallbackResource, Error> {
    let log = host.log();
    host.events()?.timer(TimerSpecV2 {
        target,
        flags: TIMER_VISIBLE_ONLY,
        delay_ms: 1000,
        interval_ms: 1000,
        ..Default::default()
    }, move |_| {
        log.write(2, "Visible timer fired");
    })
}
```

## Methods

Raw calls take `context, token` first and return `PluginStatus`.

| SDK method | Raw method and remaining parameters | Behavior |
|---|---|---|
| `subscribe(mask, target, handler)` | `subscribe(*const EventSubscriptionV2, *mut ResourceId)` | Registers a nonzero supported event mask. |
| `timer(spec, handler)` | `create_timer(*const TimerSpecV2, *mut ResourceId)` | Creates a one-shot or repeating timer. |
| `CallbackResource::cancel()` | `release(ResourceId)` | Cancels the resource and removes queued deliveries; returns `LimitExceeded` while its callback is executing. |
| `island_state()` | `island_state(*mut IslandStateV2)` | Reads the current island state; initialize raw output `struct_size` first. |
| `set_animation(target, enabled)` | `set_animation(WidgetId, u8)` | Enables/disables descriptor `on_tick` for this target. |

`subscribe` accepts `EVENT_INPUT`, `EVENT_HOST`, `EVENT_MEDIA`, `EVENT_VISIBILITY`, `EVENT_RESIZE`, and `EVENT_RESULT`. Input, visibility, and resize require a valid owned target. Use `WidgetId::INVALID` for host/media/result subscriptions: those notifications have no specific drawing target. Do not pass `EVENT_ALL` directly; it includes `EVENT_TIMER` and `EVENT_COMMAND`, which are delivered through `timer` and [command registration](/plugin-dev/api/command) instead.

| Event | Fields to read |
|---|---|
| `EVENT_INPUT` | See the [Input payload table](/plugin-dev/api/input). |
| `EVENT_HOST` | Call `island_state()` for the new snapshot. |
| `EVENT_MEDIA` | Call [Media Session `list()`](/plugin-dev/api/media-session) for the new snapshot. |
| `EVENT_VISIBILITY` | `code` is 1 when presented and 0 when removed from presentation. |
| `EVENT_RESIZE` | Raw `x, y`, or SDK `position`, holds logical width and height. |
| `EVENT_TIMER` | `resource` identifies the timer. |
| `EVENT_RESULT` | `sequence` identifies the request; `code` is `PluginStatus::code()` cast to `u32`. |

All deliveries include their callback resource ID and host-relative monotonic `time_seconds`. Raw payload slices last only for the callback; SDK `Event.data` is an owned copy. `IslandStateV2` contains expansion/visibility/theme flags, current width/height/scale, and page ID: 1 music, 2 widgets, 3 calendar, or the current plugin page's `WidgetId`. Use the surface's logical size for drawing layout.

## Scheduling and cleanup

`delay_ms` is the initial delay. `interval_ms = 0` creates a one-shot; repeating intervals must be at least 4 ms. Both values are bounded by 604,800,000 ms (seven days). Use zero flags and `WidgetId::INVALID` for a timer that does not depend on a surface. `TIMER_VISIBLE_ONLY` requires a valid target; overdue work resumes when that target is presented, without replaying every missed interval. A fired one-shot still owns its resource until released.

Animation defaults to enabled. Surfaces tick only while presented; legacy widgets retain their existing ticking behavior. `set_animation(target, false)` stops that target's `on_tick`, not its subscriptions or timers. Draw once initially, redraw on events, and enable continuous ticks only when needed.

Subscriptions and timers share a limit of 128 resources per plugin. Callback queues are bounded at 256 pending deliveries per plugin; adjacent state, resize, visibility, timer, and pointer-move events may coalesce. Keep callbacks short and do not block waiting for another callback on the same worker. Subscribe to results before issuing asynchronous requests and do not assume every intermediate state change will be delivered under load.

SDK handlers are `FnMut(Event) + Send + 'static`. Retain their handles in the instance and cancel them before releasing referenced targets. Do not drop a handle from inside its own callback: release is refused while in flight. If cancellation fails, the SDK retains callback storage for safety; explicit `cancel()` allows retry after the callback returns. Use weak references when a callback refers back to an instance that owns its handle, to avoid an ownership cycle. The host joins its worker before calling plugin `shutdown`; the plugin must still join its own threads.

[All plugin APIs](/plugin-dev/api)
