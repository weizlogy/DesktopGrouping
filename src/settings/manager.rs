use std::sync::{LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
use super::models::Settings;
use super::storage;

/// 全体で共有する設定インスタンスだよ！
static GLOBAL_SETTINGS: LazyLock<RwLock<Settings>> = LazyLock::new(|| {
    // 起動時にファイルを読み込む
    match storage::load_settings() {
        Ok(settings) => RwLock::new(settings),
        Err(e) => {
            log::error!("CRITICAL: {}", e);
            log::error!("Settings will be reset to defaults if you save. Check config.toml manualy.");
            RwLock::new(Settings::default())
        }
    }
});

/// 設定値へのアクセサ関数 (読み取り用)
pub fn get_settings_reader() -> RwLockReadGuard<'static, Settings> {
    GLOBAL_SETTINGS
        .read()
        .expect("Failed to acquire read lock on settings")
}

/// 設定値へのアクセサ関数 (書き込み用)
pub fn get_settings_writer() -> RwLockWriteGuard<'static, Settings> {
    GLOBAL_SETTINGS
        .write()
        .expect("Failed to acquire write lock on settings")
}

/// 現在の状態をファイルに保存するよ！
pub fn save() {
    let settings = get_settings_reader();
    if let Err(e) = storage::save_settings(&*settings) {
        log::error!("Failed to save settings: {}", e);
    }
}
