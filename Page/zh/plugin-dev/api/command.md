# Command API

`CommandApiV2` 注册插件命令、可选的托盘入口和全局快捷键，并执行宿主或插件命令。从 crate `0.9.0` 开始提供；声明 `CAP_COMMAND` 并查询 `IFACE_COMMAND`，或使用 `host.commands()?`。原生托盘与快捷键接入由宿主负责。

## 注册命令

```rust
use winisland_plugin_api::sdk::{CallbackResource, Error, Host};
use winisland_plugin_api::*;

fn register_ping(host: &Host) -> Result<CallbackResource, Error> {
    let log = host.log();
    host.commands()?.register(CommandSpecV2 {
        key: str_to_fixed("ping"),
        title: str_to_fixed("My plugin: Ping"),
        flags: COMMAND_ENABLED | COMMAND_TRAY,
        hotkey_modifiers: MOD_CONTROL | MOD_ALT,
        hotkey_key: u32::from('P'),
        ..Default::default()
    }, move |event| {
        log.write(2, &format!("Ping received: {} bytes", event.data.len()));
    })
}
```

保留返回的句柄。假设插件 ID 为 `my-plugin`，最终命令 ID 就是 `my-plugin.ping`。回调在插件工作线程收到 `EVENT_COMMAND`，无需另外创建 Events 订阅；只有需要订阅完成结果时，才额外声明 `CAP_EVENTS`。

## 方法

原始调用先接收 `context, token`，返回 `PluginStatus`。

| SDK 方法 | 原始方法及后续参数 | 行为 |
|---|---|---|
| `register(spec, handler)` | `register(*const CommandSpecV2, *mut ResourceId)` | 创建自有命令及其回调。 |
| `handle.set_enabled(bool)` | `set_enabled(ResourceId, u8)` | 开关命令；此辅助方法仅适用于命令句柄。 |
| `list()` | `list(*mut CommandInfoV2, capacity, *mut required)` | 列出内建命令及插件命令，包括被禁用的命令。 |
| `execute(id, data)` | `execute(Utf8Slice, ByteSlice, *mut sequence)` | 将已启用命令入队，返回请求序号。 |
| `handle.cancel()` | `release(ResourceId)` | 移除命令、回调、托盘入口和快捷键绑定。 |

键包含 1–63 个 ASCII 字母、数字、`_`、`-` 或 `.`，宿主自动加上插件 ID 前缀。标题不能为空，最多 255 个 UTF-8 字节；使用 `str_to_fixed` 和带大小的默认值初始化。重复 ID 会被拒绝。标记只有 `COMMAND_ENABLED` 与 `COMMAND_TRAY`；每插件最多 64 个命令。

`hotkey_key = 0` 表示不注册快捷键；否则使用 ASCII 字母或数字，配合 `MOD_CONTROL`、`MOD_ALT`、`MOD_SHIFT`、`MOD_SUPER`。快捷键稍后在 UI 线程绑定，命令注册成功不代表 Windows 已接受组合键。绑定失败会写入日志，并通过 `EVENT_RESULT` 报告：`code = PluginStatus::IoError.code() as u32`，`sequence = command_handle.id().get()`。

## 执行与结果

执行数据会被复制，最多 64 KiB；命令 ID 最多 160 个 UTF-8 字节。未知或禁用命令返回 `InvalidArgument`。`execute()` 成功只表示请求已入队。应先用 `WidgetId::INVALID` 订阅 `EVENT_RESULT`，再执行并按返回的 `sequence` 匹配结果。命令回调的 `data` 是执行数据；完成结果表示回调已返回，不代表回调启动的其他工作也已结束。托盘和快捷键触发的回调没有调用方请求需要完成。

原始 `list` 可用空缓冲区和零容量查询元素数量，此时返回 `Ok`；非空但容量不足的输出返回 `LimitExceeded`。两次读取间列表增长时，SDK 会重试。

| 内建 ID | 操作 |
|---|---|
| `winisland.expand` | 展开岛 |
| `winisland.collapse` | 收起岛 |
| `winisland.settings` | 打开设置 |
| `winisland.media.toggle` | 切换播放/暂停 |
| `winisland.media.next` | 下一首 |
| `winisland.media.previous` | 上一首 |

内建命令忽略执行数据。需要指定播放器或确认其操作结果时，使用 [Media Session](/plugin-dev/api/media-session)。回调执行期间释放命令会返回 `LimitExceeded`；清理方式见[回调生命周期](/plugin-dev/api/events)。插件卸载后，其注册内容会被移除。

[全部插件 API](/plugin-dev/api)
