# Media API

`MediaApiV2` 发布仅供显示的“正在播放”媒体源。声明 `CAP_MEDIA` 并查询 `IFACE_MEDIA`。SDK `host.media()?.create_source(title, artist)` 创建基础媒体源；进度、封面、播放状态和控制回调使用原始服务表。

## 方法

所有方法先接收 `context, token`，再接收下表参数，并返回 `PluginStatus`。

| 方法 | 其余参数 | 作用 |
|---|---|---|
| `create` | `*const MediaSourceDataV2`、`*mut ResourceId` | 发布媒体源并写入 ID。 |
| `update` | `ResourceId`、`*const MediaSourceDataV2` | 替换媒体源数据。 |
| `release` | `ResourceId` | 移除媒体源。 |
| `current_title` | 字节缓冲区、容量、所需长度输出 | 复制当前媒体标题的 UTF-8 字节。 |

## 媒体源与命令

`MediaSourceDataV2` 包含必填标题、可选艺术家和专辑、`duration_ms`、`position_ms`、`MEDIA_FLAG_PLAYING`、声明的 `MEDIA_CONTROL_*` 位，以及可选的 JPEG 或 PNG 封面。宿主在创建或更新时复制封面；单份封面最多 16 MiB。控制掩码不为零时必须提供 `on_command`，并保持 `callback_data` 有效。

回调在插件工作线程收到 `MediaCommandV2`，命令可为播放切换、上一首、下一首或跳转；只有跳转使用 `position_ms`。回调数据须保留到媒体源可安全释放。命令回调进行期间，更新或释放返回 `LimitExceeded`。每个插件当前最多有 4 个媒体源，媒体资源字节总量上限为 32 MiB。

`current_title` 使用所需长度输出参数。可先以零容量查询长度，再分配缓冲区读取；两次调用之间标题可能变化。

参见[媒体源示例](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/examples/media_source.rs)和[数据类型定义](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/types/v2/context.rs)。

[返回 API 目录](/plugin-dev/api)
