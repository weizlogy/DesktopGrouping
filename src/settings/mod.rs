pub mod models;
pub mod storage;
pub mod manager;

pub use models::*;
pub use manager::{get_settings_reader, get_settings_writer, save as save_settings};

use chrono::Local;

/// 子ウィンドウ識別子を生成するよ！ (ID 文字列)
pub fn generate_child_id() -> String {
    // 以前と同様, YYYYMMDDHHMMSSfff 形式にするね
    Local::now().format("%Y%m%d%H%M%S%3f").to_string()
}
