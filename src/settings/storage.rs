use std::{
    fs,
    io,
    path::PathBuf,
};
use super::models::Settings;

/// 設定ファイルの保存先ディレクトリを解決するよ！
/// `%APPDATA%/DesktopGrouping` を使うように変更するね。
fn get_settings_dir() -> io::Result<PathBuf> {
    // 実行ファイルの隣ではなく, 標準的な設定保存場所を取得するよ
    let mut path = if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata)
    } else {
        // 万が一 APPDATA がない場合は実行ファイルの隣にフォールバック
        let exe_path = std::env::current_exe()?;
        exe_path.parent().unwrap().to_path_buf()
    };
    
    path.push("DesktopGrouping");
    
    // ディレクトリがなければ作成するよ！
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    
    Ok(path)
}

/// `config.toml` へのフルパスを取得するよ！
pub fn get_config_path() -> io::Result<PathBuf> {
    Ok(get_settings_dir()?.join("config.toml"))
}

/// 設定ファイルを読み込むよ！
/// 読み込みに失敗した場合は Error を返して, デフォルト値を勝手に返さないようにするね。
pub fn load_settings() -> Result<Settings, String> {
    let config_path = get_config_path().map_err(|e| e.to_string())?;
    
    if !config_path.exists() {
        log::info!("Config file not found. Using default settings.");
        return Ok(Settings::default());
    }

    let contents = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let mut settings: Settings = match toml::from_str(&contents) {
        Ok(s) => s,
        Err(e) => {
            let bad_path = config_path.with_extension("toml.bad");
            log::error!("Failed to parse config file: {}. Backing up to {:?}", e, bad_path);
            let _ = fs::rename(&config_path, &bad_path); // 失敗ファイルを退避
            return Err(format!("Settings corruption detected. Original file saved as .bad"));
        }
    };

    // 論理バリデーションを実行
    settings.validate();

    Ok(settings)
}

/// 設定ファイルを安全に保存するよ！ (アトミック書き込み)
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let config_path = get_config_path().map_err(|e| e.to_string())?;
    let tmp_path = config_path.with_extension("tmp");

    let toml_string = toml::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    // 1. 一時ファイルに書き出す
    fs::write(&tmp_path, toml_string)
        .map_err(|e| format!("Failed to write temporary config file: {}", e))?;

    // 2. 元のファイルにリネーム（アトミックな置き換え）
    // Windows では std::fs::rename がアトミックであることを利用するよ
    fs::rename(&tmp_path, &config_path)
        .map_err(|e| {
            // リネームに失敗した場合は一時ファイルを消しておく
            let _ = fs::remove_file(&tmp_path);
            format!("Failed to finalize config file save (rename error): {}", e)
        })?;

    log::debug!("Settings saved atomically to {:?}", config_path);
    Ok(())
}
