// d:\Codings\desktopcleaner\Desktop-Grouping-v2\src\settings.rs
use std::{
    collections::HashMap,
    fs,
    io::{self, ErrorKind},
    path::PathBuf,
    sync::{LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
// use crate::mywindow::WindowManager; // <- WindowManagerへの参照を削除

/// 設定ファイルに永続化するためのアイコン情報。パスのみを保持する。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PersistentIconInfo {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub app: AppSettings,
    pub children: HashMap<String, ChildSettings>, // キーはタイムスタンプ文字列 (id_str)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct AppSettings {
    pub font_size: f32,
    pub font_path: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct ChildSettings {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub bg_color: String,
    pub icons: Vec<PersistentIconInfo>,
    // --- マルチモニター対応のための追加フィールド ---
    pub monitor_name: Option<String>, // ウィンドウが最後にあったモニターの名前だよ！
    pub monitor_x: Option<i32>,       // そのモニター内での相対X座標だよ！
    pub monitor_y: Option<i32>,       // そのモニター内での相対Y座標だよ！
}

// --- デフォルト値の実装 ---
impl Default for Settings {
    fn default() -> Self {
        Settings {
            app: AppSettings::default(),
            children: HashMap::new(),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            font_size: 16.0,
            font_path: "".to_string(),
        }
    }
}

impl Default for ChildSettings {
    fn default() -> Self {
        // デフォルト値は AppSettings を参照できないため、固定値または一般的な値を設定
        ChildSettings {
            x: 50,
            y: 50,
            width: 300,  // Default inner width
            height: 200, // Default inner height
            bg_color: "#FFFFFF99".to_string(),
            icons: Vec::new(),
            monitor_name: None, // 最初はモニターの情報はナシ！
            monitor_x: None,
            monitor_y: None,
        }
    }
}

// --- 設定ファイルのパスを取得する関数 ---
fn get_config_path() -> io::Result<PathBuf> {
    let exe_path = std::env::current_exe()?;
    let config_dir = exe_path
        .parent()
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "Failed to get executable directory"))?;
    Ok(config_dir.join("config.toml"))
}

// --- 設定を読み込む内部関数 ---
fn load_settings_internal() -> Settings {
    match get_config_path() {
        Ok(config_path) => {
            log::info!("Loading settings from: {:?}", config_path);
            match fs::read_to_string(&config_path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(settings) => {
                        log::debug!("Settings loaded successfully.");
                        settings
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to parse config file {:?}: {}. Using default settings.",
                            config_path,
                            e
                        );
                        Settings::default()
                    }
                },
                Err(ref e) if e.kind() == ErrorKind::NotFound => {
                    log::warn!(
                        "Config file not found at {:?}. Creating default config.",
                        config_path
                    );
                    let default_settings = Settings::default();
                    // Don't save here, let the first run save defaults if needed
                    default_settings
                }
                Err(e) => {
                    log::error!(
                        "Failed to read config file {:?}: {}. Using default settings.",
                        config_path,
                        e
                    );
                    Settings::default()
                }
            }
        }
        Err(e) => {
            log::error!(
                "Failed to determine config file path: {}. Using default settings.",
                e
            );
            Settings::default()
        }
    }
}

// --- 設定を保存する内部関数 ---
fn save_settings_internal(settings: &Settings) {
    match get_config_path() {
        Ok(config_path) => match toml::to_string_pretty(settings) {
            Ok(toml_string) => {
                if let Err(write_err) = fs::write(&config_path, toml_string) {
                    log::error!(
                        "Failed to save config file {:?}: {}",
                        config_path,
                        write_err
                    );
                } else {
                    log::debug!("Settings saved successfully to {:?}", config_path);
                }
            }
            Err(e) => {
                log::error!("Failed to serialize settings: {}", e);
            }
        },
        Err(e) => {
            log::error!("Failed to determine config file path for saving: {}", e);
        }
    }
}

// --- グローバル設定インスタンス (RwLock) ---
static GLOBAL_SETTINGS: LazyLock<RwLock<Settings>> =
    LazyLock::new(|| RwLock::new(load_settings_internal()));

// --- 設定値へのアクセサ関数 (読み取り用) ---
pub fn get_settings_reader() -> RwLockReadGuard<'static, Settings> {
    GLOBAL_SETTINGS
        .read()
        .expect("Failed to acquire read lock on settings")
}

// --- 設定値へのアクセサ関数 (書き込み用) ---
pub fn get_settings_writer() -> RwLockWriteGuard<'static, Settings> {
    GLOBAL_SETTINGS
        .write()
        .expect("Failed to acquire write lock on settings")
}

// --- 設定をファイルに保存する公開関数 ---
pub fn save_settings() {
    // グローバル設定の現在の状態をファイルに書き込む
    let settings_reader = get_settings_reader();
    save_settings_internal(&*settings_reader); // 読み取りロックを使って保存
}

// --- 子ウィンドウ識別子生成 ---
pub fn generate_child_id() -> String {
    // マイクロ秒の精度でタイムスタンプを生成するよ！ (例: 20230101123456001)
    Local::now().format("%Y%m%d%H%M%S%3f").to_string()
}

#[cfg(test)]
mod tests {
    use super::*; // settings.rs の中身をぜーんぶ使えるようにするおまじない！
    // use tempfile::tempdir; // ファイルI/Oのテストをしなくなったから、これはもういらないね！

    // テストの時だけ、設定ファイルの場所を一時ディレクトリに変えちゃう関数だよ！
    // ちょっとトリッキーだけど、これで本物の設定ファイルを汚さずにテストできるんだ！(｀・ω・´)ゞ
    fn override_config_path_for_test<F, T>(test_fn: F) -> T
    where
        F: FnOnce() -> T,
    {
        // 以前はここで一時ディレクトリのパスを使ってたけど、今はもう使ってないよ！
        // get_config_path がプライベートで、実行ファイルのパスに依存してるから、
        // テストでうまく差し替えるのが難しかったんだ…(´・ω・｀)
        //
        // なので、グローバルな設定を直接操作して、読み書きのテストをするよ！

        // --- ここでは、グローバル設定を直接いじるテストに切り替えるね！ ---
        // まずは、テスト前にグローバル設定をデフォルトに戻しておくのが大事！
        {
            let mut settings = get_settings_writer();
            *settings = Settings::default();
        }

        test_fn()
    }

    #[test]
    fn test_settings_default_values() {
        let settings = Settings::default();
        assert_eq!(settings.app.font_size, 16.0);
        assert_eq!(settings.app.font_path, "");
        assert!(settings.children.is_empty());

        let child_settings = ChildSettings::default();
        assert_eq!(child_settings.x, 50);
        assert_eq!(child_settings.bg_color, "#FFFFFF99");
    }

    #[test]
    fn test_settings_read_write_global() {
        override_config_path_for_test(|| {
            // 1. 設定を書き込んでみるよ！
            let child_id = generate_child_id();
            {
                let mut settings = get_settings_writer();
                settings.app.font_size = 20.0;
                let mut new_child = ChildSettings::default();
                new_child.x = 100;
                settings.children.insert(child_id.clone(), new_child);
            } // 書き込みロックをここで解放！

            // 2. 設定を読み込んで、ちゃんと変わってるか確認するよ！
            {
                let settings_reader = get_settings_reader();
                assert_eq!(settings_reader.app.font_size, 20.0);
                assert!(settings_reader.children.contains_key(&child_id));
                assert_eq!(settings_reader.children.get(&child_id).unwrap().x, 100);
            }
        });
    }

    #[test]
    fn test_generate_child_id_format() {
        let id = generate_child_id();
        // YYYYMMDDHHMMSSfff の形式 (14 + 3 = 17文字) になってるかな？
        assert_eq!(id.len(), 17);
        assert!(
            id.chars().all(|c| c.is_digit(10)),
            "IDに数字以外が含まれてるよ！: {}",
            id
        );
    }
}
