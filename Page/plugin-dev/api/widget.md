# Widget API

`WidgetApiV2` creates grid widgets and accepts complete draw lists. Declare `CAP_WIDGET` and query `IFACE_WIDGET`. The SDK provides `host.widgets()?.create(WidgetSpec::new("key").span(2, 1))`, a `Widget` handle, and `DrawListBuilder`.

## Methods

Each method takes `context, token` first and returns `PluginStatus`.

| Method | Remaining parameters | Result |
|---|---|---|
| `create` | `*const WidgetSpecV2`, `*mut WidgetId` | Registers a widget and writes its ID. |
| `update` | `WidgetId`, `*const WidgetSpecV2` | Replaces its specification. |
| `release` | `WidgetId` | Removes the widget. |
| `submit_draw_list` | `WidgetId`, byte pointer, byte length | Copies a complete frame for later validation. |
| `request_redraw` | `WidgetId` | Marks the widget for a redraw. |
| `logical_size` | `WidgetId`, width output, height output | Returns the current logical drawing size. |

## Specification and drawing

`WidgetSpecV2` supplies `span_cols`, `span_rows`, `flags`, `title`, `body`, a stable `key`, and minimum dimensions. Each span must be 1–4 cells; dimensions must be finite and nonnegative. `WIDGET_FLAG_SHOW_COMPACT` is the only current flag. Stable keys preserve configured placement across restarts.

Create a new complete list with `DrawListBuilder::new(Size::new(width, height))`, add drawing commands, and submit `finish()`. The protocol supports clips, transforms, alpha, shapes, gradients, strokes, shadows, images, and text. Read `logical_size` before layout; the SDK returns `(0, 0)` on a failed query. The expanded grid determines that size even during collapse animation.

Submission success means the bytes were copied. The host validates and prepares the list later, then clips and replays it inside the widget's grid area. A list is limited to 4 MiB and 4096 commands; repeated malformed frames can disable the widget. The current quota is eight widgets per plugin. `PluginDescriptorV2.on_tick` runs on a plugin worker and can be used to build new frames. The current Widget API has no pointer or keyboard event callback.

See the [widget example](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/examples/minimal_widget.rs) and [draw protocol](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/draw/v2.rs).

[All plugin APIs](/plugin-dev/api)
