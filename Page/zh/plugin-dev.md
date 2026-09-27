# 插件开发

WinIsland 使用 ABI v2 加载受信任的 Windows 原生 DLL。当前 `winisland-plugin-api` crate 版本为 `0.8`。宿主会拒绝 ABI v1 安装包与入口。插件可以提供 Context、媒体源、小组件、翻译、歌词转换、设置页和持久化数据。

> 插件与 WinIsland 在同一进程运行，没有沙箱。`extern "C"` 回调中的 panic 可能导致应用退出。

## 文档导航

| 指南 | 内容 |
|---|---|
| [快速开始](/plugin-dev/quickstart) | 构建、加载并打包 ABI v2 插件 |
| [ABI 与生命周期](/plugin-dev/abi-lifecycle) | Descriptor 校验、所有权、回调、卸载和迁移 |
| [宿主服务](/plugin-dev/services) | 十一张服务表、绘制、设置与限制 |
| [打包与安装](/plugin-dev/packaging) | `plugin.yml`、ZIP、签名、安装和更新 |
| [API 更新日志](/api-changelog) | 已发布 crate 的历史记录 |

精确 Rust 签名请查看 [SDK README](https://github.com/WinIslandProject/WinIsland/tree/master/crates/winisland-plugin-api) 和 [ABI 定义](https://github.com/WinIslandProject/WinIsland/tree/master/crates/winisland-plugin-api/src/abi)。

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
| `CAP_LYRICS` | `LyricsTransformApiV2` | 已解析歌词转换 |
| `CAP_SETTINGS` | `SettingsApiV2` | 声明式设置页 |
| `CAP_TEXT` | `TextApiV2` | 文字测量和字族 |
| `CAP_IMAGE` | `ImageApiV2` | 图片及当前专辑封面 |
| `CAP_STORE` | `StoreApiV2` | 插件独立命名空间的持久化数据 |

`LogApiV2` 无需能力位。只声明插件实际需要的能力。

## 开发流程

1. 创建 Rust `cdylib`，依赖 `winisland-plugin-api = "0.8"`。
2. 导出返回静态 `PluginDescriptorV2` 的 `winisland_plugin_entry_v2`。
3. 在 `create` 中校验 `PluginCreateInfoV2`，使用 SDK `Host::from_raw` 或原始服务表。
4. 保留资源句柄直到 `shutdown`；在宿主表有效时释放。
5. `shutdown` 返回成功前停止并 join 插件线程；`destroy` 释放不透明实例。
6. 使用根目录 `plugin.yml`、`abi-version: 2` 和入口 DLL 制作 ZIP；拖到岛上安装或更新。

SDK 封装常用调用并构建绘制列表。复杂控件和设置变更需要原始 ABI。插件绘制代码不会在渲染线程运行。

## 兼容性

crate `0.8`、顶层 `ABI_VERSION_2` 与服务表 `IFACE_VERSION_1` 是不同的版本号。读取表字段前检查 `struct_size` 与 `version`。ABI v1 DLL 必须迁移源码并重新打包；只改 `plugin.yml` 无法将其转换为 v2。
