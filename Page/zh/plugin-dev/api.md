# 插件 API 参考

ABI v2 提供十一张宿主服务表。下面每页介绍一个 API 的能力声明、方法、数据约定、生命周期和当前限制。尚未创建插件描述符时，先阅读 [ABI 与生命周期](/plugin-dev/abi-lifecycle)。

每个服务调用都需要传入表中的 `prefix.context` 和宿主签发的 `PluginToken`，并返回 `PluginStatus`。创建的资源归该令牌所有。原始调用通过 `PluginHostV2.query(context, IFACE_*, IFACE_VERSION_1)` 获取服务表，检查表的大小、版本和所需函数槽。SDK 的 `Host` 封装会为便捷方法执行这些检查。

| API | 提供的能力 |
|---|---|
| [Context API](/plugin-dev/api/context) | 按优先级显示活动状态文字 |
| [Media API](/plugin-dev/api/media) | 媒体源、封面、进度和控制 |
| [I18n API](/plugin-dev/api/i18n) | 插件翻译资源 |
| [Host State API](/plugin-dev/api/host-state) | 媒体与主题状态及变化通知 |
| [Widget API](/plugin-dev/api/widget) | 网格小组件和经校验的绘制列表 |
| [Lyrics Transform API](/plugin-dev/api/lyrics-transform) | 已解析歌词的逐行转换 |
| [Settings API](/plugin-dev/api/settings) | 声明式插件设置页 |
| [Text API](/plugin-dev/api/text) | 文字测量和宿主字体族 |
| [Image API](/plugin-dev/api/image) | 解码、上传和封面图片句柄 |
| [Store API](/plugin-dev/api/store) | 插件独立命名空间中的持久化字节数据 |
| [Log API](/plugin-dev/api/log) | 带插件标识的诊断日志 |

## 通用 ABI 规则

除 Log 外，使用服务前必须在 `PluginDescriptorV2.capabilities` 声明对应 `CAP_*`。查到服务表本身并不授予使用权。宿主会拒绝过期、属于其他插件或类型不符的句柄。借用的数据必须保持有效，直到同步调用返回；回调数据必须保持有效，直到回调不可能再运行。

状态值包括 `Ok`、`InvalidArgument`、`StaleHandle`、`CapabilityMissing`、`LimitExceeded`、`UnsupportedVersion`、`IoError` 和 `Internal`。二进制输出方法通常可用空缓冲区和零容量查询所需字节数；非空结果在容量不足时返回 `LimitExceeded`。具体回调规则见各接口页。

本文档对应当前 ABI v2 实现。[宿主服务概览](/plugin-dev/services)提供简短介绍；精确的 Rust 布局以 [SDK 源码](https://github.com/WinIslandProject/WinIsland/tree/master/crates/winisland-plugin-api/src)为准。
