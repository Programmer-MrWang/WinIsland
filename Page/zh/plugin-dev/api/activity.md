# Activity API

`ActivityApiV2` 注册可自绘的活动，绑定主紧凑 surface、可选展开页、优先级、超时、期望尺寸，以及无活动自动隐藏的保持显示声明。该接口从 crate `0.11.0` 起提供，需要 `CAP_ACTIVITY` 和 `IFACE_ACTIVITY`；SDK 入口为 `host.activities()?`。创建绘制 surface 时，还需通过 [Surface](/plugin-dev/api/surface) 使用 `CAP_SURFACE`。

音乐、原生计时器、新插件活动，以及旧 [Context](/plugin-dev/api/context) 文字均由同一套宿主机制选择。需要自绘和关联页面时使用 Activity；无需 surface 的文字状态仍可使用 Context。

## 创建活动和展开页

此辅助函数需要 `CAP_ACTIVITY | CAP_SURFACE`。将返回的三个句柄都保存在插件实例中；数据变化时提交新帧。额外声明 `CAP_EVENTS` 后，可通过 [Events](/plugin-dev/api/events) 接收尺寸通知或安排刷新。

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

由宿主决定紧凑内容何时显示。点击紧凑岛，会打开当前选中活动关联且已启用的展开页。[完整下载示例](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/examples/activity.rs) 还会更新进度，并让完成状态在四秒后到期。

## 方法

原始调用先接收 `context, token`，并返回 `PluginStatus`。

| SDK 方法 | 原始方法与后续参数 | 行为 |
|---|---|---|
| `Activities::create(spec)` | `create(*const ActivitySpecV2, *mut ResourceId)` | 创建活动并返回归插件所有的 ID。 |
| `Activity::update(spec)` | `update(ResourceId, *const ActivitySpecV2)` | 替换声明，并刷新更新时间和到期时间。 |
| 销毁 `Activity` | `release(ResourceId)` | 移除活动；绑定的 surface 仍是独立资源。 |
| `Activity::id()` | 不调用宿主 | 返回活动的 `ResourceId`，不是绘制用的 `WidgetId`。 |

配置 [Input](/plugin-dev/api/input) 区域或订阅事件时，目标应为 `compact.id()` 或 `page.id()`，而非 `activity.id()`。每个插件最多拥有 64 个活动，与 64 个旧 Context 的配额分开计算。绘制 surface 仍与小组件共享八个资源的配额。

## 声明与选择

通过 `..Default::default()` 初始化 `ActivitySpecV2`，以正确设置 `struct_size`。默认声明为已启用、普通优先级、无超时的活动，但仍须提供有效的 `compact_surface`。

| 字段 | 约定 |
|---|---|
| `flags` | `ACTIVITY_ENABLED` 使活动可参与选择；`ACTIVITY_KEEP_VISIBLE` 在活动仍可参与时阻止无活动自动隐藏。不接受其他位。 |
| `priority` | `PRIORITY_LOW`、`PRIORITY_MEDIUM` 或 `PRIORITY_HIGH`；优先级高的先显示。 |
| `timeout_ms` | 从创建或最近一次更新起计时；零表示持续保留。到期停止显示，但不会释放资源。 |
| `compact_surface` | 同一插件自己的 `SURFACE_COMPACT_MAIN` 资源。 |
| `expanded_page` | 插件自己的 `SURFACE_PAGE`；无关联页面时使用 `WidgetId::INVALID`。 |
| `preferred_width`、`preferred_height` | 主紧凑内容的逻辑期望尺寸，须为 0–2048 范围内的有限数值。零使用紧凑 surface 的请求尺寸；宿主至少保留配置的紧凑模式最小尺寸。 |

可参与显示的活动先比较优先级，再比较最近更新时间，最后比较资源 ID。绘制帧或收到 tick 不会刷新活动的更新时间；声明或到期时间变化时，应更新现有活动。当前原生优先级为：播放中的音乐高，暂停的音乐低，计时器普通。宿主为有效活动预留足够窗口空间；实际绘制尺寸由 `Surface::logical_size()` 提供。

即使另一个活动被选中，保持显示声明仍可阻止无活动自动隐藏。手动隐藏、全屏隐藏，以及用户隐藏组件的状态仍优先。清除 `ACTIVITY_ENABLED` 会保留资源，但停止参与选择与保持显示。

## 生命周期与不可用视图

活动不会持有或释放绑定的 surface。应保留它们的句柄，并在关闭插件时先释放活动，再释放 surface。紧凑 surface 被禁用、释放或因绘制失败而禁用后，活动不再参与选择和保持显示；展开页不可用只会移除该活动的点击展开目标。活动尚未到期时，重新启用 surface 或更新活动可恢复参与资格。

到期后活动句柄仍有效，也仍占用配额；更新它可刷新期限，结束后则应释放。插件关闭时，宿主撤销遗留的活动与 surface。资源 ID 只在运行期间有效，不应跨重启保存。

旧宿主可能拒绝 `CAP_ACTIVITY` 或 `SURFACE_COMPACT_MAIN`；仅支持 ABI v2 不代表实现了这些新增能力。参见[活动类型](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/types/v2/activity.rs)和 [SDK](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/sdk/activity.rs)。

[返回 API 目录](/plugin-dev/api)
