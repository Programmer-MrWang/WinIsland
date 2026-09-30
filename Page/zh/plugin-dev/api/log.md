# Log API

`LogApiV2` 写入带插件 ID 和版本的诊断消息。查询 `IFACE_LOG` 即可；此服务不要求 `CAP_*`。SDK `host.log().write(level, message)` 是尽力执行的便捷方法。

## 方法

`write(context, token, level, message)` 返回 `PluginStatus`。`message` 是最长 64 KiB 的借用 UTF-8 切片。数字级别分别为 `0` 错误、`1` 警告、`2` 信息、`3` 调试、`4` 跟踪。其他值返回 `InvalidArgument`。令牌已停止或撤销时返回 `StaleHandle`。

SDK 便捷方法会丢弃返回状态；需要处理错误时应查询原始 `LogApiV2.write` 函数槽。日志应记录有用的诊断信息，避免写入密钥、私人数据或无限制的大块内容。

参见[原始服务表](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/abi/tables.rs)和[宿主实现](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-host/src/services/log.rs)。

[返回 API 目录](/plugin-dev/api)
