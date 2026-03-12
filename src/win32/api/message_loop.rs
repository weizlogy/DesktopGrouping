use std::rc::Rc;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MsgWaitForMultipleObjectsEx, PeekMessageW, TranslateMessage, MSG, MWMO_INPUTAVAILABLE, PM_REMOVE, QS_ALLINPUT,
};
use tray_icon::{TrayIconEvent, menu::MenuEvent};
use crate::ui::group::GroupWindow;
use crate::graphics::GraphicsEngine;

/// ウィンドウメッセージとトレイイベントを処理し続けるループだよ！
pub fn run_message_loop(engine: Rc<GraphicsEngine>) -> Result<(), windows::core::Error> {
    unsafe {
        let mut msg = MSG::default();
        let tray_channel = TrayIconEvent::receiver();
        let menu_channel = MenuEvent::receiver();

        // 複数のグループウィンドウを管理する
        // アドレスを固定するために Box で管理するよ
        let mut windows: Vec<Box<GroupWindow>> = Vec::new();

        loop {
            // 1. Win32 メッセージを全て処理する
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == windows::Win32::UI::WindowsAndMessaging::WM_QUIT {
                    return Ok(());
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // 2. トレイアイコンのイベントを処理する
            if let Ok(event) = tray_channel.try_recv() {
                handle_tray_event(event);
            }

            // 3. メニューのイベントを処理する
            if let Ok(event) = menu_channel.try_recv() {
                handle_menu_event(event, &engine, &mut windows);
            }

            // 4. 定期的なメンテナンス (最背面維持など)
            for window in &windows {
                crate::win32::api::show_window::move_to_bottom(window.hwnd);
            }

            // 5. 次のイベントが来るまで待機して CPU 負荷を下げる
            MsgWaitForMultipleObjectsEx(None, 10, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        }
    }
}

fn handle_tray_event(event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click { .. } => {
            log::info!("Tray icon clicked!");
        }
        _ => {}
    }
}

fn handle_menu_event(
    event: MenuEvent,
    engine: &Rc<GraphicsEngine>,
    windows: &mut Vec<Box<GroupWindow>>
) {
    log::info!("Menu item clicked: {}", event.id.0);

    match event.id.0.as_str() {
        "1001" => { // New Group
            log::info!("Creating new group window with DirectX...");
            match GroupWindow::create(engine.clone(), "New Group".to_string(), "#00000080".to_string(), 300, 200) {
                Ok(mut window) => {
                    // 初回描画
                    let _ = window.draw();
                    windows.push(window);
                }
                Err(e) => log::error!("Failed to create group window: {}", e),
            }
        }
        "1002" => { // Quit
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
            }
        }
        _ => {}
    }
}
