use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN};

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
    pub font_family: String,
}

impl AppSettings {
    pub fn validate(&mut self) {
        self.font_size = self.font_size.clamp(8.0, 72.0);
        if self.font_family.is_empty() {
            self.font_family = "Meiryo".to_string();
        }
    }
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
    pub icon_size: f32, // アイコンの論理サイズ (デフォルト 48.0)
    pub icons: Vec<PersistentIconInfo>,
    
    // --- マルチモニター・高DPI対応のための追加フィールド ---
    pub monitor_name: Option<String>, 
    pub monitor_x: Option<i32>,       
    pub monitor_y: Option<i32>,       
    pub dpi_scale: f32, // 保存時の DPI スケーリング倍率 (1.0 = 100%, 1.5 = 150% 等)
}

impl ChildSettings {
    pub fn validate(&mut self) {
        self.opacity = self.opacity.clamp(0.1, 1.0);
        self.icon_size = self.icon_size.clamp(16.0, 256.0);
        self.width = self.width.max(50);
        self.height = self.height.max(50);

        if self.bg_color.is_empty() || !self.bg_color.starts_with('#') {
            self.bg_color = "#FFFFFF99".to_string();
        }

        // 画面外に飛び出している場合の救済措置
        unsafe {
            let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            // ウィンドウの左上が画面外なら (100, 100) に戻す
            // 厳密には「完全に外」かを判定すべきだが, 救済を優先するよ
            if self.x < vx || self.x > vx + vw || self.y < vy || self.y > vy + vh {
                log::warn!("Window position ({}, {}) is out of screen. Resetting to (100, 100).", self.x, self.y);
                self.x = 100;
                self.y = 100;
            }
        }
    }
}

/// 設定ファイル全体の構造。
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Settings {
    pub app: AppSettings,
    pub children: HashMap<String, ChildSettings>, // キーは ID 文字列 (タイムスタンプ)
}

impl Settings {
    pub fn validate(&mut self) {
        self.app.validate();
        for child in self.children.values_mut() {
            child.validate();
        }
    }
}

// --- 各構造体のデフォルト値の実装 ---

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            font_family: "Meiryo".to_string(),
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
            icon_size: 48.0,
            icons: Vec::new(),
            monitor_name: None,
            monitor_x: None,
            monitor_y: None,
            dpi_scale: 1.0, // デフォルトは 100%
        }
    }
}
