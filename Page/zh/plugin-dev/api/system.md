# System API

`SystemApiV2` 提供本地日期时间、公历转中国农历，以及当前 WinIsland 界面语言。该接口从 crate `0.10.0` 起提供，需要 `CAP_SYSTEM` 和 `IFACE_SYSTEM`。SDK 入口为 `host.system()?`；包括内置插件在内的插件均可使用，无需导入主程序或平台模块。

## 读取当前日期

此辅助函数只需要 `CAP_SYSTEM`。保留创建插件时收到的 `Host`，在显示需要刷新时重新读取快照。

```rust
use winisland_plugin_api::sdk::{Error, Host};

fn date_label(host: &Host) -> Result<String, Error> {
    let system = host.system()?;
    let now = system.local_datetime()?;
    let language = system.current_language()?;
    let lunar = system.lunar_date(now.year, now.month, now.day)?;
    let lunar_label = lunar.map_or_else(String::new, |date| {
        format!(" (lunar {}-{}, leap={})", date.month, date.day, date.leap)
    });
    Ok(format!(
        "{:04}-{:02}-{:02}{lunar_label} [{language}]",
        now.year, now.month, now.day,
    ))
}
```

日期时间遵循电脑的本地时区。语言标识来自 WinIsland 当前界面语言，可能与 Windows 的语言不同；插件选择自己的翻译时应重新读取。[I18n](/plugin-dev/api/i18n) 负责另外注册翻译资源。需要定期刷新时，可使用 [Events](/plugin-dev/api/events) 定时器，并额外声明 `CAP_EVENTS`。

## 方法

原始调用先接收 `context, token`，并返回 `PluginStatus`。

| SDK 方法 | 原始方法与后续参数 | 结果 |
|---|---|---|
| `local_datetime()` | `local_datetime(*mut LocalDateTimeV2)` | 写入一份本地日期时间快照。 |
| `lunar_date(year, month, day)` | `lunar_date(u16, u16, u16, *mut LunarDateV2, *mut u8)` | 写入农历值和是否找到结果的标记；SDK 返回 `Option<LunarDate>`。 |
| `current_language()` | `current_language(字节缓冲区, 容量, *mut required)` | 复制当前语言标识的 UTF-8 字节；SDK 返回 `String`。 |

原始日期输出必须指向完整、可写的数据结构。读取语言时，空指针配合零容量可查询所需字节数；结果非空且容量不足时返回 `LimitExceeded`。长度不包含 NUL 终止符。SDK 会分配缓冲区，并在两次调用之间语言发生变化时重试读取。

## 数据与错误

`LocalDateTimeV2` 包含 `struct_size`、`year`、`month`、`day`、`day_of_week`、`hour`、`minute`、`second` 和 `millisecond`。月份从 1 开始；星期从星期日 = 0 到星期六 = 6。这是本地时钟快照，不包含时区偏移或单调时间戳。

`LunarDateV2` 包含 `struct_size`、农历 `month`、`day`、表示是否为闰月的 0/1 `leap`，以及 `reserved`。SDK 的 `LunarDate` 使用 `u8` 表示月和日，使用 `bool` 表示闰月。输入须为年份 1–9999 范围内的有效公历日期；无效日期返回 `InvalidArgument`。有效日期若不在宿主转换能力的覆盖范围内，调用仍成功，但是否找到结果的标记为零，SDK 将其转换为 `None`。

日期服务依赖宿主接入的平台服务；未接入时返回 `UnsupportedVersion`，权限不足时返回 `CapabilityMissing`。较旧宿主可能没有此服务表：`Host::supports(IFACE_SYSTEM)` 可检查是否存在，`host.system()` 无法查询时返回错误。与其他新增能力一样，旧宿主也可能在创建插件前就拒绝未知能力位。

System 调用不创建资源或订阅。每次调用期间都应保留有效宿主，并通过 `sdk::Error` 或原始 `PluginStatus` 处理错误。参见[系统数据类型](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/types/v2/system.rs)和 [SDK](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/sdk/system.rs)。

[返回 API 目录](/plugin-dev/api)
