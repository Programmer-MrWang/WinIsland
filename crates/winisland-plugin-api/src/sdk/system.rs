use crate::{IFACE_SYSTEM, LocalDateTimeV2, LunarDateV2, PluginStatus, SystemApiV2};

use super::{Error, Host, status_error, success};

#[derive(Clone)]
pub struct SystemApi(Host);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LunarDate {
    pub month: u8,
    pub day: u8,
    pub leap: bool,
}

impl Host {
    pub fn system(&self) -> Result<SystemApi, Error> {
        self.query::<SystemApiV2>(IFACE_SYSTEM)?;
        Ok(SystemApi(self.clone()))
    }
}

impl SystemApi {
    pub fn local_datetime(&self) -> Result<LocalDateTimeV2, Error> {
        let table = self.0.query::<SystemApiV2>(IFACE_SYSTEM)?;
        let function = table
            .local_datetime
            .ok_or(Error::MissingFunction("system.local_datetime"))?;
        let mut value = LocalDateTimeV2::default();
        // SAFETY: The output is a complete writable date-time value during this synchronous call.
        success(unsafe { function(table.prefix.context, self.0.token, &mut value) })?;
        Ok(value)
    }

    pub fn lunar_date(&self, year: u16, month: u16, day: u16) -> Result<Option<LunarDate>, Error> {
        let table = self.0.query::<SystemApiV2>(IFACE_SYSTEM)?;
        let function = table
            .lunar_date
            .ok_or(Error::MissingFunction("system.lunar_date"))?;
        let mut value = LunarDateV2::default();
        let mut found = 0;
        // SAFETY: Both outputs remain writable until the synchronous host call returns.
        success(unsafe {
            function(
                table.prefix.context,
                self.0.token,
                year,
                month,
                day,
                &mut value,
                &mut found,
            )
        })?;
        Ok((found != 0).then_some(LunarDate {
            month: value.month,
            day: value.day,
            leap: value.leap != 0,
        }))
    }

    pub fn current_language(&self) -> Result<String, Error> {
        let table = self.0.query::<SystemApiV2>(IFACE_SYSTEM)?;
        let function = table
            .current_language
            .ok_or(Error::MissingFunction("system.current_language"))?;
        let mut required = 0;
        // SAFETY: A zero-capacity call writes only the valid required-length output.
        let status = unsafe {
            function(
                table.prefix.context,
                self.0.token,
                std::ptr::null_mut(),
                0,
                &mut required,
            )
        };
        if status != PluginStatus::Ok && status != PluginStatus::LimitExceeded {
            return Err(status_error(status));
        }
        for _ in 0..3 {
            let capacity = required;
            let mut bytes = vec![0; capacity as usize];
            // SAFETY: The output buffer has the advertised capacity and the count is writable.
            let status = unsafe {
                function(
                    table.prefix.context,
                    self.0.token,
                    bytes.as_mut_ptr(),
                    capacity,
                    &mut required,
                )
            };
            if status == PluginStatus::LimitExceeded && required > capacity {
                continue;
            }
            success(status)?;
            if required > capacity {
                return Err(Error::InvalidHost);
            }
            bytes.truncate(required as usize);
            return String::from_utf8(bytes).map_err(|_| Error::InvalidText);
        }
        Err(Error::LimitExceeded)
    }
}
