use windows::core::{PCWSTR, ComInterface};
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_SYSICONINDEX, SHGetImageList, SHIL_EXTRALARGE, ShellExecuteW};
use windows::Win32::UI::Controls::IImageList;
use windows::Win32::UI::WindowsAndMessaging::{HICON, SW_SHOWNORMAL, DestroyIcon};
use crate::win32::api::utils::to_wide;
use std::path::Path;

/// ファイルパスから 48x48 (SHIL_EXTRALARGE) のアイコン (HICON) を取得するよ！
/// 取得した HICON は呼び出し側で DestroyIcon する必要があることに注意してね。
pub fn get_icon_for_path(path: &Path) -> Option<HICON> {
    let path_str = path.to_string_lossy();
    let wide_path = to_wide(&path_str);
    
    let mut shfi = SHFILEINFOW::default();
    
    // 1. システムイメージリスト内のインデックスを取得する
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR::from_raw(wide_path.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        )
    };

    if result == 0 {
        return None;
    }

    // 2. 48x48 (SHIL_EXTRALARGE) のイメージリストを取得してアイコンを抽出する
    unsafe {
        // IImageList インターフェースを取得
        if let Ok(image_list) = SHGetImageList::<IImageList>(SHIL_EXTRALARGE as i32) {
            if let Ok(hicon) = image_list.GetIcon(shfi.iIcon, 0) {
                return Some(hicon);
            }
        }
    }

    None
}

/// 指定されたパスのファイルを実行 (開く) するよ！
pub fn execute_path(path: &Path) -> Result<(), windows::core::Error> {
    let wide_path = to_wide(&path.to_string_lossy());
    unsafe {
        ShellExecuteW(
            None,
            windows::core::w!("open"),
            PCWSTR::from_raw(wide_path.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
    Ok(())
}

/// 指定されたパスのファイルがある場所をエクスプローラーで表示 (選択状態に) するよ！
pub fn open_file_location(path: &Path) -> Result<(), windows::core::Error> {
    let path_str = path.to_string_lossy();
    let arg = format!(r#"/select,"{}""#, path_str);
    let wide_arg = to_wide(&arg);
    let wide_explorer = to_wide("explorer.exe");
    
    unsafe {
        ShellExecuteW(
            None,
            windows::core::w!("open"),
            PCWSTR::from_raw(wide_explorer.as_ptr()),
            PCWSTR::from_raw(wide_arg.as_ptr()),
            None,
            SW_SHOWNORMAL,
        );
    }
    Ok(())
}
