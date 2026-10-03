# 宿主服务

ABI v2 通过 `PluginHostV2.query` 提供十八张带版本号的服务表。本页为概览；[API 参考](/plugin-dev/api)逐页列出各服务表的方法和数据约定。SDK 的 `Host` 封装常见操作；`winisland_plugin_api::abi` 中的原始服务表提供全部函数。每张表都以 `TablePrefix` 开头，当前表版本为 `IFACE_VERSION_1`。调用前检查所需函数槽。除日志服务外，需在 `PluginDescriptorV2` 声明对应 `CAP_*` 能力。

可以把服务理解为 WinIsland 的一个功能入口：能力位表示“我要用它”，查询服务表拿到函数，调用 `create` 或 `register` 后拿到自己负责释放的资源。内容还需要显示时，就保留 SDK 对象或原始 ID。

| 能力 | 原始服务表 | SDK 入口 | 主要操作 |
|---|---|---|---|
| `CAP_CONTEXT` | `ContextApiV2` | `host.context()?` | 创建、更新、释放活动文字 |
| `CAP_ACTIVITY` | `ActivityApiV2` | `host.activities()?` | 自绘活动、优先级、展开目标和保持显示 |
| `CAP_SYSTEM` | `SystemApiV2` | `host.system()?` | 本地日期时间、农历与界面语言 |
| `CAP_MEDIA` | `MediaApiV2` | `host.media()?` | 发布媒体源、读取当前标题 |
| `CAP_I18N` | `I18nApiV2` | `host.i18n()?`（仅查询） | 注册/释放翻译资源 |
| `CAP_HOST_STATE` | `HostStateApiV2` | `host.host_state()?` | 获取/订阅媒体与主题状态 |
| `CAP_WIDGET` | `WidgetApiV2` | `host.widgets()?` | 创建、更新、释放、绘制和测量小组件 |
| `CAP_SURFACE` | `SurfaceApiV2` | `host.surfaces()?` | 独立展开页、紧凑内容和岛内绘制层 |
| `CAP_INPUT` | `InputApiV2` | `host.input()?` | 指针、键盘、输入法和文件拖入区域 |
| `CAP_EVENTS` | `EventsApiV2` | `host.events()?` | 事件订阅、定时器、岛状态和动画调度 |
| `CAP_COMMAND` | `CommandApiV2` | `host.commands()?` | 命令、托盘入口和全局快捷键 |
| `CAP_MEDIA_SESSION` | `MediaSessionApiV2` | `host.media_sessions()?` | 发现、选择和控制现有媒体会话 |
| `CAP_LYRICS` | `LyricsTransformApiV2` | `host.lyrics()?` | 注册/释放歌词转换器 |
| `CAP_SETTINGS` | `SettingsApiV2` | `host.settings()?` | 创建、更新、释放设置页 |
| `CAP_TEXT` | `TextApiV2` | `host.text()?` | 测量文字、获取字体族 |
| `CAP_IMAGE` | `ImageApiV2` | `host.images()?` | 解码、上传、获取封面、释放图片 |
| `CAP_STORE` | `StoreApiV2` | `host.store()?` | 读写、删除插件命名空间中的字节数据 |
| 无 | `LogApiV2` | `host.log()` | 写入插件日志 |

## 活动与系统数据

[Activity](/plugin-dev/api/activity) 绑定 `SURFACE_COMPACT_MAIN` 与可选的 `SURFACE_PAGE`。宿主按优先级和更新时间，将其与原生音乐、计时器及旧文字状态统一比较。需要阻止无活动自动隐藏的持续任务可声明 `ACTIVITY_KEEP_VISIBLE`；活动须仍可参与展示。到期只停止显示，结束后仍应释放句柄。简单文字状态继续使用 Context。

[System](/plugin-dev/api/system) 提供本地日期时间、可选的中国农历转换，以及当前 WinIsland 界面语言，不创建资源句柄。内置插件可通过公开服务读取这些数据，无需导入宿主的日期或语言实现。

## 活动状态文字与媒体

`ContextDataV2` 包含优先级、紧凑模式标记、超时、标题、正文和紧凑文字。超时为零时一直保留到释放。刷新时应更新现有资源，不要反复新建。SDK `host.context()?.create(title, body)` 返回持有资源的 `Resource`。

“导出完成”这样的提醒或“计时中”这样的活动状态用 Context；插件自己提供当前曲目时用 Media。`MediaSourceDataV2` 包含标题、艺术家、专辑、毫秒单位的时长/进度、播放标记、可选 PNG/JPEG 封面字节、可用控制和命令回调。SDK `create_source(title, artist)` 只发布基础信息，不带控制按钮；封面、时间轴或控制要使用原始 `MediaApiV2`。释放或禁用插件媒体源后，WinIsland 会回退到其他来源或 SMTC。回调数据要保留到可以安全释放为止。

## 小组件绘制

`host.widgets()?.create(WidgetSpec::new("key").span(2, 1))` 创建可由布局管理的小组件。稳定的键用于重启后的布局定位。`Widget::logical_size()` 返回当前逻辑尺寸；返回 `(0, 0)` 时跳过该帧。岛收起时逻辑尺寸仍取自展开网格。

使用 `DrawListBuilder::new(Size::new(width, height))` 创建完整列表，加入 `fill_round_rect`、`text`、`text_runs` 或 `image` 等命令，再调用 `widget.submit(list.finish())`。绘制协议还支持裁剪、变换、透明度、形状、渐变、描边、阴影、图片和带字体族字段的 UTF-8 文字。宿主复制列表，校验后重放。提交成功不保证最终显示；反复提交格式错误的帧可能禁用该小组件。渲染线程不会进入插件回调。

绘制列表最多 4 MiB、4096 条命令；单条命令的文字最多 64 KiB。`Widget::request_redraw` 只尽力请求重绘，不保证成功。`PluginDescriptorV2.on_tick` 在插件工作线程收到 `WidgetId` 和经过的秒数。

## 可交互内容与调度

[Surface](/plugin-dev/api/surface) 添加独立展开页、紧凑岛左右内容和背景/前景层，复用经校验的绘制协议，并与小组件共享资源配额。[Input](/plugin-dev/api/input) 为 surface 或网格小组件分配逻辑命中区域；[Events](/plugin-dev/api/events) 在插件工作线程交付指针、键盘、输入法和文件拖入回调。

Events 还提供可见性/尺寸通知、岛状态、定时器和按目标开关动画。静态内容应关闭连续动画，只在需要时提交。把订阅和定时器句柄保存在实例中，释放目标前先取消；避免让实例自己的回调再强引用实例而形成循环。

[Command](/plugin-dev/api/command) 注册命名操作，可附带托盘入口和全局快捷键。[Media Session](/plugin-dev/api/media-session) 枚举原生及插件媒体，选择来源并发送播放请求。发送请求前先订阅 `EVENT_RESULT`；同步成功表示已入队，之后的事件才报告操作状态。

## 歌词、翻译与宿主状态

歌词转换器在歌词解析后运行，先查询输出长度，再写入。逐词同步行应保留相同的 Unicode 字符数，才能保持时间边界。SDK `host.lyrics()?.register` 接受 `Send + Sync` 转换闭包，并在注册期间保存回调数据。

`I18nApiV2` 注册带语言标记的键值资源。`HostStateApiV2.get` 返回媒体标题、艺术家、播放状态和明暗主题。`subscribe` 注册状态回调；卸载前要释放订阅。

## 设置、文字、图片、存储和日志

`SettingsApiV2` 接收声明式 `SettingsPageDataV2`，包含稳定的页面键、标题、可选图标以及章节、分组、标签、开关、选择框、步进器和按钮等设置项。`on_change` 可以接受或拒绝用户操作。SDK `create_page` 使用页面标题创建章节，`create_label_page` 则单独指定章节文字；交互控件与回调需使用原始服务表。需要跨重启保存时，显式写入存储服务，并在创建页面时恢复值。

`TextApiV2.measure` 使用带字号、字重、斜体标记和 UTF-8 字体族的 `TextStyleV2`；`font_family` 返回宿主字体族。SDK `measure` 便捷方法使用 400 字重和正体；`measure_style` 按 SDK 的 `TextStyle` 测量。完整绘制方法见[绘制 SDK](/plugin-dev/api/drawing)。`ImageApiV2` 可解码 PNG/JPEG/WebP、上传 RGBA 或获取当前封面；图片 ID 持有至释放。获取的封面句柄在曲目变化后仍保留旧图。

`StoreApiV2` 在插件独立命名空间持久化字节；单个值最多 1 MiB。`LogApiV2.write` 使用数字级别，没有能力门槛。SDK `LogApi::write` 和 `Widget::request_redraw` 会忽略错误；需要状态时使用原始表。

比如要做“显示秒数”开关：在 `create` 时从 Store 读出旧值，填入 Settings 页面；`on_change` 接受新值后再写入 Store。只重新创建设置页不会自动保存选择。要做歌曲小组件，则要把 Host State 或 Media 与 Text/Image、Widget 组合使用；Widget 自身不会提供歌曲内容。

## 资源限制与错误

当前每个插件的限制为：64 个 Activity、64 条 Context 状态文字、4 个媒体源（合计 32 MiB）、16 个翻译资源（4 MiB）、小组件与 surface 合计 8 个（4 MiB）、4 个歌词转换器、1 个设置页（2 MiB）、64 张图片（64 MiB）、16 个宿主状态订阅、Events 订阅与定时器合计 128 个、64 个命令。宿主通过 `PluginStatus` 拒绝过期、属于其他插件的句柄和超额资源。`shutdown` 返回成功前应主动释放资源；宿主之后会撤销剩余资源。
