use std::ffi::c_void;

use winisland_plugin_api::{CAP_SYSTEM, LocalDateTimeV2, LunarDateV2, PluginStatus, PluginToken};

use super::{runtime, write_buffer};
use crate::runtime::{HostRuntime, SystemServices};

unsafe fn access<'a>(
    context: *mut c_void,
    token: PluginToken,
) -> Result<&'a HostRuntime, PluginStatus> {
    // SAFETY: The caller keeps its host context allocated for this ABI call.
    let host = unsafe { runtime(context)? };
    host.registry.require(token, CAP_SYSTEM)?;
    Ok(host)
}

fn services(host: &HostRuntime) -> Result<SystemServices, PluginStatus> {
    host.system_services
        .lock()
        .map_err(|_| PluginStatus::Internal)?
        .ok_or(PluginStatus::UnsupportedVersion)
}

pub unsafe extern "C" fn local_datetime(
    context: *mut c_void,
    token: PluginToken,
    out: *mut LocalDateTimeV2,
) -> PluginStatus {
    if out.is_null() {
        return PluginStatus::InvalidArgument;
    }
    let result = (|| {
        // SAFETY: The ABI caller supplies a live host context.
        let host = unsafe { access(context, token)? };
        let value = (services(host)?.local_datetime)();
        // SAFETY: The caller supplies a writable complete date-time output.
        unsafe { *out = value };
        Ok(())
    })();
    result.err().unwrap_or(PluginStatus::Ok)
}

pub unsafe extern "C" fn lunar_date(
    context: *mut c_void,
    token: PluginToken,
    year: u16,
    month: u16,
    day: u16,
    out: *mut LunarDateV2,
    found: *mut u8,
) -> PluginStatus {
    if out.is_null() || found.is_null() {
        return PluginStatus::InvalidArgument;
    }
    let result = (|| {
        // SAFETY: The ABI caller supplies a live host context.
        let host = unsafe { access(context, token)? };
        let leap =
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
        let days = match month {
            2 => {
                if leap {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => return Err(PluginStatus::InvalidArgument),
        };
        if !(1..=9999).contains(&year) || !(1..=days).contains(&day) {
            return Err(PluginStatus::InvalidArgument);
        }
        let value = (services(host)?.lunar_date)(year, month, day);
        // SAFETY: Both checked outputs are writable throughout this synchronous call.
        unsafe {
            *found = u8::from(value.is_some());
            *out = value.unwrap_or_default();
        }
        Ok(())
    })();
    result.err().unwrap_or(PluginStatus::Ok)
}

pub unsafe extern "C" fn current_language(
    context: *mut c_void,
    token: PluginToken,
    buffer: *mut u8,
    capacity: u32,
    required: *mut u32,
) -> PluginStatus {
    // SAFETY: The ABI caller supplies a live host context.
    if let Err(status) = unsafe { access(context, token) } {
        return status;
    }
    let language = winisland_core::i18n::current_lang();
    // SAFETY: The helper validates output pointers and capacity before copying the snapshot.
    unsafe { write_buffer(language.as_bytes(), buffer, capacity, required) }
}
