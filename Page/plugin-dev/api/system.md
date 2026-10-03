# System API

`SystemApiV2` reads local date/time, converts Gregorian dates to Chinese lunar dates, and reads the current WinIsland UI language. Added in crate `0.10.0`, it requires `CAP_SYSTEM` and `IFACE_SYSTEM`. The SDK entry point is `host.system()?`; plugins, including built-in plugins, can use it without importing application or platform modules.

## Read the current date

This helper uses only `CAP_SYSTEM`. Keep the `Host` supplied during plugin creation and call the service again when the display needs a fresh snapshot.

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

Date/time follows the computer's local time zone. The language is WinIsland's active UI language, which can differ from the Windows language. Refresh it when choosing the plugin's own translated labels; [I18n](/plugin-dev/api/i18n) registers translation bundles separately. To refresh periodically, use an [Events](/plugin-dev/api/events) timer and declare `CAP_EVENTS` too.

## Methods

Raw calls take `context, token` first and return `PluginStatus`.

| SDK method | Raw method and remaining parameters | Result |
|---|---|---|
| `local_datetime()` | `local_datetime(*mut LocalDateTimeV2)` | Writes a copied local date/time snapshot. |
| `lunar_date(year, month, day)` | `lunar_date(u16, u16, u16, *mut LunarDateV2, *mut u8)` | Writes a lunar value and a found flag; the SDK returns `Option<LunarDate>`. |
| `current_language()` | `current_language(byte buffer, capacity, *mut required)` | Copies the active language identifier as UTF-8; the SDK returns a `String`. |

The raw date outputs must be complete writable structures. For language output, a null buffer and zero capacity reports the required byte length; insufficient capacity for a nonempty value returns `LimitExceeded`. The length excludes a NUL terminator. The SDK allocates and retries the read if the language changes between calls.

## Values and errors

`LocalDateTimeV2` contains `struct_size`, `year`, `month`, `day`, `day_of_week`, `hour`, `minute`, `second`, and `millisecond`. Months start at 1; `day_of_week` runs from Sunday = 0 to Saturday = 6. This is a wall-clock snapshot with no time-zone offset or monotonic timestamp.

`LunarDateV2` contains `struct_size`, lunar `month`, `day`, a 0/1 `leap` flag for a leap month, and `reserved`. The SDK `LunarDate` exposes month and day as `u8` and leap as `bool`. Gregorian inputs must form a valid date with year 1–9999. Invalid dates return `InvalidArgument`; a valid date outside the host's conversion coverage succeeds with a zero found flag, represented as `None` by the SDK.

Date services depend on the host's platform service integration; an unavailable integration returns `UnsupportedVersion`. Missing permission returns `CapabilityMissing`. Older hosts may lack the table: `Host::supports(IFACE_SYSTEM)` checks availability, while `host.system()` returns an error when it cannot query it. As with other additive capabilities, an older host can reject an unknown capability before plugin creation.

System calls create no resources or subscriptions. Keep the host alive for every call and handle failures through `sdk::Error` or raw `PluginStatus`. See the [system types](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/types/v2/system.rs) and [SDK](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/sdk/system.rs).

[All plugin APIs](/plugin-dev/api)
