use std::{collections::HashMap, rc::Rc, time::{Duration, Instant}};

use arboard::Clipboard;
use desktop_grouping::win32::ui_wam;

use rand::Rng;
use tiny_skia::Color;
use winit::{
  dpi::PhysicalPosition, window::{Window, WindowId, ResizeDirection}
};

use crate::{
  file_drag::IconInfo, logger::*, settings::*, child_window::*, window_utils::show_confirmation_dialog
};

/// ダブルクリックと判定する時間閾値 (ミリ秒)
const DOUBLE_CLICK_THRESHOLD_MS: u64 = 500;

/// アプリケーション内で発生するカスタムイベント。
/// 現在はトレイアイコンのメニューイベントのみ。
#[derive(Debug)]
pub enum UserEvent {
  MenuEvent(tray_icon::menu::MenuEvent)
}

/// ウィンドウ全体を管理する構造体。
/// 子ウィンドウの集合や、フォーカス、移動/リサイズ状態などを管理します。
pub struct WindowManager {
  /// 子ウィンドウのマップ。キーは `WindowId`、値は `ChildWindow`。
  children: HashMap<WindowId, ChildWindow>,
  /// 現在フォーカスされている（操作対象の）子ウィンドウのID。
  pub focused_id: Option<WindowId>,
  /// ウィンドウ移動操作の状態。
  pub is_moving: WindowControl,
  /// ウィンドウリサイズ操作の状態。
  pub is_resizing: WindowControl,
  /// 現在マウスカーソルがホバーしているアイコンの情報。(ウィンドウID, アイコンインデックス)
  pub hovered_icon: Option<(WindowId, usize)>,
  /// ダブルクリック判定用の最後にクリックされた時刻とウィンドウID。
  last_click: Option<(WindowId, Instant)>,
  /// 各ウィンドウにおける最後のマウスカーソル位置。アイコン特定に使用。
  last_cursor_pos: HashMap<WindowId, PhysicalPosition<f64>>,
  clipboard: Option<Clipboard>, // ★追加
  last_cursor_window_id: Option<WindowId>, // ★追加: ホイールイベント用
}

/// ウィンドウ操作（移動/リサイズ）の状態を管理する構造体。
/// 対応するキー（Ctrl/Shift）とマウスボタンの押下状態を保持します。
#[derive(Debug)]
pub struct WindowControl {
  /// 対応するキーボードのキーが押されているか。
  pub keybord_pressed: bool,
  /// マウスの左ボタンが押されているか。
  pub mouse_pressed: bool,
}

impl WindowControl {
  /// 新しい `WindowControl` インスタンスを作成します。
  /// 初期状態ではキーボードもマウスも押されていない (`false`) 状態です。
  pub fn new() -> WindowControl {
    return WindowControl {
      keybord_pressed: false,
      mouse_pressed: false,
    };
  }

  /// 対応するキーとマウス左ボタンの両方が押されているかを判定します。
  /// ウィンドウの移動やリサイズ操作が可能かどうかを判断するために使用します。
  ///
  /// # 戻り値
  ///
  /// 両方が押されている場合は `true`、それ以外は `false`。
  pub fn can_control(&self) -> bool {
    return self.keybord_pressed && self.mouse_pressed;
  }
}

impl WindowManager {
  /// 新しい `WindowManager` インスタンスを作成します。
  /// 子ウィンドウリストや各種状態を初期化します。
  pub fn new(clipboard: Option<Clipboard>) -> WindowManager {
    return WindowManager {
      children: HashMap::new(), // 子ウィンドウマップを空で初期化
      focused_id: None,         // 最初はフォーカスされているウィンドウはない
      is_moving: WindowControl::new(), // 移動状態を初期化
      is_resizing: WindowControl::new(), // リサイズ状態を初期化
      hovered_icon: None,       // 最初はホバーされているアイコンはない
      last_click: None,         // ダブルクリック判定情報を初期化
      last_cursor_pos: HashMap::new(), // カーソル位置マップを空で初期化
      clipboard, // ★初期化
      last_cursor_window_id: None, // ★初期化
    };
  }

  /// 指定された `WindowId` が管理対象の子ウィンドウに存在するかどうかを確認します。
  pub fn has_window(&self, id: &WindowId) -> bool {
    self.children.contains_key(id)
  }

  /// 指定された `WindowId` に対応する `winit::window::Window` への参照を取得します。
  pub fn get_window_ref(&self, id: &WindowId) -> Option<&Window> {
    // self.children から ChildWindow を取得し、その中の window フィールドへの参照を返す
    self.children.get(id).map(|cw| &*cw.window) // Rc<Window> から &Window を取得
  }

  /// 管理対象の子ウィンドウを追加します。
  ///
  /// # 引数
  ///
  /// * `id` - 追加する子ウィンドウの `WindowId`。
  /// * `window` - 追加する `winit::window::Window` インスタンス (Rcでラップ)。
  /// * `id_str` - このウィンドウを識別するためのユニークな文字列ID。設定の読み書きに使用。
  pub fn insert(
    &mut self,
    id: &WindowId,
    window: Rc<Window>,
    id_str: String,
    // ★設定情報を受け取るように変更
    settings: &ChildSettings,
  ) {
    // ChildWindow::new に色情報を渡す
    self.children.insert(
      *id,
      ChildWindow::new(
        window,
        id_str,
        &settings.bg_color, // ★渡す
        &settings.border_color, // ★渡す
      ),
    );
  }

  /// Ctrl+V ペーストイベントを処理します。
  pub fn handle_paste(&mut self, window_id: WindowId) {
    if let Some(clipboard) = &mut self.clipboard {
      match clipboard.get_text() {
        Ok(text) => {
          log_debug(&format!("Clipboard text received: {}", text));
          // 対応する ChildWindow のメソッドを呼び出す
          if let Some(child) = self.children.get_mut(&window_id) {
            let trimmed_text = text.trim(); // 前後の空白を除去
            if trimmed_text.eq_ignore_ascii_case("#Random") {
                // "#Random" がペーストされた場合
                log_info(&format!("Window {}: Received #Random command. Generating random color.", child.id_str));
                let mut rng = rand::thread_rng();
                let r: u8 = rng.r#gen();
                let g: u8 = rng.r#gen();
                let b: u8 = rng.r#gen();
                // アルファ値はデフォルトの半透明 (0x99 = 153) にする (既存のデフォルトに合わせる)
                // もし不透明にしたければ 255 にする
                let random_color = Color::from_rgba8(r, g, b, 153);
                
                // 生成した Color を #RRGGBBAA 形式の文字列に変換
                // (既存の color_to_hex_string ヘルパー関数を利用)
                let color_str = color_to_hex_string(random_color);
                log_debug(&format!("Window {}: Generated random color: {}", child.id_str, color_str));

              // ChildWindow の set_background_color を呼び出す
              // set_background_color は &str を受け取るので、変換後の文字列を渡す
              child.set_background_color(&color_str);
              // --- ★設定保存処理を追加 ---
              self.save_child_settings(window_id);
            } else {
              // 通常の色コードがペーストされた場合 (既存の処理)
              child.set_background_color(trimmed_text);
              // --- ★設定保存処理を追加 ---
              self.save_child_settings(window_id);
            }
          }
        }
        Err(e) => {
          log_error(&format!("Failed to get clipboard text: {}", e));
        }
      }
    } else {
      log_warn("Clipboard is not available.");
    }
  }

  /// Ctrl+マウスホイールイベントを処理します。
  pub fn handle_mouse_wheel(&mut self, delta_y: f32) {
    // 最後にカーソルがあったウィンドウIDを使用
    if let Some(window_id) = self.last_cursor_window_id {
      if let Some(child) = self.children.get_mut(&window_id) {
        // delta_y の符号で方向を判断 (正が上、負が下など、環境依存確認)
        // ここでは delta_y が正なら増加、負なら減少と仮定
        let old_alpha = child.graphics.get_background_color().alpha(); // 保存前に現在のアルファ値を取得
        child.adjust_alpha(delta_y);
        // 実際にアルファ値が変わった場合のみ設定を保存
        if (child.graphics.get_background_color().alpha() - old_alpha).abs() > f32::EPSILON {
            self.save_child_settings(window_id);
        }      }
    }
  }

  /// 最後にカーソルがあったウィンドウIDを記録します。
  pub fn set_last_cursor_window(&mut self, window_id: Option<WindowId>) {
      self.last_cursor_window_id = window_id;
  }

  /// 指定された子ウィンドウの現在の状態（位置、サイズ、色、アイコン）を
  /// グローバル設定に反映し、ファイルに即時保存します。
  ///
  /// # 引数
  /// * `window_id` - 設定を保存する子ウィンドウの `WindowId`。
  pub fn save_child_settings(&mut self, window_id: WindowId) {
    // 対象の子ウィンドウを取得
    let child_window = match self.children.get(&window_id) {
        Some(cw) => cw,
        None => {
            log_error(&format!("設定保存対象のウィンドウが見つかりません (ID: {:?})", window_id));
            return;
        }
    };

    let id_str = child_window.id_str.clone(); // 設定キーとして使うID

    { // 設定書き込みロックのスコープ
    // 設定への書き込みロックを取得
    let mut settings = get_settings_writer();

    // グローバル設定から、対応するIDの子ウィンドウ設定を取得 (可変参照)
    if let Some(child_settings) = settings.children.get_mut(&id_str) {
      // --- 位置とサイズの保存 ---
      match child_window.window.outer_position() {
        Ok(pos) => {
          child_settings.x = pos.x; // 仮想デスクトップ座標 (今まで通り！)
          child_settings.y = pos.y; // 仮想デスクトップ座標 (今まで通り！)

          // --- マルチモニター情報の保存だよっ！ ---
          if let Some(monitor) = child_window.window.current_monitor() {
            child_settings.monitor_name = monitor.name(); // モニターの名前をゲット！
            let monitor_pos = monitor.position(); // モニター自体の仮想座標
            child_settings.monitor_x = Some(pos.x - monitor_pos.x); // モニター内での相対X座標！
            child_settings.monitor_y = Some(pos.y - monitor_pos.y); // モニター内での相対Y座標！
            log_debug(&format!(
                "Window {} on monitor '{}' (virt: {:?}, mon_pos: {:?}, rel: ({:?}, {:?}))",
                id_str,
                child_settings.monitor_name.as_deref().unwrap_or("N/A"),
                pos,
                monitor_pos,
                child_settings.monitor_x,
                child_settings.monitor_y
            ));
          } else {
            // あれれ？モニターが取れなかった…(´・ω・｀) 情報はクリアしとこっと。
            child_settings.monitor_name = None;
            child_settings.monitor_x = None;
            child_settings.monitor_y = None;
            log_warn(&format!("Window {} - Could not get current monitor.", id_str));
          }
        }
        Err(e) => { log_error(&format!("ウィンドウの位置取得に失敗 (id_str: {}): {}", id_str, e)); }
      }
      let size = child_window.window.inner_size();
      child_settings.width = size.width;
      child_settings.height = size.height;

      // --- 色情報の保存 ---
      child_settings.bg_color = color_to_hex_string(child_window.graphics.get_background_color());
      child_settings.border_color = color_to_hex_string(child_window.graphics.get_border_color());

      // --- アイコン情報の保存 ---
      child_settings.icons = child_window.groups.iter()
        .map(|icon_info| PersistentIconInfo { path: icon_info.path.clone() })
        .collect();
      } else {
        log_error(&format!("保存時に設定エントリが見つかりません (id_str: {})", id_str));
      }
    }
    // --- 設定をファイルに即時保存 ---
    save_settings(); // settings.rs の save_settings を呼び出す
  }

  /// 設定から読み込んだアイコンパス情報に基づいて、指定されたウィンドウにアイコンを復元します。
  ///
  /// # 引数
  /// * `window_id` - アイコンを復元する対象のウィンドウID。
  /// * `persistent_icons` - 永続化されていたアイコンパス情報のリスト (`Vec<PersistentIconInfo>`)。
  pub fn restore_icons(&mut self, window_id: &WindowId, persistent_icons: &[PersistentIconInfo]) {
    // 対象の子ウィンドウ (可変参照) を取得
    if let Some(child) = self.children.get_mut(window_id) {
      log_info(&format!("Restoring {} icons for window {}", persistent_icons.len(), child.id_str));
      // 永続化されていた各アイコンパスについてループ
      for p_icon in persistent_icons {
        // パス情報を使って IconInfo::new を呼び出し、アイコン情報を再生成
        // 注意: IconInfo::new は失敗する可能性がある (expectを使っている場合パニック)
        //       より堅牢にするには Result を返すように IconInfo::new を変更し、
        //       ここでエラーハンドリング (ログ出力など) を行うのが望ましい。
        //       今回は既存コードに合わせて expect を使うか、ログ出力に留める。
        log_debug(&format!("Attempting to restore icon from path: {:?}", p_icon.path));


        match std::panic::catch_unwind(|| IconInfo::new(p_icon.path.clone())) {
          Ok(icon_info) => {
            child.groups.push(icon_info);
          }
          Err(_) => {
            log_error(&format!(
              "Failed to restore icon (panic!) from path {:?}. Skipping.",
              p_icon.path
            ));
          }
        }
      }
    } else {
      log_error(&format!("アイコン復元対象のウィンドウが見つかりません (ID: {:?})", window_id));
    }
  }

  /// ウィンドウ移動操作のためのキーボード状態を設定します (通常はCtrlキー)。
  ///
  /// # 引数
  ///
  /// * `state` - キーが押されている (`true`) か、離された (`false`) か。
  pub fn set_moving_keybord_state(&mut self, state: bool) {
    self.is_moving.keybord_pressed = state;
    // キーが離されたら、フォーカスも解除する
    if !state {
      self.focused_id = None;
    }
  }

  /// ウィンドウリサイズ操作のためのキーボード状態を設定します (通常はShiftキー)。
  ///
  /// # 引数
  ///
  /// * `state` - キーが押されている (`true`) か、離された (`false`) か。
  pub fn set_resizing_keybord_state(&mut self, state: bool) {
    self.is_resizing.keybord_pressed = state;
    // キーが離されたら、フォーカスも解除する
    if !state {
      self.focused_id = None;
    }
  }

  /// ウィンドウのドラッグ移動操作を開始します。
  /// 移動キー (Ctrl) とマウス左ボタンが両方押されており、
  /// かつフォーカスされているウィンドウがある場合に、OSにウィンドウ移動を依頼します。
  pub fn start_dragging(&mut self) {
    // 移動操作が可能かチェック
    if !self.is_moving.can_control() || self.focused_id.is_none() {
      return; // 条件を満たさなければ何もしない
    }
    // フォーカスされている子ウィンドウを取得
    if let Some(focused_id) = self.focused_id {
      if let Some(child) = self.children.get(&focused_id) {
        child.start_os_drag(); // ChildWindow ちゃんにお願い！
      } else {
        log_error(&format!("Drag target child window not found for focused_id: {:?}", focused_id));
      }
    }
  }

  /// ウィンドウのリサイズ操作を開始します。
  pub fn start_resizing(&mut self) {
    // リサイズ操作が可能かチェック
    if !self.is_resizing.can_control() || self.focused_id.is_none() {
      return; // 条件を満たさなければ何もしない
    }
    // フォーカスされている子ウィンドウを取得
    if let Some(focused_id) = self.focused_id {
      if let Some(child) = self.children.get(&focused_id) {
        child.start_os_resize(ResizeDirection::SouthEast); // ChildWindow ちゃんにお願い！
      } else {
        log_error(&format!("Resize target child window not found for focused_id: {:?}", focused_id));
      }
    }
  }

  /// 指定されたIDのウィンドウをデスクトップの最背面 (他のウィンドウの後ろ) に移動します。
  pub fn backmost(&mut self, id: &WindowId) {
    // 対象の子ウィンドウを取得
    let child =
      self.children.get(id).expect("最背面移動対象の子ウィンドウ取得に失敗");

    // win32 API を呼び出してウィンドウを最背面に移動
    ui_wam::set_window_pos_to_bottom(&child.window);
  }

  /// 指定されたIDのウィンドウの内容を描画します。
  pub fn draw_window(&mut self, id: &WindowId) {
    // 管理している子ウィンドウがない場合は何もしない
    if self.children.is_empty() {
      return;
    }

    // 描画対象の子ウィンドウ (可変参照) を取得
    let child = self.children.get_mut(id)
      .expect("描画対象の子ウィンドウ取得に失敗");

    // このウィンドウ上でホバーされているアイコンのインデックスを取得
    let hovered_index =
      self.hovered_icon.and_then(|(hover_id, hover_idx)| {
        // hovered_icon のウィンドウIDが、描画対象のウィンドウIDと一致する場合のみ Some(インデックス) を返す
        if hover_id == *id { Some(hover_idx) } else { None }
    });
    // ChildWindow の draw メソッドを呼び出し、ホバーインデックスを渡す
    child.draw(hovered_index);
  }

  /// 指定されたIDのウィンドウのサイズが変更されたときに呼び出されます。
  pub fn resize(&mut self, id: &WindowId, new_size: winit::dpi::PhysicalSize<u32>) {
    // 管理している子ウィンドウがない場合は何もしない
    if self.children.is_empty() {
      return;
    }
    // 対象の子ウィンドウ (可変参照) を取得
    if let Some(child) = self.children.get_mut(id) {
      // ChildWindow の resize メソッドを呼び出す
      child.resize_graphics(new_size); // グラフィックスのリサイズを指示
      // サイズ変更後に再描画を要求
      child.window.request_redraw();
    }
  }

  /// 現在フォーカスされている子ウィンドウにアイコン情報を追加します。
  pub fn add_group(&mut self, icon: IconInfo) {
    // 子ウィンドウがない、またはフォーカスされているウィンドウがない場合は何もしない
    if self.children.is_empty() || self.focused_id.is_none() {
      return;
    }
    // フォーカスされている子ウィンドウ (可変参照) を取得
    let child =
      self.children.get_mut(&self.focused_id.unwrap())
        .expect("アイコン追加対象の子ウィンドウ取得に失敗");
    // ChildWindow の add メソッドを呼び出す
    child.add(icon);
    // アイコン追加後に再描画を要求 (任意だが推奨)
    child.window.request_redraw();
    // --- ★設定保存処理を追加 ---
    self.save_child_settings(self.focused_id.unwrap());
  }

  /// 指定されたウィンドウにおけるマウスカーソルの最新位置を記録します。
  pub fn update_cursor_pos(&mut self, window_id: WindowId, position: PhysicalPosition<f64>) {
    self.last_cursor_pos.insert(window_id, position);
  }

  /// マウスの左クリックイベントを処理します。
  pub fn execute_group_item(&mut self, window_id: WindowId) {
    let now = Instant::now(); // 現在時刻を取得
    let mut is_double_click = false; // ダブルクリックフラグ

    // 前回のクリック情報を確認
    if let Some((last_id, last_time)) = self.last_click {
      // 前回と同じウィンドウIDで、かつ閾値時間内にクリックされたか？
      if last_id == window_id && now.duration_since(last_time) < Duration::from_millis(DOUBLE_CLICK_THRESHOLD_MS) {
        // ダブルクリックと判定
        is_double_click = true;
        self.last_click = None; // ダブルクリックが成立したのでリセット
      } else {
        // シングルクリック (または閾値超過) なので、今回のクリック情報を保存
        self.last_click = Some((window_id, now));
      }
    } else {
      // 初めてのクリックなので、今回のクリック情報を保存
      self.last_click = Some((window_id, now));
    }

    // ダブルクリックでなければ、ここで処理を終了
    if !is_double_click {
      return;
    }

    // ダブルクリックの場合、クリックされた位置のアイコンを探す
    // 記録されている最後のカーソル位置を取得
    if let Some(cursor_pos) = self.last_cursor_pos.get(&window_id).cloned() {
      // カーソル位置にあるアイコンのインデックスを検索
      if let Some((_icon_win_id, icon_index)) = self.find_icon_at_relative_pos(window_id, cursor_pos) {
        // アイコンが見つかった場合、対応する子ウィンドウを取得
        if let Some(child) = self.children.get(&window_id) {
           // インデックスが有効範囲内か確認 (念のため)
           if icon_index < child.groups.len() {
               // IconInfo の execute メソッドを呼び出してファイル/フォルダを開く
               child.groups[icon_index].execute();
           } else {
               // 無効なインデックスの場合 (通常は起こらないはず)
               log_error(&format!("無効なインデックス {} でグループアイテムを実行しようとしました (グループ数: {})", icon_index, child.groups.len()));
           }
        }
      }
    }
  }

  /// マウスの右クリックイベント (Ctrlキー同時押し) を処理します。
  pub fn remove_group_item(&mut self, window_id: WindowId) {
    // Ctrlキーが押されているか確認 (is_moving.keybord_pressed で代用)
    if !self.is_moving.keybord_pressed {
      return; // Ctrl が押されていなければ何もしない
    }

    // クリックされた位置 (記録されている最後のカーソル位置) を取得
    if let Some(cursor_pos) = self.last_cursor_pos.get(&window_id).cloned() {
      // カーソル位置にあるアイコンのインデックスを検索
      match self.find_icon_at_relative_pos(window_id, cursor_pos) {
        // --- アイコンが見つかった場合 (既存の処理) ---
        Some((_icon_win_id, icon_index)) => {
          log_debug(&format!("Ctrl+RightClick on icon index {} in window {:?}. Removing item.", icon_index, window_id));
          // 既存のアイテム削除処理を呼び出す
          self.remove_item(window_id, icon_index);
        }
        // --- アイコンが見つからなかった場合 (新しい処理) ---
        None => {
          log_debug(&format!("Ctrl+RightClick on empty space in window {:?}. Requesting window removal.", window_id));
          // ウィンドウ削除要求処理を呼び出す
          self.request_remove_window(window_id);
        }
      }
    }
  }

  /// アイコンが右クリックされたときに、そのアイコンのファイルの場所をエクスプローラーで開くよ！
  /// Ctrlキーが押されて *いない* 右クリックのときに呼ばれるんだ♪
  pub fn open_icon_location(&mut self, window_id: WindowId) {
      // まず、どこをクリックしたか思い出すよ (最後に記録したカーソル位置！)
      if let Some(cursor_pos) = self.last_cursor_pos.get(&window_id).cloned() {
          // その場所にアイコンがあるか探してみるね！ (find_icon_at_relative_pos におまかせ！)
          if let Some((_icon_win_id, icon_index)) = self.find_icon_at_relative_pos(window_id, cursor_pos) {
              // やったー！アイコン見っけ！ (ログにも記録しとこっと)
              log_debug(&format!("RightClick on icon index {} in window {:?}. Opening location.", icon_index, window_id));
              // そのアイコンの情報 (IconInfo) を取り出すよ！
              if let Some(child) = self.children.get(&window_id) {
                  if icon_index < child.groups.len() { // ちゃんとリストにあるインデックスかな？
                      let icon_info = &child.groups[icon_index];
                      // アイコンのパス (例: C:\Users\Me\Desktop\すごいファイル.txt) から、
                      // そのファイルがいるフォルダ (例: C:\Users\Me\Desktop) を見つけるよ！
                      if let Some(parent_dir) = icon_info.path.parent() {
                          log_info(&format!("Opening location for {:?}: {:?}", icon_info.path, parent_dir));
                          // 見つけたフォルダをエクスプローラーでオープン！ パソコンの中を探検だー！٩(ˊᗜˋ*)و
                          match open::that(parent_dir) {
                              Ok(_) => log_info(&format!("Successfully opened directory: {:?}", parent_dir)),
                              Err(e) => log_error(&format!("Failed to open directory {:?}: {}", parent_dir, e)), // あれれ？開けなかった…(´・ω・`)
                          }
                      } else {
                          // もし親フォルダが見つからなかったら (例: C:\ ドライブ自体とか？)、
                          // そのパス自体を開いてみる！ ちょっとレアケースかも？
                          log_warn(&format!("Could not get parent directory for path: {:?}. Attempting to open the path itself.", icon_info.path));
                           match open::that(&icon_info.path) {
                              Ok(_) => log_info(&format!("Successfully opened path: {:?}", icon_info.path)),
                              Err(e) => log_error(&format!("Failed to open path {:?}: {}", icon_info.path, e)), // うーん、やっぱりダメだったか…
                          }
                      }
                  } // icon_index < child.groups.len() の終わり
              } // child が見つかった場合の終わり
          } // アイコンが見つかった場合の終わり
          // アイコンじゃない場所を右クリックしたときは、何もしないよ！ (ウィンドウ削除はCtrl+右クリックだけ！)
      } // カーソル位置が取れた場合の終わり
  }

  /// ウィンドウ削除の要求を受け付け、確認ダイアログを表示するメソッド。
  fn request_remove_window(&mut self, window_id: WindowId) {
    // 確認ダイアログを表示し、ユーザーが「はい」を押した場合のみ削除処理を実行
    if show_confirmation_dialog() {
      log_info(&format!("User confirmed removal for window {:?}. Proceeding.", window_id));
      self.remove_window(window_id);
    } else {
      log_info(&format!("User cancelled removal for window {:?}.", window_id));
    }
  }

  /// 指定されたウィンドウIDに対応する子ウィンドウと関連データを削除するメソッド。
  /// 設定ファイルからも該当エントリを削除します。
  fn remove_window(&mut self, window_id: WindowId) {
    // 1. 設定ファイルから削除するために id_str を取得
    let id_str_to_remove = if let Some(child) = self.children.get(&window_id) {
      child.id_str.clone() // 後で使うためにクローンしておく
    } else {
      log_error(&format!("Cannot remove window {:?}: ChildWindow not found before removal.", window_id));
      return; // 削除対象が見つからなければ終了
    };

    // 2. WindowManager の管理下から ChildWindow を削除
    if let Some(removed_child) = self.children.remove(&window_id) {
      log_info(&format!("Removed ChildWindow (id_str: {}) from manager.", removed_child.id_str));
      // removed_child がドロップされることで、Rc<Window> の参照カウントが減る。
      // 参照が他になければ、Window もドロップされ、OSレベルで閉じられるはず。
    } else {
      // remove_group_item で存在確認しているので、通常ここには来ないはず
      log_error(&format!("Cannot remove window {:?}: ChildWindow not found during removal.", window_id));
      // return; // 続行しても良いかもしれない
    }

    // 3. 関連する状態をクリーンアップ
    self.last_cursor_pos.remove(&window_id);
    if self.focused_id == Some(window_id) {
      self.focused_id = None;
    }
    if let Some((hover_id, _)) = self.hovered_icon {
      if hover_id == window_id {
        self.hovered_icon = None;
      }
    }
    if let Some((click_id, _)) = self.last_click {
      if click_id == window_id {
        self.last_click = None;
      }
    }

    // 4. グローバル設定から該当するウィンドウの設定を削除
    { // 書き込みロックのスコープ
      let mut settings = get_settings_writer();
      if settings.children.remove(&id_str_to_remove).is_some() {
        log_info(&format!("Removed settings entry for id_str: {}", id_str_to_remove));
      } else {
        log_warn(&format!("Settings entry for id_str {} not found during removal.", id_str_to_remove));
      }
    } // 書き込みロック解放

    // --- ★設定保存処理を追加 (ウィンドウ削除後) ---
    log_info(&format!("Window {:?} and its data removed successfully.", window_id));
    save_settings(); // 設定ファイルに即時保存
  }

  /// 指定されたウィンドウの、指定されたインデックスにあるアイコンを削除します。
  fn remove_item(&mut self, window_id: WindowId, index: usize) {
    // 対象の子ウィンドウ (可変参照) を取得
    if let Some(child) = self.children.get_mut(&window_id) {
       // インデックスが有効範囲内か確認
       if index < child.groups.len() {
          child.groups.remove(index); // ベクターからアイテムを削除
          self.hovered_icon = None; // ホバー状態をリセット
          self.last_click = None; // ダブルクリック状態をリセット
          // カーソル位置情報は他の操作で必要になる可能性があるので、ここでは削除しない
          // self.last_cursor_pos.remove(&window_id);
          child.window.request_redraw(); // アイテム削除後に再描画を要求
          // --- ★設定保存処理を追加 ---
          self.save_child_settings(window_id);
       } else {
          // 無効なインデックスの場合 (通常は起こらないはず)
          log_error(&format!("無効なインデックス {} でグループアイテムを削除しようとしました (グループ数: {})", index, child.groups.len()));
       }
    }
  }

  /// 指定されたウィンドウ内の特定の物理座標 (`PhysicalPosition`) に
  /// どのアイコンが存在するかを判定します。
  pub fn find_icon_at_relative_pos(
    &self,
    window_id: WindowId,
    cursor_pos_rel: PhysicalPosition<f64>,
  ) -> Option<(WindowId, usize)> {
    // 対象の子ウィンドウを取得
    if let Some(child_window) = self.children.get(&window_id) {
      // カーソル座標を f64 から f32 に変換 (描画座標系に合わせる)
      let cursor_x = cursor_pos_rel.x as f32;
      let cursor_y = cursor_pos_rel.y as f32;

      // ウィンドウ内のすべてのアイコンについてループ
      for index in 0..child_window.groups.len() {
        // 各アイコンの描画矩形を取得 (MyGraphics に実装されている想定)
        if let Some(item_rect) = child_window.graphics.get_item_rect_f32(index) {
          // カーソル座標がアイコンの矩形内にあるか判定
          if cursor_x >= item_rect.x() &&
            cursor_x < item_rect.x() + item_rect.width() &&
            cursor_y >= item_rect.y() &&
            cursor_y < item_rect.y() + item_rect.height()
          {
            // 矩形内にあれば、そのウィンドウIDとアイコンインデックスを返す
            return Some((window_id, index));
          }
        }
      }
    }
    // どのアイコンの矩形内にもカーソルがなければ None を返す
    None
  }

  /// マウスカーソルのホバー状態を更新し、必要に応じて再描画を要求します。
  /// カーソルが新しいアイコン上に移動した、またはアイコンから離れた場合に呼び出されます。
  pub fn update_hover_state(&mut self, new_hover: Option<(WindowId, usize)>) {
    let old_hover = self.hovered_icon; // 更新前のホバー状態を保持
    // ホバー状態が変化していなければ何もしない
    if old_hover == new_hover {
      return;
    }
    // ホバー状態を更新
    self.hovered_icon = new_hover;

    // --- 再描画要求 ---
    // 以前ホバーされていたアイコンが存在する場合、そのウィンドウを再描画
    // (ホバー解除による表示更新のため)
    if let Some((old_id, _)) = old_hover {
      if let Some(child) = self.children.get(&old_id) {
        child.window.request_redraw();
      }
    }
    // 新しくホバーされたアイコンが存在する場合、そのウィンドウを再描画
    // (ホバー強調表示のため)
    if let Some((new_id, _)) = new_hover {
      // ただし、以前ホバーされていたウィンドウと同じ場合は再描画要求を重複させない
      if old_hover.map_or(true, |(old_id, _)| old_id != new_id) {
        if let Some(child) = self.children.get(&new_id) {
          child.window.request_redraw();
        }
      }
    }
  }

}
