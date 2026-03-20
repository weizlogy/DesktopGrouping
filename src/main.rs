#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;
use desktop_grouping::{graphics, logger, tray, win32, settings::manager};

fn main() -> Result<(), windows::core::Error> {
    // 1. ロガーの初期化
    logger::init();
    log::info!("Desktop Grouping v3.0.0 (Native) Starting...");

    // 2. 引数の解析と設定の更新
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    let mut settings_changed = false;
    
    while i < args.len() {
        match args[i].as_str() {
            "--font" if i + 1 < args.len() => {
                let font_family = args[i + 1].clone();
                let mut settings = manager::get_settings_writer();
                settings.app.font_family = font_family;
                settings_changed = true;
                i += 2;
            }
            "--fsize" if i + 1 < args.len() => {
                if let Ok(size) = args[i + 1].parse::<f32>() {
                    let mut settings = manager::get_settings_writer();
                    settings.app.font_size = size;
                    settings_changed = true;
                }
                i += 2;
            }
            _ => i += 1,
        }
    }

    if settings_changed {
        manager::save();
        log::info!("Settings updated from command line arguments.");
    }

    // 3. COM の初期化 (WIC や DirectComposition で必要)
    unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        )?;
    }

    // 3. グラフィックスエンジンの初期化 (全ウィンドウで共有)
    let engine = Rc::new(graphics::GraphicsEngine::new()?);

    // 3. メインウィンドウを作成 (非表示。常駐用)
    let _window = win32::Window::new("DesktopGroupingClass", "Desktop Grouping Native")?;

    // 4. トレイアイコンを作成
    let _tray = tray::tray_icon::create_tray();
    log::info!("Tray icon created. Running in background...");

    // 5. メッセージループを開始 (エンジンを渡す)
    win32::run_message_loop(engine)?;

    log::info!("Application exiting.");
    Ok(())
}
