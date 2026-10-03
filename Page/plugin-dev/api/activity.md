# Activity API

`ActivityApiV2` registers an activity with a main compact surface, optional expanded page, priority, timeout, preferred dimensions, and an inactivity auto-hide hold. Added in crate `0.11.0`, it requires `CAP_ACTIVITY` and `IFACE_ACTIVITY`; the SDK entry point is `host.activities()?`. Create its drawing surfaces through [Surface](/plugin-dev/api/surface) with `CAP_SURFACE`.

Music, the native timer, new plugin activities, and legacy [Context](/plugin-dev/api/context) text participate in one host selection mechanism. Use Activity for custom drawing and navigation; Context remains available for text without surfaces.

## Create an activity and its page

This helper requires `CAP_ACTIVITY | CAP_SURFACE`. Keep all three returned handles in the plugin instance. Submit new frames when the activity's data changes; [Events](/plugin-dev/api/events) can provide resize notifications or scheduled updates when `CAP_EVENTS` is also declared.

```rust
use winisland_plugin_api::sdk::{
    Activity, DrawListBuilder, Error, Host, Rect, Rgba, Size, Surface, TextStyle,
};
use winisland_plugin_api::*;

fn start_download(host: &Host) -> Result<(Activity, Surface, Surface), Error> {
    let surfaces = host.surfaces()?;
    let compact = surfaces.create(SurfaceSpecV2 {
        key: str_to_fixed("download-compact"),
        title: str_to_fixed("Download"),
        kind: SURFACE_COMPACT_MAIN,
        flags: SURFACE_ENABLED,
        width: 220.0,
        height: 36.0,
        ..Default::default()
    })?;
    let page = surfaces.create(SurfaceSpecV2 {
        key: str_to_fixed("download-page"),
        title: str_to_fixed("Download details"),
        kind: SURFACE_PAGE,
        flags: SURFACE_ENABLED,
        width: 360.0,
        height: 200.0,
        ..Default::default()
    })?;
    for surface in [&compact, &page] {
        let (w, h) = surface.logical_size();
        if w > 0.0 && h > 0.0 {
            let mut frame = DrawListBuilder::new(Size::new(w, h));
            frame.text("Download 0%", Rect::new(12.0, 4.0, w - 24.0, h - 8.0),
                &TextStyle::default_at(13.0), Rgba::WHITE);
            surface.submit(frame.finish())?;
        }
    }
    let activity = host.activities()?.create(ActivitySpecV2 {
        flags: ACTIVITY_ENABLED | ACTIVITY_KEEP_VISIBLE,
        priority: PRIORITY_MEDIUM,
        compact_surface: compact.id(),
        expanded_page: page.id(),
        ..Default::default()
    })?;
    Ok((activity, compact, page))
}
```

The host chooses when the compact content is presented. Clicking the compact island opens the selected activity's enabled expanded page. The complete [download example](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/examples/activity.rs) also updates progress and gives completion a four-second expiry.

## Methods

Raw calls take `context, token` first and return `PluginStatus`.

| SDK method | Raw method and remaining parameters | Behavior |
|---|---|---|
| `Activities::create(spec)` | `create(*const ActivitySpecV2, *mut ResourceId)` | Creates an owned activity and returns its ID. |
| `Activity::update(spec)` | `update(ResourceId, *const ActivitySpecV2)` | Replaces its declaration and refreshes update time and expiry. |
| Drop the `Activity` | `release(ResourceId)` | Removes the activity; its surfaces remain separate resources. |
| `Activity::id()` | No host call | Returns the activity's `ResourceId`, not a drawing `WidgetId`. |

For [Input](/plugin-dev/api/input) regions or event subscriptions, target `compact.id()` or `page.id()`, not `activity.id()`. The host allows 64 activity resources per plugin, separately from 64 legacy Context resources. Drawing surfaces share the existing eight-resource widget/surface quota.

## Declaration and selection

Initialize `ActivitySpecV2` with `..Default::default()` to set `struct_size`. The default enables a medium-priority persistent activity; it still needs a valid `compact_surface`.

| Field | Contract |
|---|---|
| `flags` | `ACTIVITY_ENABLED` makes the activity eligible. `ACTIVITY_KEEP_VISIBLE` prevents inactivity auto-hide while it remains eligible. No other bits are accepted. |
| `priority` | `PRIORITY_LOW`, `PRIORITY_MEDIUM`, or `PRIORITY_HIGH`; higher priority wins. |
| `timeout_ms` | Time since creation or the last update; zero means persistent. Expiry stops presentation without releasing the resource. |
| `compact_surface` | A `SURFACE_COMPACT_MAIN` resource owned by the same plugin. |
| `expanded_page` | An owned `SURFACE_PAGE`, or `WidgetId::INVALID` when there is no associated page. |
| `preferred_width`, `preferred_height` | Finite values in 0–2048 logical units for the main compact content. Zero uses the compact surface's requested dimension; the host keeps at least its configured compact minimum. |

Eligible activities compete by priority, then most recent update time, then resource ID. Drawing a frame or receiving a tick does not refresh the activity's update time. Update the existing activity when its declaration or expiry needs to change. Current native priorities are high for playing music, low for paused music, and medium for the timer. The host reserves enough window space for eligible activities and supplies the actual drawing size through `Surface::logical_size()`.

The inactivity auto-hide hold applies even while another activity wins selection. Manual hiding, fullscreen hiding, and the user's hidden-component state still take precedence. Clearing `ACTIVITY_ENABLED` keeps the resource but removes it from selection and the hold.

## Lifetime and unavailable surfaces

An activity does not retain or release its bound surfaces. Keep their handles alive and release the activity before its surfaces during shutdown. A disabled, released, or failed compact surface removes the activity from selection and auto-hide holding; an unavailable expanded page only removes that activity's click-to-open destination. Re-enabling a surface or updating the activity can restore eligibility if it has not expired.

Expiry leaves the activity handle valid and still consumes its quota. Update it to renew the deadline, or release it when finished. Plugin shutdown revokes any remaining activities and surfaces. Resource IDs are transient and must not be saved across restarts.

Older hosts can reject `CAP_ACTIVITY` or `SURFACE_COMPACT_MAIN`; ABI v2 alone does not promise these additions. See the [activity types](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/types/v2/activity.rs) and [SDK](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/sdk/activity.rs).

[All plugin APIs](/plugin-dev/api)
