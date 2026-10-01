# Events API

`EventsApiV2` 提供事件订阅、定时器、岛状态和动画调度。从 crate `0.9.0` 开始提供；声明 `CAP_EVENTS` 并查询 `IFACE_EVENTS`，或使用 `host.events()?`。回调在插件工作线程执行，不在渲染线程执行。

## 仅可见时运行的定时器

把返回的 `CallbackResource` 保存到不再需要定时器时。目标必须是本插件已经创建的小组件或 surface。

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

## 方法

原始调用先接收 `context, token`，返回 `PluginStatus`。

| SDK 方法 | 原始方法及后续参数 | 行为 |
|---|---|---|
| `subscribe(mask, target, handler)` | `subscribe(*const EventSubscriptionV2, *mut ResourceId)` | 注册非零且受支持的事件掩码。 |
| `timer(spec, handler)` | `create_timer(*const TimerSpecV2, *mut ResourceId)` | 创建单次或周期定时器。 |
| `CallbackResource::cancel()` | `release(ResourceId)` | 取消资源并移除排队投递；该回调正在执行时返回 `LimitExceeded`。 |
| `island_state()` | `island_state(*mut IslandStateV2)` | 读取当前岛状态；原始输出必须先初始化 `struct_size`。 |
| `set_animation(target, enabled)` | `set_animation(WidgetId, u8)` | 开关此目标的描述符 `on_tick` 调用。 |

`subscribe` 接受 `EVENT_INPUT`、`EVENT_HOST`、`EVENT_MEDIA`、`EVENT_VISIBILITY`、`EVENT_RESIZE` 和 `EVENT_RESULT`。输入、可见性与尺寸事件要求有效的自有目标；宿主、媒体及结果事件没有具体绘制目标，应使用 `WidgetId::INVALID` 订阅。不能直接传入 `EVENT_ALL`，因为其中还包含 `EVENT_TIMER` 和 `EVENT_COMMAND`，这两类分别通过 `timer` 和[命令注册](/plugin-dev/api/command)回调交付。

| 事件 | 读取什么 |
|---|---|
| `EVENT_INPUT` | 见 [Input 数据表](/plugin-dev/api/input)。 |
| `EVENT_HOST` | 调用 `island_state()` 读取新快照。 |
| `EVENT_MEDIA` | 调用 [Media Session 的 `list()`](/plugin-dev/api/media-session) 读取新快照。 |
| `EVENT_VISIBILITY` | `code` 为 1 表示正在呈现，为 0 表示已退出呈现。 |
| `EVENT_RESIZE` | 原始 `x, y` 或 SDK `position` 是逻辑宽高。 |
| `EVENT_TIMER` | `resource` 标识定时器。 |
| `EVENT_RESULT` | `sequence` 标识请求；`code` 为 `PluginStatus::code()` 转成 `u32` 后的值。 |

每次投递都有对应回调资源 ID 和宿主相对单调时间 `time_seconds`。原始数据切片仅在回调期间有效；SDK 的 `Event.data` 是自有副本。`IslandStateV2` 包含展开、可见、主题标记，以及当前宽高、缩放和页面 ID：1 为音乐，2 为小组件，3 为日历，4 为计时器，插件页则为其 `WidgetId`。绘制布局仍应使用目标自己的逻辑尺寸。

## 调度与清理

`delay_ms` 是首次延迟；`interval_ms = 0` 表示单次，否则间隔至少为 4 ms。两者上限均为 604,800,000 ms，即七天。无需依赖界面可见性的定时器可以使用零标记和 `WidgetId::INVALID`。`TIMER_VISIBLE_ONLY` 必须绑定有效目标；目标重新呈现后会恢复到期回调，但不会逐次补发隐藏期间错过的全部间隔。单次定时器触发后仍占用资源，需要释放。

动画默认启用。Surface 只在被呈现时 tick；旧有网格小组件保留原先的 tick 行为。`set_animation(target, false)` 只关闭此目标的 `on_tick`，不会停止事件订阅或定时器。先提交初始帧，随后按事件重绘，只有需要连续动画时才开启 tick。

订阅和定时器共享每插件 128 个资源的上限。每插件回调队列最多 256 项；相邻的状态、尺寸、可见性、定时器及指针移动事件可能合并。回调应尽快返回，不要阻塞等待同一工作线程中的另一个回调。发起异步请求前先订阅结果，也不要假设高负载时每个中间状态都会被投递。

SDK 处理器的类型为 `FnMut(Event) + Send + 'static`。句柄要保存在实例中，释放关联目标前先取消它们。不要在回调内部丢弃它自己的句柄，执行中的回调会拒绝释放。取消失败时 SDK 会保留回调存储以避免悬空；显式 `cancel()` 可以在回调结束后重试。若实例持有回调句柄，而回调又引用实例，应使用弱引用避免所有权循环。宿主调用插件 `shutdown` 前会停止并等待自己的工作线程；插件仍需停止并等待自己创建的线程。

[全部插件 API](/plugin-dev/api)
