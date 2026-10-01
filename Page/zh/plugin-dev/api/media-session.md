# Media Session API

`MediaSessionApiV2` 枚举并控制现有原生 SMTC 会话和插件媒体源。从 crate `0.9.0` 开始提供；声明 `CAP_MEDIA_SESSION` 并查询 `IFACE_MEDIA_SESSION`，或使用 `host.media_sessions()?`。若要由自己的插件发布新媒体源，请使用 [Media API](/plugin-dev/api/media)。

## 暂停当前会话

以下函数还需要 `CAP_EVENTS`。它先订阅结果，再将操作入队，返回订阅句柄和请求序号。应保留句柄，并在实例状态中用序号匹配结果。

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

示例回调会记录发送给本插件的全部结果，不只这一条暂停请求。不要在另一个插件回调中同步等待它完成。

## 方法与快照

原始调用先接收 `context, token`，返回 `PluginStatus`。

| SDK 方法 | 原始方法及后续参数 | 行为 |
|---|---|---|
| `list()` | `list(*mut MediaSessionV2, capacity, *mut required)` | 返回会话快照副本。 |
| `send(session, command, position_ms)` | `send(u64, u32, u64, *mut sequence)` | 操作入队并返回请求序号。 |

原始 `list` 用空缓冲区和零容量查询时，返回 `Ok` 并写入所需元素数量；非空缓冲区不足时返回 `LimitExceeded`。SDK 负责分配并重试。收到 `EVENT_MEDIA` 后重新枚举，事件本身只是更新信号，不携带完整会话快照。

`MediaSessionV2` 包含运行期 `id`、标记、`available_controls`、毫秒时长/进度、宿主相对单调时间 `sampled_at_seconds`，以及 NUL 结尾 UTF-8 的 `source`、`title`、`artist`、`album`。标记为 `SESSION_PLAYING`、`SESSION_CURRENT`、`SESSION_PLUGIN`。控制掩码使用已有的 `MEDIA_CONTROL_TOGGLE_PLAY`、`MEDIA_CONTROL_PREVIOUS`、`MEDIA_CONTROL_NEXT`、`MEDIA_CONTROL_SEEK` 位，不是 `SESSION_*` 命令数字的掩码；播放和暂停没有各自独立的能力位。

原生会话发现遵循宿主的 SMTC 启用设置。空列表可能表示没有播放器提供会话，或原生发现已关闭；插件媒体源仍独立加入列表。ID 仅在运行期间有效，会话消失后应重新枚举，不要跨进程重启保存 ID。

## 命令与选择

| 命令 | 含义 |
|---|---|
| `SESSION_TOGGLE` | 切换播放状态 |
| `SESSION_PLAY` / `SESSION_PAUSE` | 请求明确的播放或暂停状态 |
| `SESSION_PREVIOUS` / `SESSION_NEXT` | 切换曲目 |
| `SESSION_SEEK` | 跳转到 `position_ms` 指定的绝对位置 |
| `SESSION_SELECT` | 选择岛使用的会话 |

播放操作中的会话 ID `0` 表示当前选中或自动选择的源。`send(0, SESSION_SELECT, 0)` 恢复自动选择。显式选择会影响岛展示的媒体和默认控制目标，但选择本身不会开始播放。负责选择的插件卸载后，会释放该选择。除跳转外，`position_ms` 使用 `0`。

`send()` 返回成功只表示接受处理。用返回序号匹配 `EVENT_RESULT.sequence`，从 `code` 读取状态。会话可能在枚举后消失，返回 `StaleHandle`；不支持的操作可能返回 `InvalidArgument`，平台失败可能返回 `IoError`。收到结果后刷新状态，不要假设播放状态已同步改变。对于插件媒体源，明确播放/暂停在需要改变状态时会使用既有切换回调；完成结果确认回调已交付，不保证插件控制的外部播放器工作已经完成。

[全部插件 API](/plugin-dev/api)
