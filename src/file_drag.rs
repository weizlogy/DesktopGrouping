use std::path::PathBuf;

use desktop_grouping::win32::ui_shell;
use windows::Win32::Graphics::Gdi::BITMAPINFO;

use crate::logger::{log_error, log_info};

/// ドラッグされたファイルの情報を保持する構造体
#[derive(Debug)]
pub struct IconInfo {
  pub path: PathBuf,
  pub name: String,
  pub icon: (BITMAPINFO, Vec<u8>),
}

impl IconInfo {
  pub fn new(path: PathBuf) -> Self {
    // file_stem() を使って拡張子を除去
    let name = match path.file_stem() { // <-- file_name() から file_stem() に変更
      Some(stem) => stem.to_string_lossy().to_string(),
      None => { // file_stem が取れない場合 (ルートディレクトリなど？)、ファイル名をそのまま使う
        path.file_name()
          .map(|name| name.to_string_lossy().to_string())
          .unwrap_or_else(|| {
            let fallback_name = path.to_string_lossy().to_string(); // パス全体を代替名に
            fallback_name
          })
      }
    };

    log_info(&format!("Path: {:?} Name: {:?}", path, name));

    // アイコンの取得
    let icon = ui_shell::get_file_icon(&path).expect("Failed to get file icon");

    log_info(&format!("Icon: {:?}", icon.0));

    IconInfo { path, name, icon }
  }

  /// この IconInfo が示すパスを実行（開く）します。
  pub fn execute(&self) {
    log_info(&format!("Executing path: {:?}", self.path));
    match open::that(&self.path) {
      Ok(_) => log_info(&format!("Successfully opened path: {:?}", self.path)),
      Err(e) => log_error(&format!("Failed to open path {:?}: {}", self.path, e)),
    }
  }
}