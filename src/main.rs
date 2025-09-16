// debugビルドでない場合（つまり release ビルドの場合）に "windows" サブシステムを使用
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod file_drag;
// mod logger; // logger は lib.rs にお引越ししたから、ここからは消すね！
mod child_window; // 新しいお家を教えてあげるよ！
mod mywindow;
mod settings;
mod window_utils; // 便利屋さんのお家も教えてあげる！

use arboard::Clipboard;
use desktop_grouping::tray::tray_icon::create_tray;
use file_drag::IconInfo;
use logger::{log_debug, log_info, log_warn, log_error};
use mywindow::UserEvent;
use std::rc::Rc;

// generate_child_id, ChildSettings など必要なものをインポート
use desktop_grouping::logger; // logger モジュールをライブラリから使うよ！

use settings::{ChildSettings, generate_child_id, get_settings_reader, get_settings_writer};
use winit::{
    dpi::PhysicalPosition,
    event::{
        DeviceEvent, ElementState, Event, MouseButton, MouseScrollDelta, StartCause, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoopBuilder},
    keyboard::{Key, NamedKey},
};

const MOUSE_WHEEL_PIXEL_TO_LINE_FACTOR: f64 = 30.0; // スクロールの変換係数 (環境に合わせて調整)

// トレイメニューのIDを定数化するよっ！٩(ˊᗜˋ*)و
const MENU_ID_NEW_GROUP: &str = "1001";
const MENU_ID_QUIT: &str = "1002";

/// アプリケーションのエントリーポイント。
///
/// # 説明
///
/// イベントループを作成し、ウィンドウやトレイアイコンを初期化して実行します。
fn main() {
    // ロガーの初期化
    logger::init();

    // イベントループの作成
    let event_loop = EventLoopBuilder::with_user_event()
        .build()
        .expect("Failed to create event loop");

    // メインウィンドウ作成 (非表示)
    let _main_window = window_utils::create_main_window(&event_loop); // 便利屋さんにお願い！

    // WindowManager の初期化
    // WindowManager の初期化時にクリップボードも初期化
    let clipboard = Clipboard::new().ok(); // エラーは許容する (None になる)
    let mut manager = mywindow::WindowManager::new(clipboard); // new に引数を追加

    // トレイアイコンの作成
    let _tray = create_tray();
    // トレイイベント用プロキシ
    let proxy = event_loop.create_proxy();
    tray_icon::menu::MenuEvent::set_event_handler(Some(move |event| {
        if let Err(e) = proxy.send_event(UserEvent::MenuEvent(event)) {
            // イベント送信失敗時のログを追加 (より丁寧なエラーハンドリング)
            logger::log_error(&format!("Failed to send MenuEvent: {:?}", e));
        }
    }));

    // --- イベントループの実行 ---
    event_loop
        .run(move |event, target| {
            // target は EventLoopWindowTarget
            target.set_control_flow(ControlFlow::Wait);
            match event {
                Event::NewEvents(StartCause::Init) => handle_new_events_init(target, &mut manager),
                Event::WindowEvent { event, window_id } => {
                    handle_window_event(target, &mut manager, event, window_id)
                }
                Event::DeviceEvent { event, .. } => handle_device_event(&mut manager, event),
                Event::UserEvent(user_event) => handle_user_event(target, &mut manager, user_event),
                Event::LoopExiting => {
                    log_info("Exiting application...");
                    // --- 終了時の保存処理は不要になったよ！ ---
                }
                _ => {} // 他のイベントは無視
            }
        })
        .expect("Failed to start event loop");
}

/// `Event::NewEvents(StartCause::Init)` イベントを処理するよ！
fn handle_new_events_init(
    target: &winit::event_loop::EventLoopWindowTarget<UserEvent>,
    manager: &mut mywindow::WindowManager,
) {
    // --- 設定から既存の子ウィンドウを読み込む ---
    {
        // settings_reader のスコープを限定
        let settings_reader = get_settings_reader();
        for (id_str, child_setting) in settings_reader.children.iter() {
            log_info(&format!("Loading child window: {}", id_str));
            // --- ウィンドウの初期位置を決めるよ！ ---
            let mut initial_position = PhysicalPosition::new(child_setting.x, child_setting.y); // まずは今までの仮想座標を使うね！

            // もしモニター情報があったら…
            if let (Some(monitor_name), Some(monitor_x), Some(monitor_y)) = (
                &child_setting.monitor_name,
                child_setting.monitor_x,
                child_setting.monitor_y,
            ) {
                let mut found_monitor = false;
                // 今つながってるモニターの中に、覚えてた名前のモニターがあるか探すよ！
                for monitor_handle in target.available_monitors() {
                    if monitor_handle.name().as_deref() == Some(monitor_name.as_str()) {
                        // あった！٩(ˊᗜˋ*)و
                        let current_monitor_pos = monitor_handle.position(); // そのモニターの今の場所
                        // 新しいウィンドウの位置を計算するよ！ (モニターの場所 + モニターの中の相対的な場所)
                        initial_position.x = current_monitor_pos.x + monitor_x;
                        initial_position.y = current_monitor_pos.y + monitor_y;
                        log_info(&format!(
                            "Window {} restored to monitor '{}' at relative ({}, {}), virtual ({}, {})",
                            id_str,
                            monitor_name,
                            monitor_x,
                            monitor_y,
                            initial_position.x,
                            initial_position.y
                        ));
                        found_monitor = true;
                        break; // 見つかったからループは終わり！
                    }
                }
                if !found_monitor {
                    // あれ～？覚えてたモニターが見つからなかった…(´・ω・`)
                    log_warn(&format!(
                        "Window {} - Monitor '{}' not found. Falling back to virtual coordinates ({}, {}).",
                        id_str, monitor_name, child_setting.x, child_setting.y
                    ));

                    // --- フォールバック位置の検証 ---
                    let mut position_is_visible = false;
                    for monitor in target.available_monitors() {
                        let monitor_pos = monitor.position();
                        let monitor_size = monitor.size();
                        let monitor_right = monitor_pos.x + monitor_size.width as i32;
                        let monitor_bottom = monitor_pos.y + monitor_size.height as i32;

                        // ウィンドウの左上がモニター内に入っているか簡易チェック
                        if initial_position.x >= monitor_pos.x && initial_position.x < monitor_right &&
                           initial_position.y >= monitor_pos.y && initial_position.y < monitor_bottom {
                            position_is_visible = true;
                            break;
                        }
                    }

                    // どのモニターにも表示されない位置なら、プライマリモニターに強制移動
                    if !position_is_visible {
                        log_warn(&format!(
                            "Window {} - Fallback position ({}, {}) is not on any visible monitor. Moving to primary monitor.",
                            id_str, initial_position.x, initial_position.y
                        ));
                        if let Some(primary_monitor) = target.primary_monitor() {
                            let monitor_pos = primary_monitor.position();
                            let monitor_size = primary_monitor.size();
                            // プライマリモニターの中央あたりに配置
                            initial_position.x = monitor_pos.x + (monitor_size.width as i32 / 4);
                            initial_position.y = monitor_pos.y + (monitor_size.height as i32 / 4);
                            log_info(&format!(
                                "Window {} moved to primary monitor center at ({}, {}).",
                                id_str, initial_position.x, initial_position.y
                            ));
                        } else {
                            // プライマリモニターすら取れない最悪のケース… (0, 0) にする
                            log_error("Could not get primary monitor. Falling back to (0, 0).");
                            initial_position.x = 0;
                            initial_position.y = 0;
                        }
                    }
                }
            } else {
                // モニター情報がなかったから、今まで通り仮想座標を使うね！
                log_info(&format!(
                    "Window {} - No monitor-specific info. Using virtual coordinates ({}, {}).",
                    id_str, child_setting.x, child_setting.y
                ));
            }
            let mut effective_settings = child_setting.clone();
            effective_settings.x = initial_position.x;
            effective_settings.y = initial_position.y;
            let child_window =
                window_utils::create_child_window(&target, Some(&effective_settings)); // 便利屋さんにお願い！
            let child_window_id = child_window.id();

            // アイコン復元処理だよっ！
            // manager にウィンドウを登録してからアイコン情報をロードするね！

            // manager にウィンドウと id_str、設定情報を登録
            manager.insert(
                &child_window_id,
                Rc::new(child_window),
                id_str.clone(),
                child_setting,
            );
            // 次に、設定から読み込んだアイコンパスを使ってアイコンを復元し、manager 経由で追加
            manager.restore_icons(&child_window_id, &child_setting.icons);
            manager.backmost(&child_window_id);

            if let Some(child_win) = manager.get_window_ref(&child_window_id) {
                child_win.request_redraw();
            }
        }
    } // settings_reader のスコープ終了
}

/// `Event::WindowEvent` を処理するよ！
fn handle_window_event(
    target: &winit::event_loop::EventLoopWindowTarget<UserEvent>,
    manager: &mut mywindow::WindowManager,
    event: WindowEvent,
    window_id: winit::window::WindowId,
) {
    // manager が管理していないウィンドウからのイベントは無視
    if !manager.has_window(&window_id) {
        return;
    }

    match event {
        WindowEvent::CloseRequested => {
            log_info(&format!("Close requested for window: {:?}", window_id));
            target.exit();
        }
        WindowEvent::Focused(_) => {
            manager.backmost(&window_id);
        }
        WindowEvent::KeyboardInput { event, .. } => {
            match event.logical_key {
                Key::Named(NamedKey::Shift) => {
                    if event.state.is_pressed() {
                        manager.focused_id = Some(window_id);
                    }
                    manager.set_resizing_keybord_state(event.state.is_pressed());
                }
                Key::Named(NamedKey::Control) => {
                    if event.state.is_pressed() {
                        manager.focused_id = Some(window_id);
                    }
                    manager.set_moving_keybord_state(event.state.is_pressed());
                }
                // V キーの処理 (Ctrl+V)
                // manager.is_moving.keybord_pressed は Ctrl キーが押されているかを示しているよ。
                Key::Character(ref s) if s.eq_ignore_ascii_case("v") => {
                    if event.state == ElementState::Pressed && manager.is_moving.keybord_pressed {
                        log_debug(&format!("Ctrl+V detected for window: {:?}", window_id));
                        manager.handle_paste(window_id);
                    }
                }
                _ => {}
            }
        }
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            // --- スケールファクター変更イベントの処理 ---
            // `ScaleFactorChanged` イベントは、ウィンドウのDPIスケーリングが変わった時に発生するよ！
            // これを検知して、ウィンドウの見た目の位置と、中身の描画スケールを調整するんだ。
            // `new_inner_size` は winit の最近のバージョンではこのイベントに含まれなくなったみたい。
            // サイズの変更は `Resized` イベントで処理されるから、ここでは主にスケールファクターの更新と
            // それに伴う位置の調整、再描画の要求をするよ！
            log_info(&format!(
                "Window {:?}: ScaleFactorChanged to {}",
                window_id, scale_factor
            ));
            if let Some(child_window) = manager.get_child_window_mut(&window_id) {
                let old_scale_factor = child_window.scale_factor; // 前の拡大率を覚えておく
                child_window.update_scale_factor(scale_factor); // 新しい拡大率を ChildWindow と MyGraphics に教える

                // 見た目の位置が変わらないように、物理的な位置を調整するよ！
                if let Ok(current_physical_pos) = child_window.window.outer_position() {
                    let logical_pos_x = current_physical_pos.x as f64 / old_scale_factor;
                    let logical_pos_y = current_physical_pos.y as f64 / old_scale_factor;

                    let new_physical_pos_x = (logical_pos_x * scale_factor) as i32;
                    let new_physical_pos_y = (logical_pos_y * scale_factor) as i32;

                    child_window
                        .window
                        .set_outer_position(PhysicalPosition::new(
                            new_physical_pos_x,
                            new_physical_pos_y,
                        ));
                    log_debug(&format!(
                        "Position adjusted to ({}, {})",
                        new_physical_pos_x, new_physical_pos_y
                    ));
                }
                child_window.window.request_redraw(); // 再描画をお願い！
            }
        }
        WindowEvent::MouseInput { state, button, .. } => {
            match button {
                MouseButton::Left => {
                    if state == ElementState::Pressed {
                        manager.execute_group_item(window_id);
                    }
                    manager.is_resizing.mouse_pressed = state.is_pressed();
                    manager.is_moving.mouse_pressed = state.is_pressed();
                    manager.start_dragging();
                }
                MouseButton::Right => {
                    if state == ElementState::Pressed {
                        // Ctrl+右クリックでのアイテム削除 (remove_group_item内でCtrlチェック)
                        // manager.is_moving.keybord_pressed は Ctrl キーが押されているかを示しているよ。
                        manager.remove_group_item(window_id);
                        // Ctrlキーが押されていなければ、アイコンの場所を開くよ！
                        if !manager.is_moving.keybord_pressed {
                            manager.open_icon_location(window_id);
                        }
                    }
                }
                _ => {}
            }
            // マウスボタンが離された時、かつ移動またはリサイズ操作中だった場合に保存
            // manager.is_moving.keybord_pressed (Ctrl) や manager.is_resizing.keybord_pressed (Shift) の状態を見てるね！
            if state == ElementState::Released
                && (manager.is_moving.keybord_pressed || manager.is_resizing.keybord_pressed)
            {
                log_debug(&format!(
                    "Mouse released after move/resize on window {:?}. Saving settings.",
                    window_id
                ));
                manager.save_child_settings(window_id);
            }
        }
        WindowEvent::RedrawRequested => {
            manager.draw_window(&window_id);
        }
        WindowEvent::Resized(size) => {
            manager.resize(&window_id, size);
        }
        WindowEvent::DroppedFile(path) => {
            let icon = IconInfo::new(path);
            log_debug(&format!("Dropped Icon: {:#?}", icon));
            manager.focused_id = Some(window_id);
            manager.add_group(icon);
            if let Some(child_win) = manager.get_window_ref(&window_id) {
                child_win.request_redraw();
            }
        }
        WindowEvent::CursorMoved { position, .. } => {
            manager.update_cursor_pos(window_id, position);
            let current_hover = manager.find_icon_at_relative_pos(window_id, position);
            manager.update_hover_state(current_hover);
            manager.set_last_cursor_window(Some(window_id)); // ★最後にカーソルがあったウィンドウを記録
        }
        WindowEvent::CursorLeft { .. } => {
            if let Some((hover_id, _)) = manager.hovered_icon {
                if hover_id == window_id {
                    manager.update_hover_state(None);
                }
            }
            manager.set_last_cursor_window(None); // ★カーソルが離れたことを記録
            // ドラッグ操作中かもしれないので、ここで保存するのが安全
            log_debug(&format!(
                "Cursor left window {:?}. Saving settings.",
                window_id
            ));
            manager.save_child_settings(window_id);
        }
        WindowEvent::Moved(..) => {} // ウィンドウ移動完了時のイベント
        _ => {}
    }
}

/// `Event::DeviceEvent` を処理するよ！
fn handle_device_event(manager: &mut mywindow::WindowManager, event: DeviceEvent) {
    match event {
        DeviceEvent::MouseMotion { .. } => {
            manager.start_resizing();
        }
        DeviceEvent::MouseWheel { delta } => {
            // Ctrl キーの状態を WindowManager から取得 (is_moving が Ctrl に対応)
            // manager.is_moving.keybord_pressed は Ctrl キーが押されているかを示しているよ。
            if manager.is_moving.keybord_pressed {
                let y_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => {
                        (pos.y / MOUSE_WHEEL_PIXEL_TO_LINE_FACTOR) as f32
                    }
                };
                if y_delta.abs() > f32::EPSILON {
                    log_debug(&format!("Ctrl+MouseWheel detected: delta_y = {}", y_delta));
                    manager.handle_mouse_wheel(y_delta);
                }
            }
        }
        _ => {}
    }
}

/// `Event::UserEvent` (トレイメニューイベント) を処理するよ！
fn handle_user_event(
    target: &winit::event_loop::EventLoopWindowTarget<UserEvent>,
    manager: &mut mywindow::WindowManager,
    user_event: UserEvent,
) {
    match user_event {
        UserEvent::MenuEvent(event) => match event.id.as_ref() {
            MENU_ID_NEW_GROUP => {
                // "New Group" の処理だよ！
                log_info("MenuEvent: New Group.");
                let new_id_str = generate_child_id();
                let default_settings = ChildSettings::default();
                {
                    let mut settings_writer = get_settings_writer();
                    settings_writer
                        .children
                        .insert(new_id_str.clone(), default_settings.clone());
                    log_info(&format!(
                        "Inserted default settings for new window: {}",
                        new_id_str
                    ));
                }
                let child_window =
                    window_utils::create_child_window(target, Some(&default_settings)); // 便利屋さんにお願い！
                let child_window_id = child_window.id();
                manager.insert(
                    &child_window_id,
                    Rc::new(child_window),
                    new_id_str,
                    &default_settings,
                );
                manager.backmost(&child_window_id);
                if let Some(child_win) = manager.get_window_ref(&child_window_id) {
                    child_win.request_redraw();
                }
            }
            MENU_ID_QUIT => {
                // "Quit" の処理だよ！
                log_info("MenuEvent: Quit.");
                target.exit();
            }
            _ => {
                // 知らないメニューイベントが来ちゃった！Σ(ﾟДﾟ)
                log_debug(&format!("Unknown MenuEvent received: ID = {:?}", event.id));
            }
        },
    }
}
