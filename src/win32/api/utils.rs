use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::DataExchange::{OpenClipboard, CloseClipboard, GetClipboardData};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

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
                // 実際には null 終端まで読み取る必要があるよ
                let mut len = 0;
                while * ( (ptr as *const u16).add(len) ) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(ptr as *const u16, len);
                result = Some(String::from_utf16_lossy(slice));
                let _ = GlobalUnlock(hglobal);
            }
        }

        let _ = CloseClipboard();
        result
    }
}

/// HDROP ハンドルからファイルパスのリストを取得するよ！
pub fn get_dropped_files(hdrop: HDROP) -> Vec<PathBuf> {
    unsafe {
        let count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
        let mut files = Vec::new();

        for i in 0..count {
            let len = DragQueryFileW(hdrop, i, None);
            let mut buffer = vec![0u16; (len + 1) as usize];
            DragQueryFileW(hdrop, i, Some(&mut buffer));
            
            let path_str = String::from_utf16_lossy(&buffer[..len as usize]);
            files.push(PathBuf::from(path_str));
        }

        files
    }
}
