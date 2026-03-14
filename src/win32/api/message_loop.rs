use std::rc::Rc;
use windows::core::PCWSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MsgWaitForMultipleObjectsEx, PeekMessageW, TranslateMessage, MSG, MWMO_INPUTAVAILABLE, PM_REMOVE, QS_ALLINPUT,
    GetCursorPos, GetWindowRect,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, GetAsyncKeyState, VK_CONTROL};
use windows::Win32::Foundation::{POINT, RECT};
use tray_icon::{TrayIconEvent, menu::MenuEvent};
use crate::ui::group::GroupWindow;
use crate::graphics::GraphicsEngine;
use crate::settings::{manager, models::ChildSettings};
use crate::ui::group::interaction::InteractionAction;
use crate::win32::vproc::window_proc;
use crate::win32::api;

/// ウィンドウメッセージとトレイイベントを処理し続けるループだよ！
pub fn run_message_loop(engine: Rc<GraphicsEngine>) -> Result<(), windows::core::Error> {
    unsafe {
        let mut msg = MSG::default();
        let tray_channel = TrayIconEvent::receiver();
        let menu_channel = MenuEvent::receiver();

        // 1. ウィンドウクラスを1回だけ登録する
        let instance = GetModuleHandleW(None)?;
        let class_name_str = "DesktopGroupingGroupClass";
        let class_name = api::utils::to_wide(class_name_str);
        let class_pcwstr = PCWSTR::from_raw(class_name.as_ptr());

        api::register_class::register_window_class(
            instance.into(),
            class_pcwstr,
            Some(window_proc),
        )?;

        // 複数のグループウィンドウを管理する
        let mut windows: Vec<Box<GroupWindow>> = Vec::new();

        // キーの状態管理
        let mut v_was_down = false;

        // 起動時に設定から既存のグループを復元するよ
        {
            let settings = manager::get_settings_reader();
            for (id, child) in &settings.children {
                log::info!("Restoring group: {}", id);
                let icons = child.icons.iter().map(|i| i.path.clone()).collect();
                match GroupWindow::create(
                    engine.clone(),
                    id.clone(),
                    "Restored Group".to_string(),
                    child.bg_color.clone(),
                    child.opacity,
                    child.width,
                    child.height,
                    icons,
                ) {
                    Ok(mut window) => {
                        windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                            window.hwnd,
                            windows::Win32::UI::WindowsAndMessaging::HWND_BOTTOM,
                            child.x,
                            child.y,
                            0,
                            0,
                            windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                        ).ok();

                        let _ = window.draw();
                        windows.push(window);
                    }
                    Err(e) => log::error!("Failed to restore group {}: {}", id, e),
                }
            }
        }

        loop {
            // 1. Win32 メッセージを全て処理する
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == windows::Win32::UI::WindowsAndMessaging::WM_QUIT {
                    return Ok(());
                }

                // カスタムメッセージの処理: ウィンドウ削除通知
                if msg.message == api::WM_REMOVE_WINDOW {
                    let target_hwnd = windows::Win32::Foundation::HWND(msg.wParam.0 as isize);
                    log::info!("Removing window from management list: {:?}", target_hwnd);
                    windows.retain(|w| w.hwnd != target_hwnd);
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

            // 4. キー入力を監視
            let ctrl_down = (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let v_is_down = (GetAsyncKeyState(0x56) as u16 & 0x8000) != 0;

            if ctrl_down && v_is_down && !v_was_down {
                let mut pt = POINT::default();
                if GetCursorPos(&mut pt).is_ok() {
                    for window in &mut windows {
                        let mut rect = RECT::default();
                        if GetWindowRect(window.hwnd, &mut rect).is_ok() {
                            if pt.x >= rect.left && pt.x <= rect.right && pt.y >= rect.top && pt.y <= rect.bottom {
                                let _ = window.perform_action(InteractionAction::PasteColor);
                                break;
                            }
                        }
                    }
                }
            }
            v_was_down = v_is_down;

            // 5. 定期的なメンテナンス (現在は特に不要)

            // 6. 次のイベントが来るまで待機

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
    match event.id.0.as_str() {
        "1001" => { // New Group
            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string();
            let title = "New Group".to_string();
            let bg_color = "#000000".to_string();
            let opacity = 0.5f32;
            let width = 300u32;
            let height = 200u32;

            {
                let mut settings = manager::get_settings_writer();
                settings.children.insert(id.clone(), ChildSettings {
                    x: 100, y: 100, width, height, bg_color: bg_color.clone(), opacity, ..Default::default()
                });
                drop(settings);
                manager::save();
            }

            match GroupWindow::create(engine.clone(), id, title, bg_color, opacity, width, height, Vec::new()) {
                Ok(mut window) => {
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
