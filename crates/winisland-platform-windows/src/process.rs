use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, HANDLE};
use windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::{Owned, PWSTR};

pub(crate) fn open(process_id: u32) -> Option<Owned<HANDLE>> {
    // SAFETY: The requested access only queries identity, and the new handle is owned by the guard.
    unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id)
            .ok()
            .map(|process| Owned::new(process))
    }
}

pub(crate) fn app_user_model_id(process: HANDLE) -> Option<String> {
    let mut length = 0;
    // SAFETY: The first call only queries the required buffer length.
    if unsafe { GetApplicationUserModelId(process, &mut length, None) } != ERROR_INSUFFICIENT_BUFFER
        || length == 0
    {
        return None;
    }
    let mut buffer = vec![0u16; length as usize];
    // SAFETY: The buffer has the length reported by the first call.
    if unsafe { GetApplicationUserModelId(process, &mut length, Some(PWSTR(buffer.as_mut_ptr()))) }
        != ERROR_SUCCESS
    {
        return None;
    }
    buffer.truncate(length.saturating_sub(1) as usize);
    String::from_utf16(&buffer).ok()
}

pub(crate) fn executable_name(process: HANDLE) -> Option<String> {
    let path = executable_path(process)?;
    std::path::Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn executable_path(process: HANDLE) -> Option<String> {
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    // SAFETY: The process handle is queryable and the writable buffer holds length UTF-16 units.
    unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }
    .ok()?;
    Some(String::from_utf16_lossy(&buffer[..length as usize]))
}

pub(crate) fn running_app_identities() -> std::collections::HashSet<String> {
    let mut paths = std::collections::HashSet::new();
    // SAFETY: The snapshot is owned until enumeration ends; the entry has its required size and valid storage.
    unsafe {
        let Ok(handle) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return paths;
        };
        if handle.is_invalid() {
            return paths;
        }
        let snapshot = Owned::new(handle);
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(*snapshot, &mut entry).is_err() {
            return paths;
        }
        loop {
            if let Some(process) = open(entry.th32ProcessID) {
                if let Some(path) = executable_path(*process) {
                    paths.insert(path.to_lowercase());
                }
                if let Some(app_id) = app_user_model_id(*process) {
                    paths.insert(app_id.split('!').next().unwrap_or(&app_id).to_lowercase());
                    paths.insert(app_id.to_lowercase());
                }
            }
            if Process32NextW(*snapshot, &mut entry).is_err() {
                break;
            }
        }
    }
    paths
}
