# 插件开发

WinIsland 插件是通过 ABI v2 加载的 Windows DLL。插件先向 WinIsland 获取一项服务，再用它添加状态文字、媒体源、小组件或设置页等内容。当前 `winisland-plugin-api` 库版本为 `0.9`；现有宿主不能加载 ABI v1 DLL。

> 插件与 WinIsland 在同一进程运行，没有沙箱。`extern "C"` 回调中的 panic 可能导致应用退出。

## 文档导航

| 指南 | 内容 |
|---|---|
| [快速开始](/plugin-dev/quickstart) | 构建、加载并打包 ABI v2 插件 |
| [ABI 与生命周期](/plugin-dev/abi-lifecycle) | 描述符校验、所有权、回调、卸载和迁移 |
| [宿主服务](/plugin-dev/services) | 十六张服务表、绘制、设置与限制 |
| [API 参考](/plugin-dev/api) | 每张公开服务表单独一页，列出方法、数据约定和限制 |
| [打包与安装](/plugin-dev/packaging) | `plugin.yml`、ZIP、签名、安装和更新 |
| [API 更新日志](/api-changelog) | 已发布库版本的历史记录 |

完整的 Rust 签名请查看 [SDK 说明文档](https://github.com/WinIslandProject/WinIsland/tree/master/crates/winisland-plugin-api)和 [ABI 定义](https://github.com/WinIslandProject/WinIsland/tree/master/crates/winisland-plugin-api/src/abi)。

## 先按想做的事找入口

| 我想…… | 从这里开始 | 最终会做什么 |
|---|---|---|
| 在岛上显示一条状态或提醒 | [Context API](/plugin-dev/api/context) | 发布文字，需要时更新或移除。 |
| 提供歌曲信息或播放控制 | [Media API](/plugin-dev/api/media) | 发布曲目；控制按钮还需要命令回调。 |
| 在展开页画自己的内容 | [Widget API](/plugin-dev/api/widget) | 给网格小组件提交一整帧绘制内容。 |
| 添加可交互页面或紧凑面板 | [Surface](/plugin-dev/api/surface)、[Input](/plugin-dev/api/input) 和 [Events](/plugin-dev/api/events) | 绘制内容、定义命中区域并保留事件订阅。 |
| 添加托盘命令或快捷键 | [Command API](/plugin-dev/api/command) | 注册回调，可附带托盘入口与快捷键。 |
| 仅在可见时更新内容 | [Events API](/plugin-dev/api/events) | 使用可见性定时器或开关动画 tick。 |
| 控制已有播放器 | [Media Session API](/plugin-dev/api/media-session) | 枚举、选择会话，发送操作并接收完成结果。 |
| 在设置中增加选项 | [Settings API](/plugin-dev/api/settings) 和 [Store API](/plugin-dev/api/store) | 描述控件，并单独保存用户选择。 |
| 跟随当前歌曲或主题变化 | [Host State API](/plugin-dev/api/host-state) | 读取当前状态或订阅变化。 |
| 修改显示出来的歌词 | [Lyrics Transform API](/plugin-dev/api/lyrics-transform) | 在显示前处理已解析的歌词行。 |

第一次写插件，建议先做完[只显示一条状态的示例](/plugin-dev/quickstart)。其余服务及调用细节可在 [API 参考](/plugin-dev/api)中查找。

## 先弄清三个词

1. **能力位：**在插件描述符里声明“我需要用哪项服务”。这里仅日志服务不需要能力位。
2. **服务表：**WinIsland 提供的一组函数。SDK 封装了常用操作；原始服务表提供完整接口。
3. **资源：**插件创建的状态、小组件、设置页等。需要它时保留 ID 或 SDK 对象；关闭插件前释放。

0.9 新增独立页面、紧凑区域、背景/前景层、输入区域、命令、定时器和媒体会话控制。交互界面通过 Surface 或 Widget 配合 Input、Events 实现。这些仍是明确的宿主扩展点，不能任意替换内部实现或拦截区域外的输入。原生 Windows 接入由宿主负责，插件使用这些功能不需要各自增加 Windows 绑定依赖。

## 运行模型

```text
DLL 导出 winisland_plugin_entry_v2() -> 静态 PluginDescriptorV2
    -> 宿主校验 ABI、能力、回调和元数据
    -> 宿主携带 token 与 PluginHostV2 调用 create(PluginCreateInfoV2)
    -> 插件查询版本化服务表并创建资源
    -> 可选的 descriptor.on_tick 在插件工作线程执行
    -> 小组件提交完整绘制列表；宿主校验并重放
    -> 宿主调用 shutdown(handle)，再调用 destroy(handle)，最后卸载 DLL
```

`PluginDescriptorV2.capabilities` 声明要使用的服务。每项资源归宿主签发的 `PluginToken` 所有。服务表通过 `PluginHostV2.query` 获取，版本为 `IFACE_VERSION_1`，与顶层 `ABI_VERSION_2` 不同。

| 能力 | 服务表 | 用途 |
|---|---|---|
| `CAP_CONTEXT` | `ContextApiV2` | 活动状态文字 |
| `CAP_MEDIA` | `MediaApiV2` | 媒体源、封面和控制 |
| `CAP_I18N` | `I18nApiV2` | 翻译资源 |
| `CAP_HOST_STATE` | `HostStateApiV2` | 媒体/主题快照与订阅 |
| `CAP_WIDGET` | `WidgetApiV2` | 小组件与绘制列表 |
| `CAP_SURFACE` | `SurfaceApiV2` | 独立展开页、紧凑内容和岛内绘制层 |
| `CAP_INPUT` | `InputApiV2` | 指针、键盘、输入法和文件拖入区域 |
| `CAP_EVENTS` | `EventsApiV2` | 事件订阅、定时器、岛状态和动画调度 |
| `CAP_COMMAND` | `CommandApiV2` | 命令、托盘入口和全局快捷键 |
| `CAP_MEDIA_SESSION` | `MediaSessionApiV2` | 发现、选择和控制现有媒体会话 |
| `CAP_LYRICS` | `LyricsTransformApiV2` | 已解析歌词转换 |
| `CAP_SETTINGS` | `SettingsApiV2` | 声明式设置页 |
| `CAP_TEXT` | `TextApiV2` | 文字测量和字体族 |
| `CAP_IMAGE` | `ImageApiV2` | 图片及当前专辑封面 |
| `CAP_STORE` | `StoreApiV2` | 插件独立命名空间的持久化数据 |

`LogApiV2` 无需能力位。只声明插件实际需要的能力。

## 开发流程

1. 创建 Rust `cdylib`，按照[快速开始](/plugin-dev/quickstart)添加 `winisland-plugin-api` 依赖。
2. 导出返回静态 `PluginDescriptorV2` 的 `winisland_plugin_entry_v2`。
3. 在 `create` 中校验 `PluginCreateInfoV2`，使用 SDK `Host::from_raw` 或原始服务表。
4. 保留资源句柄直到 `shutdown`；在宿主表有效时释放。
5. `shutdown` 返回成功前停止插件线程，并等待其全部结束；`destroy` 释放不透明实例。
6. 使用根目录 `plugin.yml`、`abi-version: 2` 和入口 DLL 制作 ZIP；拖到岛上安装或更新。

SDK 封装常用调用并构建绘制列表。复杂控件和设置变更需要原始 ABI。插件绘制代码不会在渲染线程运行。

## 兼容性

库版本 `0.9`、顶层 `ABI_VERSION_2` 与服务表 `IFACE_VERSION_1` 是不同的版本号。新服务表以追加方式扩展，原有 ABI v2 布局不变。但新能力位仍要求新版宿主，旧 ABI v2 宿主可能在 `create` 前就拒绝描述符。`Host::supports(IFACE_*)` 只查询服务表是否存在，不代表已有调用权限，也无法绕过加载阶段的能力检查。读取表字段前检查 `struct_size` 与 `version`。ABI v1 DLL 必须迁移源码并重新打包；只改 `plugin.yml` 无法将其转换为 v2。
