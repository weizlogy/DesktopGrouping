use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

/// Rust の文字列を Windows API 用の null 終端ワイド文字列 (Vec<u16>) に変換するよ！
pub fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
