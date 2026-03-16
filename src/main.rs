#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;
use desktop_grouping::{graphics, logger, tray, win32};

fn main() -> Result<(), windows::core::Error> {
    // 1. ロガーの初期化
    logger::init();
    log::info!("Desktop Grouping v3.0.0 (Native) Starting...");

    // 2. COM の初期化 (WIC や DirectComposition で必要)
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
