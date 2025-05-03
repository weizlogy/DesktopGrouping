// debugビルドでない場合（つまり release ビルドの場合）に "windows" サブシステムを使用
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod file_drag;
mod logger;
mod mywindow;
mod settings;

use std::rc::Rc;

use arboard::Clipboard;
use desktop_grouping_v2::tray::tray_icon::create_tray;
use file_drag::IconInfo;
use logger::{log_debug, log_info};
use mywindow::UserEvent;

// generate_child_id, ChildSettings など必要なものをインポート
use settings::{
  generate_child_id, get_settings_reader, get_settings_writer, save_settings, ChildSettings,
};
use winit::{
  event::{DeviceEvent, ElementState, Event, MouseButton, MouseScrollDelta, StartCause, WindowEvent},
  event_loop::{ControlFlow, EventLoopBuilder},
  keyboard::{Key, NamedKey},
};

const MOUSE_WHEEL_PIXEL_TO_LINE_FACTOR: f64 = 30.0; // スクロールの変換係数 (環境に合わせて調整)

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
  let _main_window = mywindow::create_main_window(&event_loop);

  // WindowManager の初期化
  // WindowManager の初期化時にクリップボードも初期化
  let clipboard = Clipboard::new().ok(); // エラーは許容する (None になる)
  let mut manager = mywindow::WindowManager::new(clipboard); // new に引数を追加

  // トレイアイコンの作成
  let _tray = create_tray();
  // トレイイベント用プロキシ
  let proxy = event_loop.create_proxy();
  tray_icon::menu::MenuEvent::set_event_handler(Some(move |event| {
    proxy
      .send_event(UserEvent::MenuEvent(event))
      .expect("Failed to send MenuEvent");
  }));

  // --- イベントループの実行 ---
  event_loop
    .run(move |event, target| { // target は EventLoopWindowTarget
      target.set_control_flow(ControlFlow::Wait);

      match event {
        Event::NewEvents(StartCause::Init) => {
          // --- 設定から既存の子ウィンドウを読み込む ---
          {
            // settings_reader のスコープを限定
            let settings_reader = get_settings_reader();
            for (id_str, child_setting) in settings_reader.children.iter() {
              log_info(&format!("Loading child window: {}", id_str));

              let child_window = mywindow::create_child_window(&target, Some(child_setting));
              let child_window_id = child_window.id();

              // --- ★アイコン復元処理を追加★ ---
              // manager にウィンドウを登録 *する前* にアイコン情報を ChildWindow にロードする方が良いか、
              // manager に登録 *した後* に manager 経由で追加する方が良いか検討。
              // ここでは manager に登録した後に追加する方式を採用。

              // まず manager にウィンドウと id_str、設定情報を登録
              manager.insert(&child_window_id, Rc::new(child_window), id_str.clone(), child_setting);

              // 次に、設定から読み込んだアイコンパスを使ってアイコンを復元し、manager 経由で追加
              // child_setting.icons を manager のメソッドに渡す
              // restore_icons を呼び出す前に child_window の参照が必要な場合があるため注意
              // (例: 復元中にエラーが発生した場合にウィンドウを閉じるなど)
              // ここでは manager 経由で問題ない想定
              manager.restore_icons(&child_window_id, &child_setting.icons);

              // 必要であれば最背面に移動など
              manager.backmost(&child_window_id);

              // 復元後に再描画を要求 (任意)
              if let Some(child_win) = manager.get_window_ref(&child_window_id) {
                  child_win.request_redraw();
              }
            }
          } // settings_reader のスコープ終了
        },
        Event::WindowEvent { event, window_id } => {
          // manager が管理していないウィンドウからのイベントは無視 (メインウィンドウなど)
          if !manager.has_window(&window_id) {
              // log_debug(&format!("Ignoring event for unmanaged window ID: {:?}", window_id));
              return;
          }

          match event {
            WindowEvent::CloseRequested => {
              // 特定の子ウィンドウを閉じるか、アプリ全体を終了するか選択できる
              // ここではアプリ全体を終了
              log_info(&format!("Close requested for window: {:?}", window_id));
              target.exit();
            }
            WindowEvent::Focused(_) => { // focused が true/false 両方で発生
              // 本プログラムはデスクトップランチャーであり常に最背面を維持する必要があるため、
              // フォーカスが当たった時に最背面に送る
              manager.backmost(&window_id);
            }
            WindowEvent::KeyboardInput { event, .. } => {
              match event.logical_key {
                // Ctrl と Shift の状態更新 (既存のコード)
                Key::Named(NamedKey::Shift) => {
                  // Shiftキー押下時もフォーカス対象ウィンドウを更新する
                  // (Ctrl+Vなどの操作対象を一貫させるため)
                  if event.state.is_pressed() {
                      manager.focused_id = Some(window_id);
                  }
                  manager.set_resizing_keybord_state(event.state.is_pressed());
                }
                Key::Named(NamedKey::Control) => {
                  // Ctrlキー押下時もフォーカス対象ウィンドウを更新する
                  if event.state.is_pressed() {
                      manager.focused_id = Some(window_id);
                  }
                  manager.set_moving_keybord_state(event.state.is_pressed());
                }
                // V キーの処理 (Ctrl+V)
                // winit 0.29 では Key::Character("v") または Key::Named(NamedKey::KeyV)
                // Key::Character を使う方が一般的
                Key::Character(ref s) if s.eq_ignore_ascii_case("v") => {
                  // V キーが押された瞬間 (Pressed) かつ Ctrl キーが押されている状態か
                  if event.state == ElementState::Pressed && manager.is_moving.keybord_pressed {
                    log_debug(&format!("Ctrl+V detected for window: {:?}", window_id));
                    // handle_paste を呼び出す (イベントが発生したウィンドウIDを渡す)
                    manager.handle_paste(window_id);
                  }
                }
                // 他のキーが押された場合、フォーカスIDをリセットしないように注意
                // (Ctrlを押しながら他のキーを押す場合があるため)
                _ => {}
              }
            },
            WindowEvent::MouseInput { state, button, .. } => {
              match button {
                MouseButton::Left => {
                  if state == ElementState::Pressed {
                    // ダブルクリック判定とアイテム実行
                    manager.execute_group_item(window_id);
                  }
                  // 移動/リサイズ状態の更新
                  manager.is_resizing.mouse_pressed = state.is_pressed();
                  manager.is_moving.mouse_pressed = state.is_pressed();
                  // ドラッグ移動開始
                  manager.start_dragging();
                }
                MouseButton::Right => {
                  if state == ElementState::Pressed {
                    // Ctrl+右クリックでのアイテム削除 (remove_group_item内でCtrlチェック)
                    // またはウィンドウ削除
                    manager.remove_group_item(window_id);
                    // ★追加だよっ！: Ctrlなし右クリックで場所を開く処理♪
                    if !manager.is_moving.keybord_pressed { // Ctrlキーが押されてないかなー？ってチェック！
                      manager.open_icon_location(window_id);
                    }
                  }
                }
                _ => {}
              }
            }
            WindowEvent::RedrawRequested => {
              // ウィンドウの再描画
              manager.draw_window(&window_id);
            }
            WindowEvent::Resized(size) => {
              // ウィンドウのリサイズ処理
              manager.resize(&window_id, size);
            }
            WindowEvent::DroppedFile(path) => {
              // ファイルドロップ時の処理
              let icon = IconInfo::new(path);
              log_debug(&format!("Dropped Icon: {:#?}", icon));
              manager.focused_id = Some(window_id); // ドロップされたウィンドウをフォーカス
              manager.add_group(icon); // アイコンを追加
              // 追加後に再描画を要求 (add_group 内で request_redraw しても良い)
              if let Some(child_win) = manager.get_window_ref(&window_id) {
                  child_win.request_redraw();
              }
            }
            WindowEvent::CursorMoved { position, .. } => {
              // カーソル移動時の処理 (ホバー判定など)
              manager.update_cursor_pos(window_id, position); // カーソル位置を記録
              let current_hover = manager.find_icon_at_relative_pos(window_id, position);
              manager.update_hover_state(current_hover); // ホバー状態を更新
              manager.set_last_cursor_window(Some(window_id)); // ★最後にカーソルがあったウィンドウを記録
            }
            WindowEvent::CursorLeft { .. } => {
              // カーソルがウィンドウから離れた時の処理
              if let Some((hover_id, _)) = manager.hovered_icon {
                if hover_id == window_id {
                  manager.update_hover_state(None); // ホバー解除
                }
              }
              manager.set_last_cursor_window(None); // ★カーソルが離れたことを記録
              // --- ★設定保存処理を追加 ---
              log_debug(&format!("Cursor left window {:?}. Requesting settings save.", window_id));
              manager.save_window_settings(window_id);
              // --------------------------
            }
            WindowEvent::Moved(_position) => {
              // ウィンドウ移動完了時のイベント (必要ならここで何か処理)
              // 位置の保存は終了時に update_settings_from_windows で行う
              // log_debug(&format!("Window {:?} moved to {:?}", window_id, position));
            }
            _ => {} // 他のウィンドウイベントは無視
          }
        }
        Event::DeviceEvent { event, .. } => match event {
          DeviceEvent::MouseMotion { .. } => {
            // マウス移動イベント (ウィンドウ外も含む)
            manager.start_resizing(); // リサイズ操作の開始判定
          }
          DeviceEvent::MouseWheel { delta } => {
            // Ctrl キーの状態を WindowManager から取得 (is_moving が Ctrl に対応)
            // target.keyboard_modifiers() はここでは使えない
            if manager.is_moving.keybord_pressed { // <- 修正点: manager の状態を直接参照
              // winit の MouseScrollDelta::LineDelta または PixelDelta を処理
              let y_delta = match delta {
                  MouseScrollDelta::LineDelta(_, y) => y, // winit::event:: を省略可能に
                  MouseScrollDelta::PixelDelta(pos) => {
                      // ピクセルデルタの場合、適当なスケールで LineDelta に近似
                      // スクロール方向が逆になる可能性があるので注意 (環境依存)
                      // 一般的には PixelDelta の y が正なら上、負なら下
                      // LineDelta の y が正なら上、負なら下
                      // そのまま使うか、符号を反転させるか要確認
                      (pos.y / MOUSE_WHEEL_PIXEL_TO_LINE_FACTOR) as f32
                  }
              };
              if y_delta.abs() > f32::EPSILON {
                  log_debug(&format!("Ctrl+MouseWheel detected: delta_y = {}", y_delta));
                  // handle_mouse_wheel に渡す delta_y の符号が期待通りか確認
                  // (上スクロールで増加、下スクロールで減少させたい場合など)
                  manager.handle_mouse_wheel(y_delta);
              }
            }
          }
          _ => {} // 他のデバイスイベントは無視
        },
        Event::UserEvent(UserEvent::MenuEvent(event)) => {
          // トレイメニューからのイベント
          if event.id.as_ref() == "1001" { // New Group
            log_info("MenuEvent: New Group.");

            // 1. 新しいウィンドウ用のユニークID生成
            let new_id_str = generate_child_id();
            // 2. デフォルト設定を作成
            let default_settings = ChildSettings::default();
            // 3. デフォルト設定をグローバル設定に即時保存 (ファイル保存は終了時)
            {
              // 書き込みロックのスコープ
              let mut settings_writer = get_settings_writer();
              settings_writer
                .children
                .insert(new_id_str.clone(), default_settings.clone());
              log_info(&format!(
                "Inserted default settings for new window: {}",
                new_id_str
              ));
            } // 書き込みロック解放

            // 4. 新しい子ウィンドウを作成 (target を渡す)
            //    create_child_window の引数を EventLoopWindowTarget に戻す必要あり
            //    *** 再度修正: create_child_window は &EventLoopWindowTarget を受け取る ***
            let child_window = mywindow::create_child_window(target, Some(&default_settings));
            let child_window_id = child_window.id();

            // 5. manager にウィンドウと id_str を登録
            manager.insert(&child_window_id, Rc::new(child_window), new_id_str, &default_settings);
            manager.backmost(&child_window_id); // 最背面に

            // 6. 初回描画を要求 (任意)
            if let Some(child_win) = manager.get_window_ref(&child_window_id) {
                child_win.request_redraw();
            }
          }
          if event.id.as_ref() == "1002" { // Quit
            log_info("MenuEvent: Quit.");
            target.exit(); // イベントループを終了
          }
        }
        Event::LoopExiting => {
          log_info("Exiting application...");
          // --- 終了前に現在のウィンドウ状態を設定に反映 ---
          manager.update_settings_from_windows(); // <- まず設定を更新
          // --- 設定をファイルに保存 ---
          save_settings(); // <- 次に保存 (引数なし)
        }
        _ => {} // 他のイベントは無視
      }
    })
    .expect("Failed to start event loop");
}
