use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::DataExchange::{OpenClipboard, CloseClipboard, GetClipboardData};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;

/// Rust の文字列を Windows API 用の null 終端ワイド文字列 (Vec<u16>) に変換するよ！
pub fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// クリップボードからテキストを取得するよ！
pub fn get_clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(None).is_err() {
            return None;
        }

        let mut result = None;
        if let Ok(handle) = GetClipboardData(CF_UNICODETEXT.0 as u32) {
            let hglobal = HGLOBAL(handle.0 as *mut _);
            let ptr = GlobalLock(hglobal);
            if !ptr.is_null() {
                // ワイド文字列 (u16) として読み取る
                let slice = std::slice::from_raw_parts(ptr as *const u16, 1024); // 最大 1024 文字
                let len = slice.iter().take_while(|&&c| c != 0).count();
                result = Some(String::from_utf16_lossy(&slice[..len]));
                GlobalUnlock(hglobal).ok()?;
            }
        }

        let _ = CloseClipboard();
        result
    }
}
