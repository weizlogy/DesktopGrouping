use std::path::PathBuf;

use desktop_grouping::win32::ui_shell;
use windows::Win32::Graphics::Gdi::BITMAPINFO;

#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
static ICON_FETCH_MUTEX: Mutex<()> = Mutex::new(()); // テストの時だけ使う秘密の鍵だよっ！
use crate::logger::{log_error, log_info};

/// ドラッグされたファイルの情報を保持する構造体
#[derive(Debug)]
pub struct IconInfo {
  pub path: PathBuf,
  pub name: String,
  pub icon: (BITMAPINFO, Vec<u8>),
}

impl IconInfo {
  /// 新しい `IconInfo` インスタンスを作るよ！
  ///
  /// ファイルのパスから、ファイル名（拡張子なし）とアイコン情報を取得するんだ。
  /// もしファイル名がうまく取れなかったら、パス全体を名前にしちゃうこともあるよ！(・ω<)
  /// アイコン取得に失敗したら、デフォルトの空っぽアイコンになっちゃうから気をつけてね！
  ///
  /// テストの時は、アイコン取得が順番こになるように `ICON_FETCH_MUTEX` っていう秘密の鍵を使ってるんだ♪
  pub fn new(path: PathBuf) -> Self {
    // ファイル名を取得するよ！拡張子はナシでね！(ゝω・)v
    let name =
      path.file_stem()
        .and_then(|stem| stem.to_str()) // OsStr を &str に変換するよ
        .map(|s| s.to_string()) // &str を String にするよ
        .unwrap_or_else(|| { // もし file_stem が取れなかったり、UTF-8じゃなかったら…
            path.file_name() // ファイル名全体を試してみるよ！
                .and_then(|name| name.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| path.to_string_lossy().into_owned()) // それでもダメなら、パス全体を名前にしちゃえ！(๑•̀ㅂ•́)و✧
        });


    log_info(&format!("IconInfo作成中… Path: {:?}, Name: {:?}", path, name));

    let icon = {
        #[cfg(test)]
        let _guard = ICON_FETCH_MUTEX.lock().unwrap(); // テストの時は、順番にアイコンを取りに行くよ！
        // Windowsくんからファイルアイコンをもらってくるよ！
        match ui_shell::get_file_icon(&path) {
            Ok(icon_data) => {
                log_info(&format!("Icon: {:?}", icon_data.0));
                icon_data
            }
            Err(e) => {
                log_error(&format!("Failed to get file icon for {:?}: {}. Using default.", path, e));
                // アイコン取得に失敗しちゃった…(´・ω・｀) とりあえず空っぽのアイコン情報を返すね。
                (BITMAPINFO::default(), Vec::new())
            }
        }
    };


    IconInfo { path, name, icon }
  }

  /// この IconInfo が示すパスを実行（開く）します。
  /// `open::that` を使って、関連付けられたアプリケーションでファイルやフォルダを開くよ！
  pub fn execute(&self) {
    log_info(&format!("Executing path: {:?}", self.path));
    match open::that(&self.path) {
      Ok(_) => log_info(&format!("Successfully opened path: {:?}", self.path)),
      Err(e) => log_error(&format!("Failed to open path {:?}: {}", self.path, e)),
    }
  }
}

#[cfg(test)]
mod tests {
    use super::*; // file_drag.rs の中身をぜーんぶ使えるようにするおまじない！
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir; // 一時的なファイルやディレクトリを作るのに便利だよ！

    // 拡張子があるファイルから IconInfo を作るテストだよ！
    #[test]
    fn test_icon_info_new_with_extension() {
        // --- 準備するよっ！ ---
        let dir = tempdir().unwrap(); // 一時ディレクトリを作るよ！
        let file_path = dir.path().join("test_file.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "じぇみにだよ！").unwrap(); // ちょっとだけ書き込んでみる！

        // --- 実行してみるよっ！ ---
        let icon_info = IconInfo::new(file_path.clone());

        // --- 確認するよっ！ ---
        assert_eq!(icon_info.path, file_path);
        assert_eq!(icon_info.name, "test_file"); // 拡張子なしの名前になってるかな？
        // アイコンデータが空っぽじゃないことを確認！ (ui_shell がちゃんと何か返してればOK！)
        assert!(!icon_info.icon.1.is_empty(), "アイコンデータが空っぽだよ！＞＜");

        // --- お片付け ---
        dir.close().unwrap(); // 一時ディレクトリを消すよ！
    }

    // 拡張子がないファイルから IconInfo を作るテストだよ！
    #[test]
    fn test_icon_info_new_without_extension() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file_no_ext"); // 拡張子なしのファイル！
        File::create(&file_path).unwrap();

        let icon_info = IconInfo::new(file_path.clone());

        assert_eq!(icon_info.path, file_path);
        assert_eq!(icon_info.name, "test_file_no_ext"); // そのままの名前になってるかな？
        assert!(!icon_info.icon.1.is_empty(), "アイコンデータが空っぽだよ！＞＜");

        dir.close().unwrap();
    }

    // IconInfo の execute メソッドがパニックせずに実行できるかテストするよ！
    #[test]
    fn test_icon_info_execute_runs_without_panic() {
        // --- 準備するよっ！ ---
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("executable_test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "実行テストだよ！").unwrap();

        let icon_info = IconInfo::new(file_path.clone());

        // --- 実行してみるよっ！ ---
        // open::that が実際に何かを開こうとするけど、テストではパニックしないことを確認！
        // (環境によっては何か開いちゃうかもだけど、それはテストの範囲外ってことで！(ゝω・)v)
        icon_info.execute();

        // --- お片付け ---
        dir.close().unwrap();
    }
}