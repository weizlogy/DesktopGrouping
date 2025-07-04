use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, GetDC, GetDIBits, GetObjectW, ReleaseDC,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Controls::IImageList;
use windows::Win32::UI::Shell::{
    CSIDL_FONTS, SHFILEINFOW, SHGFI_SYSICONINDEX, SHGetFileInfoW, SHGetFolderPathW, SHGetImageList,
    SHIL_EXTRALARGE,
}; // SHIL_EXTRALARGE はこっちにいたよ！
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};
use windows::core::{Error, PCWSTR, Result};

// 使用例
// let font_path = get_system_font_path("segoeui.ttf");
pub fn get_system_font_path(font_name: &str) -> PathBuf {
    let mut path = [0u16; 260];
    unsafe {
        SHGetFolderPathW(HWND(0), CSIDL_FONTS as i32, None, 0, &mut path)
            .expect("Failed to get font path.");
    }
    let font_dir = String::from_utf16_lossy(&path);
    PathBuf::from(font_dir.trim_end_matches('\0')).join(font_name)
}

/// ファイルまたはディレクトリのアイコンを取得し、BITMAPINFO とピクセルデータを返します。
/// 失敗した場合は WinError を返します。
pub fn get_file_icon(path: &Path) -> Result<(BITMAPINFO, Vec<u8>)> {
    let mut sh_file_info = SHFILEINFOW::default();
    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();

    // scopeguard を使うためにインポート
    use scopeguard::defer;

    unsafe {
        // SHGetFileInfoW でシステムイメージリストのインデックスを取得
        let result = SHGetFileInfoW(
            PCWSTR(path_wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut sh_file_info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX, // アイコンハンドルではなくインデックスを取得
        );

        if result == 0 {
            return Err(Error::from_win32());
        }
        let icon_index = sh_file_info.iIcon; // アイコンインデックスを取得

        // SHGetImageList で 48x48 (SHIL_EXTRALARGE) のイメージリストを取得
        let himagelist: IImageList = SHGetImageList::<IImageList>(SHIL_EXTRALARGE as i32) // u32 から i32 にキャスト！
            .expect("Failed to get image list"); // SHIL_EXTRALARGE (48x48)

        // ImageList_GetIcon で HICON を取得
        // ImageList_GetIcon は HICON を直接返す。失敗時は is_invalid() でチェック
        let hicon = himagelist.GetIcon(icon_index, 0)?; // ILD_NORMAL (0) を指定
        // hicon = ImageList_GetIcon(himagelist, icon_index, 0); // 古い形式 (HIMAGELIST を直接使う場合)

        if hicon.is_invalid() {
            // GetLastError は ImageList_GetIcon では通常設定されない
            return Err(Error::new(
                windows::core::HRESULT(0x80004005u32 as i32),
                "ImageList_GetIcon failed".into(),
            ));
        }
        // 取得した HICON は最後に必ず破棄する
        defer! {{
          let _ = DestroyIcon(hicon);
        }}

        // --- ここから下は HICON から BITMAPINFO とピクセルデータを取得する処理 (ほぼ変更なし) ---

        // GetIconInfo 呼び出し
        let mut icon_info = ICONINFO::default();
        GetIconInfo(hicon, &mut icon_info)
            .ok()
            .expect("Failed to get icon info");

        // DeleteObject の defer は不要 (コメントアウト済み)

        // カラービットマップハンドル (hbmColor) を使用する
        let hbm = icon_info.hbmColor;
        if hbm.is_invalid() {
            return Err(Error::new(
                windows::core::HRESULT(0x80004005u32 as i32),
                "Invalid hbmColor".into(),
            ));
        }

        // GetObjectW 呼び出し
        let mut bmp: windows::Win32::Graphics::Gdi::BITMAP = std::mem::zeroed();
        let obj_size = std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAP>() as i32;
        if GetObjectW(
            hbm,
            obj_size,
            Some(&mut bmp as *mut _ as *mut std::ffi::c_void),
        ) == 0
        {
            return Err(Error::from_win32());
        }

        // BITMAPINFOHEADER の作成
        let mut bmih = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: bmp.bmWidth,
            biHeight: bmp.bmHeight,
            biPlanes: 1,
            biBitCount: bmp.bmBitsPixel as u16,
            biCompression: BI_RGB.0 as u32,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        // スクリーン DC の取得
        let hdc_screen = GetDC(HWND::default());
        if hdc_screen.is_invalid() {
            return Err(Error::new(
                windows::core::HRESULT(0x80004005u32 as i32),
                "Failed to get screen DC".into(),
            ));
        }
        defer! {{
            ReleaseDC(HWND::default(), hdc_screen);
        }}

        // GetDIBits (1回目)
        let mut bmi_for_size = BITMAPINFO {
            bmiHeader: bmih,
            ..Default::default()
        };
        if GetDIBits(
            hdc_screen,
            hbm,
            0,
            bmp.bmHeight as u32,
            None,
            &mut bmi_for_size,
            DIB_RGB_COLORS,
        ) == 0
        {
            return Err(Error::from_win32());
        }
        bmih = bmi_for_size.bmiHeader;

        // biSizeImage の計算
        if bmih.biSizeImage == 0 {
            let stride =
                ((bmih.biWidth.abs() as usize * (bmih.biBitCount as usize / 8) + 3) & !3) as u32;
            bmih.biSizeImage = stride * bmih.biHeight.abs() as u32;
        }
        if bmih.biSizeImage == 0 {
            return Err(Error::new(
                windows::core::HRESULT(0x80004005u32 as i32),
                "biSizeImage is zero".into(),
            ));
        }

        // ピクセルデータバッファの確保
        let mut pixel_data: Vec<u8> = vec![0; bmih.biSizeImage as usize];

        // GetDIBits (2回目)
        let mut bmi = BITMAPINFO {
            bmiHeader: bmih,
            ..Default::default()
        };
        let lines = GetDIBits(
            hdc_screen,
            hbm,
            0,
            bmih.biHeight.abs() as u32,
            Some(pixel_data.as_mut_ptr() as *mut std::ffi::c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        if lines == 0 {
            return Err(Error::from_win32());
        }

        Ok((bmi, pixel_data))
    }
}

/// アイコンを解放します。
pub fn release_icon(icon: HICON) {
    unsafe {
        let _ = DestroyIcon(icon);
    }
}
