use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, GetClipboardSequenceNumber, IsClipboardFormatAvailable,
    OpenClipboard,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;

pub(super) fn sequence() -> u32 {
    // SAFETY: GetClipboardSequenceNumber takes no arguments and only reads a counter.
    unsafe { GetClipboardSequenceNumber() }
}

pub(super) fn text(max_chars: usize) -> Option<String> {
    let format = u32::from(CF_UNICODETEXT.0);
    // SAFETY: Querying format availability does not require the clipboard to be open.
    unsafe { IsClipboardFormatAvailable(format) }.ok()?;
    // SAFETY: Opening without an owner window is allowed; it is closed below on every path.
    unsafe { OpenClipboard(None) }.ok()?;
    let text = read_unicode_text(format, max_chars);
    // SAFETY: The clipboard was opened above by this thread.
    let _ = unsafe { CloseClipboard() };
    text
}

fn read_unicode_text(format: u32, max_chars: usize) -> Option<String> {
    // SAFETY: The clipboard is open on this thread and the handle stays owned by the clipboard.
    let handle = unsafe { GetClipboardData(format) }.ok()?;
    let global = HGLOBAL(handle.0);
    // SAFETY: CF_UNICODETEXT data is a movable global memory block owned by the clipboard.
    let pointer = unsafe { GlobalLock(global) }.cast::<u16>();
    if pointer.is_null() {
        return None;
    }
    // SAFETY: The block is locked; its size bounds the readable UTF-16 units.
    let available = unsafe { GlobalSize(global) } / size_of::<u16>();
    let limit = available.min(max_chars);
    // SAFETY: `pointer` is valid for `limit` units while the block remains locked.
    let units = unsafe { std::slice::from_raw_parts(pointer, limit) };
    let length = units.iter().position(|unit| *unit == 0).unwrap_or(limit);
    let text = String::from_utf16_lossy(&units[..length]);
    // SAFETY: Balances the GlobalLock above.
    let _ = unsafe { GlobalUnlock(global) };
    Some(text)
}
