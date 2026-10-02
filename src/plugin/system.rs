use winisland_plugin_api::{LocalDateTimeV2, LunarDateV2};
use winisland_plugin_host::runtime::SystemServices;

pub(crate) fn services() -> SystemServices {
    SystemServices {
        local_datetime,
        lunar_date,
    }
}

fn local_datetime() -> LocalDateTimeV2 {
    let value = crate::platform::shell().local_datetime();
    LocalDateTimeV2 {
        year: value.year,
        month: value.month,
        day: value.day,
        day_of_week: value.day_of_week,
        hour: value.hour,
        minute: value.minute,
        second: value.second,
        millisecond: value.millisecond,
        ..Default::default()
    }
}

fn lunar_date(year: u16, month: u16, day: u16) -> Option<LunarDateV2> {
    crate::platform::shell()
        .lunar_date(year, month, day)
        .map(|value| LunarDateV2 {
            month: value.month,
            day: value.day,
            leap: u8::from(value.leap),
            ..Default::default()
        })
}
