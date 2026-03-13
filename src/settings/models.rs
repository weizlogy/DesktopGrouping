use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 設定ファイルに永続化するためのアイコン情報。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PersistentIconInfo {
    pub path: PathBuf,
}

/// アプリケーション全体の共通設定。
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct AppSettings {
    pub font_size: f32,
    pub font_path: String,
}

/// 各グループ（子ウィンドウ）ごとの個別設定。
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct ChildSettings {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub bg_color: String,
    pub opacity: f32, // 0.0 ~ 1.0
    pub icons: Vec<PersistentIconInfo>,
    
    // --- マルチモニター・高DPI対応のための追加フィールド ---
    pub monitor_name: Option<String>, 
    pub monitor_x: Option<i32>,       
    pub monitor_y: Option<i32>,       
    pub dpi_scale: f32, // 保存時の DPI スケーリング倍率 (1.0 = 100%, 1.5 = 150% 等)
}

/// 設定ファイル全体の構造。
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Settings {
    pub app: AppSettings,
    pub children: HashMap<String, ChildSettings>, // キーは ID 文字列 (タイムスタンプ)
}

// --- 各構造体のデフォルト値の実装 ---

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            font_path: String::new(),
        }
    }
}

impl Default for ChildSettings {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 300,
            height: 200,
            bg_color: "#FFFFFF99".to_string(),
            opacity: 1.0,
            icons: Vec::new(),
            monitor_name: None,
            monitor_x: None,
            monitor_y: None,
            dpi_scale: 1.0, // デフォルトは 100%
        }
    }
}
