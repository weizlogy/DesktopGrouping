use std::path::PathBuf;

use desktop_grouping::win32::ui_shell;
use windows::Win32::Graphics::Gdi::BITMAPINFO;

#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
static ICON_FETCH_MUTEX: Mutex<()> = Mutex::new(()); // テストの時だけ使う秘密の鍵だよっ！
use crate::logger::{log_debug, log_error, log_info};

/// ドラッグされたファイルの情報を保持する構造体
#[derive(Debug)]
pub struct IconInfo {
    pub path: PathBuf,
    pub name: String,
    // アイコンデータは遅延読み込みしてキャッシュする
    icon_cache: Option<(BITMAPINFO, Vec<u8>)>,
}

impl IconInfo {
    /// 新しい `IconInfo` インスタンスを（遅延読み込みで）作成します。
    ///
    /// ファイルのパスから、ファイル名（拡張子なし）を取得します。
    /// この時点ではアイコンの読み込みは行わず、パスと名前のみを保持します。
    /// アイコンデータは `get_or_load_icon` が初めて呼ばれたときに読み込まれます。
    pub fn new(path: PathBuf) -> Self {
        // ファイル名を取得するよ！拡張子はナシでね！(ゝω・)v
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str()) // OsStr を &str に変換するよ
            .map(|s| s.to_string()) // &str を String にするよ
            .unwrap_or_else(|| {
                // もし file_stem が取れなかったり、UTF-8じゃなかったら…
                path.file_name() // ファイル名全体を試してみるよ！
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned()) // それでもダメなら、パス全体を名前にしちゃえ！(๑•̀ㅂ•́)و✧
            });

        log_info(&format!(
            "IconInfo作成 (遅延): Path: {:?}, Name: {:?}",
            path, name
        ));

        IconInfo {
            path,
            name,
            icon_cache: None, // この時点ではアイコンデータは空 (None)
        }
    }

    /// アイコンデータを取得または遅延読み込みします。
    ///
    /// 内部にキャッシュがあればそれを返し、なければディスクから読み込みます。
    /// 読み込みに成功しても失敗しても、結果はキャッシュされ、次回以降はキャッシュが返されます。
    pub fn get_or_load_icon(&mut self) -> &(BITMAPINFO, Vec<u8>) {
        // `get_or_insert_with` を使うと、`icon_cache` が `None` の場合だけクロージャが実行されて、
        // 結果がキャッシュに保存される。とてもスマートな方法。
        self.icon_cache.get_or_insert_with(|| {
            log_debug(&format!("Lazy loading icon for: {:?}", self.path));
            #[cfg(test)]
            let _guard = ICON_FETCH_MUTEX.lock().unwrap(); // テストの時は、順番にアイコンを取りに行く

            // Windows API を呼び出してファイルアイコンを取得
            match ui_shell::get_file_icon(&self.path) {
                Ok(icon_data) => {
                    log_info(&format!("Icon loaded successfully for: {:?}", self.path));
                    icon_data
                }
                Err(e) => {
                    log_error(&format!(
                        "Failed to lazy load file icon for {:?}: {}. Using default.",
                        self.path, e
                    ));
                    // アイコン取得に失敗した場合、
                    // 毎回失敗しないように、空のアイコン情報をキャッシュしておく
                    (BITMAPINFO::default(), Vec::new())
                }
            }
        })
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
        let mut icon_info = IconInfo::new(file_path.clone());

        // --- 確認するよっ！ ---
        assert_eq!(icon_info.path, file_path);
        assert_eq!(icon_info.name, "test_file"); // 拡張子なしの名前になってるかな？

        // get_or_load_icon を呼んで、アイコンデータが空っぽじゃないことを確認
        let icon_data = icon_info.get_or_load_icon();
        assert!(!icon_data.1.is_empty(), "アイコンデータが空っぽだよ！＞＜");

        // --- お片付け ---
        dir.close().unwrap(); // 一時ディレクトリを消すよ！
    }

    // 拡張子がないファイルから IconInfo を作るテストだよ！
    #[test]
    fn test_icon_info_new_without_extension() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file_no_ext"); // 拡張子なしのファイル！
        File::create(&file_path).unwrap();

        let mut icon_info = IconInfo::new(file_path.clone());

        assert_eq!(icon_info.path, file_path);
        assert_eq!(icon_info.name, "test_file_no_ext"); // そのままの名前になってるかな？
        let icon_data = icon_info.get_or_load_icon();
        assert!(!icon_data.1.is_empty(), "アイコンデータが空っぽだよ！＞＜");

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
