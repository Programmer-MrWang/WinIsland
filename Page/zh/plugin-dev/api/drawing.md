# 绘制 SDK

绘制 SDK 为 [Widget](/plugin-dev/api/widget) 和 [Surface](/plugin-dev/api/surface) 构建完整 ABI v2 帧，也适用于活动的紧凑 surface。它没有额外的服务表；根据接收帧的资源声明 `CAP_WIDGET` 或 `CAP_SURFACE`。文字度量由 [Text](/plugin-dev/api/text) 提供，可复用图片句柄由 [Image](/plugin-dev/api/image) 创建。

## 构建并提交一帧

保留 builder 直到提交结束；`finish()` 返回借用的 `DrawList`。此辅助函数可接收 `Widget`，也可通过 `Surface` 的 `Widget` 绘制方法使用。

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

提交时，宿主复制完整帧。新帧会替换上一帧；`request_redraw()` 只请求重放已有内容。内容变化时重新构建列表，布局前先读取逻辑尺寸。

## 数据与辅助方法

| 类型或方法 | 约定 |
|---|---|
| `Size::new(width, height)` | 写入帧头的逻辑坐标尺寸。 |
| `Rect::new(x, y, width, height)` | 逻辑矩形；所有值须有限，宽高须非负。 |
| `Rgba::WHITE` | 不透明白色。 |
| `Rgba::from_argb(u32)`、`argb()` | 构造或读取 `0xAARRGGBB` 格式的打包颜色。 |
| `Rgba::with_alpha(u8)` | 从 `0.10.0` 起提供：用 0–255 的值替换透明度，保留 RGB；不会与旧透明度相乘。 |
| `TextStyle::default_at(size)` | 正数字号、400 字重、正体、左对齐、不换行、不省略；空字体族表示宿主默认字体。 |
| `TextStyle::bold()` | 返回字重为 700 的样式。其余字段可直接设置：`size`、`weight`、`italic`、`align`、`wrap`、`ellipsis`、`family`。 |
| `TextRun` | `text_runs` 中某段的借用 UTF-8 `text`、`color` 和 `weight`。 |
| `GradientStop` | 0–1 范围的 `position` 和 `color`；各点位置须按非递减顺序排列。 |
| `Deg12(f32)` | `stroke_arc` 接受的角度包装类型，数值单位为度。 |
| `ImageFit::{Contain, Cover, Fill}` | 完整容纳、裁剪铺满或拉伸至目标矩形。 |
| `DrawListBuilder::new(Size)` | 以指定逻辑尺寸创建空帧。 |
| `DrawListBuilder::finish()` | 返回字节的借用视图，不消耗 builder。 |
| `DrawList::as_bytes()` | 读取编码后的字节，用于原始 `submit_draw_list` 调用。 |

文字对齐值为 0 左、1 中、2 右。需要按绘制字体测量时，声明 `CAP_TEXT` 并使用 `host.text()?.measure_style(text, &style)`；它测量字体度量，不模拟矩形中的最终换行、省略或对齐。

## 绘制命令

以下都是 `DrawListBuilder` 的方法，返回 `&mut Self`，支持链式调用。它们只在本地追加命令，由宿主完成校验。

| SDK 方法 | 协议操作码 | 参数与行为 |
|---|---|---|
| `clip_rect(rect)` | `PUSH_CLIP_RECT` | 压入矩形裁剪。 |
| `clip_round_rect(rect, radius)` | `PUSH_CLIP_ROUND_RECT` | 压入圆角裁剪。 |
| `pop_clip()` | `POP_CLIP` | 恢复上一层裁剪。 |
| `transform(affine)` | `PUSH_TRANSFORM` | 压入六个分量的二维仿射变换。 |
| `pop_transform()` | `POP_TRANSFORM` | 恢复上一层变换。 |
| `alpha(value)` | `PUSH_ALPHA` | 压入 0–255 范围的透明度乘数。 |
| `pop_alpha()` | `POP_ALPHA` | 恢复上一层透明度。 |
| `fill_rect(rect, color)` | `FILL_RECT` | 填充矩形。 |
| `fill_round_rect(rect, radius, color)` | `FILL_ROUND_RECT` | 填充圆角矩形。 |
| `fill_circle(x, y, radius, color)` | `FILL_CIRCLE` | 按中心坐标填充圆。 |
| `fill_convex_polygon(points, color)` | `FILL_CONVEX_POLYGON` | 填充由 3–1024 个 `(x, y)` 点组成的凸多边形。 |
| `fill_linear_gradient(rect, angle, stops)` | `FILL_LINEAR_GRADIENT` | 用 2–16 个有序 `GradientStop` 填充渐变，角度单位为度。 |
| `stroke_line(from, to, width, color)` | `STROKE_LINE` | 以正数描边宽度绘制线段。 |
| `stroke_round_rect(rect, radius, width, color)` | `STROKE_ROUND_RECT` | 描边圆角矩形。 |
| `stroke_arc(rect, start, sweep, width, color)` | `STROKE_ARC` | 使用 `Deg12` 起始角和扫过角描边圆弧。 |
| `image(id, rect, fit)` | `DRAW_IMAGE` | 绘制插件自己的 `ImageId`；图片句柄应保留至帧准备结束。 |
| `image_pixels(rgba, dimensions, rect, fit)` | `DRAW_IMAGE_PIXELS` | 将 `(width, height)` 对应的 RGBA 字节直接嵌入帧。 |
| `text(value, rect, style, color)` | `DRAW_TEXT` | 用统一样式和颜色绘制 UTF-8 文字。 |
| `text_runs(runs, rect, style)` | `DRAW_TEXT_RUNS` | 各段可分别指定颜色和字重，其余样式共用。 |
| `shadow_round_rect(rect, radius, sigma, offset, color)` | `DRAW_SHADOW_ROUND_RECT` | 绘制带模糊及 `(x, y)` 偏移的圆角矩形阴影。 |

裁剪、变换和透明度分别拥有独立栈，各最多 64 层。结束列表前，每次压栈都应有对应的弹栈。绘制结果仍被裁剪在宿主分配的小组件或 surface 区域内。

## 协议与限制

协议定义位于 `winisland_plugin_api::draw::v2`。`MAGIC` 为 `0x57494432`，`VERSION` 为 2，数字以小端序编码。24 字节的 `DrawListHeader` 包含 `magic`、`version`、`logical_w`、`logical_h`、`command_count` 和 `payload_len`；每条命令的八字节 `DrawCommandHeader` 包含 `opcode`、须为零的 `flags` 和 `payload_len`。SDK 会代写这些头部与计数。

限制为：`MAX_LIST_BYTES` = 4 MiB，`MAX_COMMANDS` = 4096，每条命令的 `MAX_PAYLOAD_BYTES` = 1 MiB，每个文字值的 `MAX_TEXT_BYTES` = 64 KiB，以及 `MAX_FAMILY_BYTES` = 255 个 UTF-8 字节。SDK 会按 UTF-8 字符边界截断过长字体族，但不会自动修复其他超限或无效命令。内联像素要求非零尺寸、宽高不超过 4096 × 4096，并恰好有 `width × height × 4` 字节；同时还须满足单条命令载荷上限。

提交成功不代表已通过校验或最终显示。未知操作码、非有限几何值、不平衡的栈、无效句柄或超限载荷会在准备帧时被拒绝；反复提交无效帧可能禁用目标。参见 [SDK 实现](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/sdk/draw.rs)和[协议定义](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/draw/v2.rs)。

[返回 API 目录](/plugin-dev/api)
