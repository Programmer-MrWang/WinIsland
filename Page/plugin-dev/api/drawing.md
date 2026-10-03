# Drawing SDK

The drawing SDK builds complete ABI v2 frames for [Widget](/plugin-dev/api/widget) and [Surface](/plugin-dev/api/surface), including activity compact surfaces. It does not query another service table: declare `CAP_WIDGET` or `CAP_SURFACE` for the resource receiving the frame. Use [Text](/plugin-dev/api/text) for metrics and [Image](/plugin-dev/api/image) when creating reusable image handles.

## Build and submit a frame

Keep the builder alive until submission; `finish()` returns a borrowed `DrawList`. This helper works with a `Widget` or a `Surface` through its `Widget` drawing methods.

```rust
use winisland_plugin_api::sdk::{
    DrawListBuilder, Error, Rect, Rgba, Size, TextStyle, Widget,
};

fn draw_status(widget: &Widget) -> Result<(), Error> {
    let (width, height) = widget.logical_size();
    if width <= 0.0 || height <= 0.0 {
        return Ok(());
    }
    let mut frame = DrawListBuilder::new(Size::new(width, height));
    frame.fill_round_rect(Rect::new(0.0, 0.0, width, height), 8.0,
        Rgba::from_argb(0xff20_2838).with_alpha(180));
    frame.text("Ready", Rect::new(8.0, 4.0, width - 16.0, height - 8.0),
        &TextStyle::default_at(13.0).bold(), Rgba::WHITE);
    widget.submit(frame.finish())
}
```

Submission copies the complete frame into the host. A later frame replaces the previous one; `request_redraw()` only asks for replay of existing content. Build a new list when content changes and read logical size before layout.

## Values and helpers

| Type or method | Contract |
|---|---|
| `Size::new(width, height)` | The logical coordinate size encoded in the frame header. |
| `Rect::new(x, y, width, height)` | A logical rectangle; all values must be finite and dimensions nonnegative. |
| `Rgba::WHITE` | Opaque white. |
| `Rgba::from_argb(u32)`, `argb()` | Construct/read a packed `0xAARRGGBB` color. |
| `Rgba::with_alpha(u8)` | Since `0.10.0`: replace alpha in 0–255 while preserving RGB. This does not multiply the previous alpha. |
| `TextStyle::default_at(size)` | Positive font size, weight 400, upright, left-aligned, no wrapping or ellipsis, empty family for the host default. |
| `TextStyle::bold()` | Returns the style with weight 700. Other fields can be set directly: `size`, `weight`, `italic`, `align`, `wrap`, `ellipsis`, `family`. |
| `TextRun` | Borrowed UTF-8 `text`, `color`, and `weight` for one run in `text_runs`. |
| `GradientStop` | `position` in 0–1 and `color`; stops must be in nondecreasing order. |
| `Deg12(f32)` | Angle wrapper accepted by `stroke_arc`; values are expressed in degrees. |
| `ImageFit::{Contain, Cover, Fill}` | Fit the image inside, crop it to cover, or stretch it to the destination rectangle. |
| `DrawListBuilder::new(Size)` | Starts an empty frame with that logical size. |
| `DrawListBuilder::finish()` | Returns a borrowed view of its bytes without consuming the builder. |
| `DrawList::as_bytes()` | Reads the encoded bytes for a raw `submit_draw_list` call. |

Text alignment is 0 left, 1 center, or 2 right. Use `host.text()?.measure_style(text, &style)` with `CAP_TEXT` when measuring the same font you draw; it measures font metrics, not the final wrapping, ellipsis, or alignment inside a rectangle.

## Drawing commands

All commands below are `DrawListBuilder` methods returning `&mut Self` for chaining. They append commands locally; validation happens in the host.

| SDK method | Protocol opcode | Parameters and behavior |
|---|---|---|
| `clip_rect(rect)` | `PUSH_CLIP_RECT` | Push a rectangular clip. |
| `clip_round_rect(rect, radius)` | `PUSH_CLIP_ROUND_RECT` | Push a rounded clip. |
| `pop_clip()` | `POP_CLIP` | Restore the preceding clip. |
| `transform(affine)` | `PUSH_TRANSFORM` | Push a six-component 2D affine transform. |
| `pop_transform()` | `POP_TRANSFORM` | Restore the preceding transform. |
| `alpha(value)` | `PUSH_ALPHA` | Push an opacity multiplier in 0–255. |
| `pop_alpha()` | `POP_ALPHA` | Restore the preceding opacity. |
| `fill_rect(rect, color)` | `FILL_RECT` | Fill a rectangle. |
| `fill_round_rect(rect, radius, color)` | `FILL_ROUND_RECT` | Fill a rounded rectangle. |
| `fill_circle(x, y, radius, color)` | `FILL_CIRCLE` | Fill a circle at the supplied center. |
| `fill_convex_polygon(points, color)` | `FILL_CONVEX_POLYGON` | Fill 3–1024 `(x, y)` points forming a convex polygon. |
| `fill_linear_gradient(rect, angle, stops)` | `FILL_LINEAR_GRADIENT` | Fill with 2–16 ordered `GradientStop` values; angle is in degrees. |
| `stroke_line(from, to, width, color)` | `STROKE_LINE` | Draw a line with positive stroke width. |
| `stroke_round_rect(rect, radius, width, color)` | `STROKE_ROUND_RECT` | Stroke a rounded rectangle. |
| `stroke_arc(rect, start, sweep, width, color)` | `STROKE_ARC` | Stroke an arc using `Deg12` start and sweep values. |
| `image(id, rect, fit)` | `DRAW_IMAGE` | Draw an owned `ImageId`; keep its image handle alive through frame preparation. |
| `image_pixels(rgba, dimensions, rect, fit)` | `DRAW_IMAGE_PIXELS` | Embed RGBA bytes for `(width, height)` directly in the frame. |
| `text(value, rect, style, color)` | `DRAW_TEXT` | Draw UTF-8 text with one style and color. |
| `text_runs(runs, rect, style)` | `DRAW_TEXT_RUNS` | Draw runs with per-run color and weight, sharing the remaining style. |
| `shadow_round_rect(rect, radius, sigma, offset, color)` | `DRAW_SHADOW_ROUND_RECT` | Draw a rounded-rectangle shadow with blur and `(x, y)` offset. |

Clips, transforms, and alpha have separate stacks, each limited to 64 entries. Match every push with its corresponding pop before finishing the list. Drawing stays clipped to the host's widget or surface presentation.

## Protocol and limits

The wire definitions are in `winisland_plugin_api::draw::v2`. `MAGIC` is `0x57494432` and `VERSION` is 2. Encode numbers in little-endian order: the 24-byte `DrawListHeader` contains `magic`, `version`, `logical_w`, `logical_h`, `command_count`, and `payload_len`; each eight-byte `DrawCommandHeader` contains `opcode`, zero `flags`, and `payload_len`. The SDK writes these headers and counts for you.

Limits are `MAX_LIST_BYTES` = 4 MiB, `MAX_COMMANDS` = 4096, `MAX_PAYLOAD_BYTES` = 1 MiB per command, `MAX_TEXT_BYTES` = 64 KiB per text value, and `MAX_FAMILY_BYTES` = 255 UTF-8 bytes. The SDK truncates overlong font families at a UTF-8 boundary; it does not make other oversized or invalid commands valid. Inline pixels require nonzero dimensions no greater than 4096 × 4096 and exactly `width × height × 4` bytes, while also fitting the per-command payload limit.

A successful submission is not proof that validation or presentation succeeded. Invalid opcodes, nonfinite geometry, unbalanced stacks, invalid handles, or over-limit payloads are rejected during preparation; repeated invalid frames can disable the target. See the [SDK implementation](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/sdk/draw.rs) and [wire definitions](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/draw/v2.rs).

[All plugin APIs](/plugin-dev/api)
