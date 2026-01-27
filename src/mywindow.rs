use std::{
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

use arboard::Clipboard;
use desktop_grouping::win32::ui_wam;
use rand::Rng; // ★ rand を use するよ！
use tiny_skia::Color;
use winit::{
    dpi::PhysicalPosition,
    window::{ResizeDirection, Window, WindowId},
};

use crate::{
    child_window::*, file_drag::IconInfo, logger::*, settings::*,
    window_utils::show_confirmation_dialog,
};

// ★ color_to_hex_string を use するよ！
use crate::child_window::color_to_hex_string;

/// ダブルクリックと判定する時間閾値 (ミリ秒)
const DOUBLE_CLICK_THRESHOLD_MS: u64 = 500;

/// アイコン実行時エフェクトの表示時間 (ミリ秒)
const EXECUTION_EFFECT_DURATION_MS: u64 = 200;


/// アプリケーション内で発生するカスタムイベント。
/// トレイメニューからのイベントと、設定読み込み完了イベントがあるよ！
#[derive(Debug)]
pub enum UserEvent {
    MenuEvent(tray_icon::menu::MenuEvent),
    SettingsLoaded(HashMap<String, ChildSettings>),
}

/// ウィンドウ全体を管理する構造体。
/// たくさんの子ウィンドウちゃんたち (`children`) を覚えておいたり、
/// 今どの子ウィンドウちゃんに注目してるか (`focused_id`)、
/// マウスでぐりぐり動かしたり大きさを変えたりする時の状態 (`is_moving`, `is_resizing`) を管理してるんだ！
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
    /// 現在実行エフェクトを表示中のアイコンの情報。(ウィンドウID, アイコンインデックス, 開始時刻)
    executing_icon: Option<(WindowId, usize, Instant)>,
    /// ダブルクリック判定用の最後にクリックされた時刻とウィンドウID。
    last_click: Option<(WindowId, Instant)>,
    /// 各ウィンドウちゃんの中で、最後にマウスカーソルがいた場所を覚えておくよ！アイコンの場所を特定するのに使うんだ。
    last_cursor_pos: HashMap<WindowId, PhysicalPosition<f64>>,
    /// クリップボードにアクセスするためのものだよ！コピペ機能で使うんだ♪
    clipboard: Option<Clipboard>,
    /// 最後にマウスカーソルがいたウィンドウのIDだよ！マウスホイールで透明度を変える時とかに使うんだ。
    last_cursor_window_id: Option<WindowId>,
    /// 設定が変更されたかどうかを示すフラグ。true の場合、アイドル時にファイルに保存されるよ！
    pub settings_are_dirty: bool,
}

/// ウィンドウ操作（移動/リサイズ）の状態を管理する構造体。
/// ウィンドウを動かす時 (Ctrlキー) や、大きさを変える時 (Shiftキー) に、
/// 対応するキーとマウスの左ボタンが押されてるかどうかを覚えておくためのものだよ！
#[derive(Debug)]
pub struct WindowControl {
    /// 対応するキーボードのキーが押されているか。
    pub keybord_pressed: bool,
    /// マウスの左ボタンが押されているか。
    pub mouse_pressed: bool,
}

impl WindowControl {
    /// 新しい `WindowControl` インスタンスを作るよ！
    /// 最初は、キーもマウスも押されてない (`false`) 状態からスタートだね！
    pub fn new() -> WindowControl {
        return WindowControl {
            keybord_pressed: false,
            mouse_pressed: false,
        };
    }

    /// 対応するキーとマウス左ボタンの両方が押されているかを判定します。
    /// これが `true` の時だけ、ウィンドウを動かしたり大きさを変えたりできるんだ！
    /// まさに「操作可能！」ってことだね！(๑•̀ㅂ•́)و✧
    pub fn can_control(&self) -> bool {
        return self.keybord_pressed && self.mouse_pressed;
    }
}

/// ランダムな色を生成し、#RRGGBBAA 形式の16進数文字列で返すよ！
pub fn generate_random_color_hex() -> String {
    let mut rng = rand::thread_rng();
    let r: u8 = rng.r#gen();
    let g: u8 = rng.r#gen();
    let b: u8 = rng.r#gen();
    // アルファ値はデフォルトの半透明 (0x99 = 153) にする
    let random_color = Color::from_rgba8(r, g, b, 153);
    color_to_hex_string(random_color)
}

impl WindowManager {
    /// 新しい `WindowManager` インスタンスを作成します。
    /// たくさんの子ウィンドウちゃんたちを管理するための準備をするよ！
    /// クリップボードもここで初期化するんだ。
    pub fn new(clipboard: Option<Clipboard>) -> WindowManager {
        return WindowManager {
            children: HashMap::new(),          // 子ウィンドウマップを空で初期化
            focused_id: None,                  // 最初はフォーカスされているウィンドウはない
            is_moving: WindowControl::new(),   // 移動状態を初期化
            is_resizing: WindowControl::new(), // リサイズ状態を初期化
            hovered_icon: None,                // 最初はホバーされているアイコンはない
            executing_icon: None,              // 実行エフェクトも最初はなし
            last_click: None,                  // ダブルクリック判定情報を初期化
            last_cursor_pos: HashMap::new(),   // カーソル位置マップを空で初期化
            clipboard,
            last_cursor_window_id: None,
            settings_are_dirty: false, // ダーティフラグを初期化
        };
    }

    /// アイドル時や終了時に、ダーティフラグが立っている場合のみ設定をファイルに保存します。
    pub fn save_settings_if_dirty(&mut self) {
        if self.settings_are_dirty {
            log_info("Settings are dirty, saving to file...");
            save_settings(); // settings.rs のグローバル関数を呼び出す
            self.settings_are_dirty = false; // 保存したのでフラグをリセット
        }
    }

    /// 指定された `WindowId` が管理対象の子ウィンドウに存在するかどうかを確認します。
    /// 「ねぇねぇ、このIDの子ウィンドウちゃん、知ってる？」って聞く感じだね！
    pub fn has_window(&self, id: &WindowId) -> bool {
        self.children.contains_key(id)
    }

    /// 指定された `WindowId` に対応する `winit::window::Window` への参照を取得します。
    /// IDを渡すと、その子の `Window` オブジェクトをそっと教えてくれるよ。再描画をお願いする時とかに使うんだ。
    pub fn get_window_ref(&self, id: &WindowId) -> Option<&Window> {
        // self.children から ChildWindow を取得し、その中の window フィールドへの参照を返す
        self.children.get(id).map(|cw| &*cw.window) // Rc<Window> から &Window を取得
    }

    /// 指定された `WindowId` に対応する `ChildWindow` への可変参照を取得します。
    pub fn get_child_window_mut(&mut self, id: &WindowId) -> Option<&mut ChildWindow> {
        self.children.get_mut(id)
    }

    /// 管理対象の子ウィンドウを追加します。
    /// 新しい子ウィンドウちゃんが仲間入りする時に呼ばれるよ！
    /// `Window` オブジェクトと、その子の名前 (id_str)、それから初期設定をもらって、
    /// `ChildWindow` インスタンスを作って `children` マップに追加するんだ。
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
            ChildWindow::new(window, id_str, &settings.bg_color),
        );
    }

    /// Ctrl+V ペーストイベントを処理します。
    /// クリップボードにコピーされた文字を見て、それが色コードだったら背景色を変えたり、
    /// `#Random` って書いてあったらランダムな色にしちゃったりするよ！楽しいね！(<em>´艸｀</em>)
    pub fn handle_paste(&mut self, window_id: WindowId) {
        if let Some(clipboard) = &mut self.clipboard {
            match clipboard.get_text() {
                Ok(text) => {
                    log_debug(&format!("Clipboard text received: {}", text));
                    let mut settings_changed = false;
                    // 対応する ChildWindow のメソッドを呼び出す
                    if let Some(child) = self.children.get_mut(&window_id) {
                        let trimmed_text = text.trim(); // 前後の空白を除去
                        if trimmed_text.eq_ignore_ascii_case("#Random") {
                            // "#Random" がペーストされた場合
                            log_info(&format!(
                                "Window {}: Received #Random command. Generating random color.",
                                child.id_str
                            ));
                            let color_str = generate_random_color_hex();
                            log_debug(&format!(
                                "Window {}: Generated random color: {}",
                                child.id_str, color_str
                            ));
                            child.set_background_color(&color_str);
                            settings_changed = true;
                        } else {
                            // 通常の色コードがペーストされた場合 (既存の処理)
                            child.set_background_color(trimmed_text);
                            settings_changed = true;
                        }
                    }
                    // 変更があった場合のみ、メモリ上の設定を更新してダーティフラグを立てる
                    if settings_changed {
                        self.update_child_settings_in_memory(window_id);
                        self.settings_are_dirty = true;
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
    /// 最後にマウスカーソルがいたウィンドウの透明度を、くるくる～って変えるんだ♪
    /// 透明度が実際に変わった時だけ、設定を保存するようになってるよ。エコだね！
    pub fn handle_mouse_wheel(&mut self, delta_y: f32) {
        // 最後にカーソルがあったウィンドウIDを使用
        if let Some(window_id) = self.last_cursor_window_id {
            let mut settings_changed = false;
            if let Some(child) = self.children.get_mut(&window_id) {
                // delta_y の符号で方向を判断 (正が上、負が下など、環境依存確認)
                // ここでは delta_y が正なら増加、負なら減少と仮定
                let old_alpha = child.graphics.get_background_color().alpha(); // 保存前に現在のアルファ値を取得
                child.adjust_alpha(delta_y);
                // 実際にアルファ値が変わった場合のみ設定を保存
                if (child.graphics.get_background_color().alpha() - old_alpha).abs() > f32::EPSILON
                {
                    settings_changed = true;
                }
            }
            // 変更があった場合のみ、メモリ上の設定を更新してダーティフラグを立てる
            if settings_changed {
                self.update_child_settings_in_memory(window_id);
                self.settings_are_dirty = true;
            }
        }
    }

    /// 最後にカーソルがあったウィンドウIDを記録します。
    /// マウスホイールイベントの時に、どこのウィンドウの透明度を変えるか判断するのに使うよ！
    pub fn set_last_cursor_window(&mut self, window_id: Option<WindowId>) {
        self.last_cursor_window_id = window_id;
    }

    /// 指定された子ウィンドウの現在の状態（位置、サイズ、色、アイコン）を
    /// グローバル設定に反映します。（ファイル保存は行わない）
    /// この関数は高速に実行されるべきで、ディスクI/Oは含まないよ！
    pub fn update_child_settings_in_memory(&mut self, window_id: WindowId) {
        // 対象の子ウィンドウを取得
        let child_window = match self.children.get(&window_id) {
            Some(cw) => cw,
            None => {
                log_error(&format!(
                    "設定更新対象のウィンドウが見つかりません (ID: {:?})",
                    window_id
                ));
                return;
            }
        };

        let id_str = child_window.id_str.clone(); // 設定キーとして使うID

        {
            // 設定書き込みロックのスコープ
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
                        // ウィンドウの左上座標 (pos) がどのモニターに属するかを判定するよ！
                        let mut belonging_monitor = None;
                        for monitor in child_window.window.available_monitors() {
                            let monitor_pos = monitor.position();
                            let monitor_size = monitor.size();
                            let monitor_right = monitor_pos.x + monitor_size.width as i32;
                            let monitor_bottom = monitor_pos.y + monitor_size.height as i32;

                            // ウィンドウの左上 (pos.x, pos.y) がこのモニターの範囲内にあるかチェック！
                            if pos.x >= monitor_pos.x && pos.x < monitor_right &&
                               pos.y >= monitor_pos.y && pos.y < monitor_bottom {
                                belonging_monitor = Some(monitor);
                                break; // 見つかったからループを抜けるよ！
                            }
                        }

                        // 見つかったモニターの情報を保存するよ！
                        if let Some(monitor) = belonging_monitor {
                            child_settings.monitor_name = monitor.name(); // モニターの名前をゲット！
                            let monitor_pos = monitor.position(); // モニター自体の仮想座標
                            child_settings.monitor_x = Some(pos.x - monitor_pos.x); // モニター内での相対X座標！
                            child_settings.monitor_y = Some(pos.y - monitor_pos.y); // モニター内での相対Y座標！
                        } else {
                            // どのモニターにも属してない！？ ちょっと珍しいケースだけど、情報はクリアしとこっと。
                            child_settings.monitor_name = None;
                            child_settings.monitor_x = None;
                            child_settings.monitor_y = None;
                        }
                    }
                    Err(e) => {
                        log_error(&format!(
                            "ウィンドウの位置取得に失敗 (id_str: {}): {}",
                            id_str, e
                        ));
                    }
                }
                let size = child_window.window.inner_size();
                child_settings.width = size.width;
                child_settings.height = size.height;

                // --- 色情報の保存 ---
                child_settings.bg_color =
                    color_to_hex_string(child_window.graphics.get_background_color());

                // --- アイコン情報の保存 ---
                child_settings.icons = child_window
                    .groups
                    .iter()
                    .map(|icon_info| PersistentIconInfo {
                        path: icon_info.path.clone(),
                    })
                    .collect();
                log_debug(&format!("Updated settings in memory for window {}", id_str));
            } else {
                log_error(&format!("設定更新時にエントリが見つかりません (id_str: {})", id_str));
            }
        }
        // --- ここではファイルに保存しない！ ---
    }

    /// 設定から読み込んだアイコンパス情報に基づいて、指定されたウィンドウにアイコンを復元します。
    /// アプリを起動した時に、前に保存したアイコンたちをウィンドウに戻してあげるお仕事だよ！
    /// もしアイコンの復元で何かあっても、`catch_unwind` でアプリ全体が困っちゃわないようにしてるんだ。えらい！
    pub fn restore_icons(&mut self, window_id: &WindowId, persistent_icons: &[PersistentIconInfo]) {
        // 対象の子ウィンドウ (可変参照) を取得
        if let Some(child) = self.children.get_mut(window_id) {
            log_info(&format!(
                "Restoring {} icons for window {}",
                persistent_icons.len(),
                child.id_str
            ));
            // 永続化されていた各アイコンパスについてループ
            for p_icon in persistent_icons {
                // パス情報を使って IconInfo::new を呼び出し、アイコン情報を再生成
                // 注意: IconInfo::new は失敗する可能性がある (expectを使っている場合パニック)
                //       より堅牢にするには Result を返すように IconInfo::new を変更し、
                //       ここでエラーハンドリング (ログ出力など) を行うのが望ましい。
                //       今回は既存コードに合わせて expect を使うか、ログ出力に留める。
                log_debug(&format!(
                    "Attempting to restore icon from path: {:?}",
                    p_icon.path
                ));

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
            log_error(&format!(
                "アイコン復元対象のウィンドウが見つかりません (ID: {:?})",
                window_id
            ));
        }
    }

    /// ウィンドウ移動操作のためのキーボード状態を設定します (通常はCtrlキー)。
    /// Ctrlキーが押されたり離されたりした時に呼ばれて、`is_moving.keybord_pressed` の状態を更新するよ。
    /// キーが離されたら、フォーカスも解除するのを忘れないようにね！
    pub fn set_moving_keybord_state(&mut self, state: bool) {
        self.is_moving.keybord_pressed = state;
        // キーが離されたら、フォーカスも解除する
        if !state {
            self.focused_id = None;
        }
    }

    /// ウィンドウリサイズ操作のためのキーボード状態を設定します (通常はShiftキー)。
    /// Shiftキーが押されたり離されたりした時に呼ばれて、`is_resizing.keybord_pressed` の状態を更新するよ。
    /// こっちも、キーが離されたらフォーカスを解除するのを忘れずに！
    pub fn set_resizing_keybord_state(&mut self, state: bool) {
        self.is_resizing.keybord_pressed = state;
        // キーが離されたら、フォーカスも解除する
        if !state {
            self.focused_id = None;
        }
    }

    /// ウィンドウのドラッグ移動操作を開始します。
    /// 移動キー (Ctrl) とマウス左ボタンが両方押されており、
    /// かつフォーカスされているウィンドウがある場合に、その子ウィンドウちゃんに「動いてー！」ってお願いするよ。
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
                log_error(&format!(
                    "Drag target child window not found for focused_id: {:?}",
                    focused_id
                ));
            }
        }
    }

    /// ウィンドウのリサイズ操作を開始します。
    /// リサイズキー (Shift) とマウス左ボタンが両方押されてて、
    /// フォーカスされてるウィンドウがあったら、その子に「大きくなってー！」 (または「小さくなってー！」) ってお願いするよ。
    /// 今は右下方向にしかリサイズできないけどね！(・ω<)
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
                log_error(&format!(
                    "Resize target child window not found for focused_id: {:?}",
                    focused_id
                ));
            }
        }
    }

    /// 指定されたIDのウィンドウをデスクトップの最背面 (他のウィンドウの後ろ) に移動します。
    /// 「ちょっと奥に行っててね～」って感じで、ウィンドウを一番後ろに隠すんだ。
    pub fn backmost(&mut self, id: &WindowId) {
        // 対象の子ウィンドウを取得
        let child = self
            .children
            .get(id)
            .expect("最背面移動対象の子ウィンドウ取得に失敗");

        // win32 API を呼び出してウィンドウを最背面に移動
        ui_wam::set_window_pos_to_bottom(&child.window);
    }

    /// 指定されたIDのウィンドウの内容を描画します。
    /// その子ウィンドウちゃんに「お絵かきお願いね！」って伝えて、
    /// マウスカーソルがアイコンの上にあったら、それも教えてあげるんだ。
    pub fn draw_window(&mut self, id: &WindowId) {
        if self.children.is_empty() {
            return;
        }

        // 実行中エフェクトが時間切れになっていたら、状態をリセットして再描画を要求するよ！
        let mut needs_redraw_after_effect = false;
        if let Some((win_id, _, start_time)) = self.executing_icon {
            if win_id == *id
                && start_time.elapsed() > Duration::from_millis(EXECUTION_EFFECT_DURATION_MS)
            {
                self.executing_icon = None;
                needs_redraw_after_effect = true;
            }
        }

        // 実行中エフェクトの対象アイコンインデックスを取得
        let executing_index =
            self.executing_icon.and_then(
                |(exec_id, exec_idx, _)| {
                    if exec_id == *id { Some(exec_idx) } else { None }
                },
            );

        // 描画対象の子ウィンドウ（可変参照）を取得
        let child = self
            .children
            .get_mut(id)
            .expect("描画対象の子ウィンドウ取得に失敗");

        // このウィンドウ上でホバーされているアイコンのインデックスを取得するよ
        let hovered_index = self.hovered_icon.and_then(|(hover_id, hover_idx)| {
            // hovered_icon のウィンドウIDが、描画対象のウィンドウIDと一致する場合のみ Some(インデックス) を返す
            if hover_id == *id {
                Some(hover_idx)
            } else {
                None
            }
        });

        // ChildWindow の draw メソッドを呼び出して、ホバーと実行中の情報を渡すんだ
        child.draw(hovered_index, executing_index);

        // エフェクトが終わった直後なら、消えた状態を反映するために再描画をお願いするよ
        if needs_redraw_after_effect {
            child.window.request_redraw();
        }
    }

    /// 指定されたIDのウィンドウのサイズが変更されたときに呼び出されます。
    /// ウィンドウの大きさが変わったら、その子ウィンドウちゃんに「サイズ変わったよー！」って教えて、
    /// グラフィックスの準備をし直してもらってから、再描画をお願いするんだ。
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
    /// ファイルがウィンドウにドラッグ＆ドロップされた時とかに呼ばれるよ！
    /// 新しいアイコンを仲間入りさせて、設定もちゃんと保存するんだ。
    pub fn add_group(&mut self, icon: IconInfo) {
        // 子ウィンドウがない、またはフォーカスされているウィンドウがない場合は何もしない
        if self.children.is_empty() || self.focused_id.is_none() {
            return;
        }
        // フォーカスされている子ウィンドウ (可変参照) を取得
        let focused_id = self.focused_id.unwrap();
        let child = self
            .children
            .get_mut(&focused_id)
            .expect("アイコン追加対象の子ウィンドウ取得に失敗");
        // ChildWindow の add メソッドを呼び出す
        child.add(icon);
        // アイコン追加後に再描画を要求 (任意だが推奨)
        child.window.request_redraw();
        // --- ★設定保存処理を更新 ---
        self.update_child_settings_in_memory(focused_id);
        self.settings_are_dirty = true;
    }

    /// 指定されたウィンドウにおけるマウスカーソルの最新位置を記録します。
    /// マウスが動くたびに、どこのウィンドウのどのへんにいるか覚えておくんだ。
    /// アイコンをクリックしたかとか、ホバーしてるかとかを判断するのに使うよ！
    pub fn update_cursor_pos(&mut self, window_id: WindowId, position: PhysicalPosition<f64>) {
        self.last_cursor_pos.insert(window_id, position);
    }

    /// マウスの左クリックイベントを処理します。
    /// ダブルクリックされたかどうかをチェックして、もしダブルクリックだったら、その場所にあるアイコンを実行するよ！
    pub fn execute_group_item(&mut self, window_id: WindowId) {
        let now = Instant::now(); // 現在時刻を取得
        let is_double_click; // ダブルクリックフラグ

        // 前回のクリック情報を確認
        if let Some((last_id, last_time)) = self.last_click {
            // 前回と同じウィンドウIDで、かつ閾値時間内にクリックされたか？
            if last_id == window_id
                && now.duration_since(last_time) < Duration::from_millis(DOUBLE_CLICK_THRESHOLD_MS)
            {
                // ダブルクリックと判定
                is_double_click = true;
                self.last_click = None; // ダブルクリックが成立したのでリセット
            } else {
                // シングルクリック (または閾値超過) なので、今回のクリック情報を保存
                is_double_click = false;
                self.last_click = Some((window_id, now));
            }
        } else {
            // 初めてのクリックなので、今回のクリック情報を保存
            is_double_click = false;
            self.last_click = Some((window_id, now));
        }

        // ダブルクリックでなければ、ここで処理を終了
        if !is_double_click {
            return;
        }

        // ダブルクリックの場合のみ、ここから先の処理に進むよ！
        if let Some(cursor_pos) = self.last_cursor_pos.get(&window_id).cloned() {
            // カーソル位置にあるアイコンのインデックスを検索
            if let Some((_icon_win_id, icon_index)) =
                self.find_icon_at_relative_pos(window_id, cursor_pos)
            {
                // アイコンが見つかった場合、対応する子ウィンドウを取得
                if let Some(child) = self.children.get(&window_id) {
                    // インデックスが有効範囲内か確認 (念のため)
                    if icon_index < child.groups.len() {
                        // エフェクト開始！実行中のアイコンとして記録して、再描画をお願いするよ！
                        self.executing_icon = Some((window_id, icon_index, Instant::now()));
                        child.window.request_redraw();

                        // IconInfo の execute メソッドを呼び出してファイル/フォルダを開く
                        child.groups[icon_index].execute();
                    } else {
                        // 無効なインデックスの場合 (通常は起こらないはず)
                        log_error(&format!(
                            "無効なインデックス {} でグループアイテムを実行しようとしました (グループ数: {})",
                            icon_index,
                            child.groups.len()
                        ));
                    }
                }
            }
        }
    }

    /// マウスの右クリックイベント (Ctrlキー同時押し) を処理します。
    /// Ctrlキーを押しながら右クリックすると、その場所にあるアイコンを削除したり、
    /// 何もないところだったらウィンドウ自体を削除するかどうか聞いたりするよ！
    /// ちょっと危険な操作だから、気をつけてね！＞＜
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
                    log_debug(&format!(
                        "Ctrl+RightClick on icon index {} in window {:?}. Removing item.",
                        icon_index, window_id
                    ));
                    // 既存のアイテム削除処理を呼び出す
                    self.remove_item(window_id, icon_index);
                }
                // --- アイコンが見つからなかった場合 (新しい処理) ---
                None => {
                    log_debug(&format!(
                        "Ctrl+RightClick on empty space in window {:?}. Requesting window removal.",
                        window_id
                    ));
                    // ウィンドウ削除要求処理を呼び出す
                    self.request_remove_window(window_id);
                }
            }
        }
    }

    /// アイコンが右クリックされたときに、そのアイコンのファイルの場所をエクスプローラーで開くよ！
    /// Ctrlキーが押されて *いない* 右クリックのときに呼ばれるんだ♪
    /// 「このアイコン、どこのファイルだっけ～？」って時に便利だね！
    /// エクスプローラーで、そのファイルがあるフォルダを開いてくれるよ！
    pub fn open_icon_location(&mut self, window_id: WindowId) {
        // まず、どこをクリックしたか思い出すよ (最後に記録したカーソル位置！)
        if let Some(cursor_pos) = self.last_cursor_pos.get(&window_id).cloned() {
            // その場所にアイコンがあるか探してみるね！ (find_icon_at_relative_pos におまかせ！)
            if let Some((_icon_win_id, icon_index)) =
                self.find_icon_at_relative_pos(window_id, cursor_pos)
            {
                // やったー！アイコン見っけ！ (ログにも記録しとこっと)
                log_debug(&format!(
                    "RightClick on icon index {} in window {:?}. Opening location.",
                    icon_index, window_id
                ));
                // そのアイコンの情報 (IconInfo) を取り出すよ！
                if let Some(child) = self.children.get(&window_id) {
                    if icon_index < child.groups.len() {
                        // ちゃんとリストにあるインデックスかな？
                        let icon_info = &child.groups[icon_index];
                        // アイコンのパス (例: C:\Users\Me\Desktop\すごいファイル.txt) から、
                        // そのファイルがいるフォルダ (例: C:\Users\Me\Desktop) を見つけるよ！
                        if let Some(parent_dir) = icon_info.path.parent() {
                            log_info(&format!(
                                "Opening location for {:?}: {:?}",
                                icon_info.path, parent_dir
                            ));
                            // 見つけたフォルダをエクスプローラーでオープン！ パソコンの中を探検だー！٩(ˊᗜˋ*)و
                            match open::that(parent_dir) {
                                Ok(_) => log_info(&format!(
                                    "Successfully opened directory: {:?}",
                                    parent_dir
                                )),
                                Err(e) => log_error(&format!(
                                    "Failed to open directory {:?}: {}",
                                    parent_dir, e
                                )), // あれれ？開けなかった…(´・ω・`)
                            }
                        } else {
                            // もし親フォルダが見つからなかったら (例: C:\ ドライブ自体とか？)、
                            // そのパス自体を開いてみる！ ちょっとレアケースかも？
                            log_warn(&format!(
                                "Could not get parent directory for path: {:?}. Attempting to open the path itself.",
                                icon_info.path
                            ));
                            match open::that(&icon_info.path) {
                                Ok(_) => log_info(&format!(
                                    "Successfully opened path: {:?}",
                                    icon_info.path
                                )),
                                Err(e) => log_error(&format!(
                                    "Failed to open path {:?}: {}",
                                    icon_info.path, e
                                )), // うーん、やっぱりダメだったか…
                            }
                        }
                    } // icon_index < child.groups.len() の終わり
                } // child が見つかった場合の終わり
            } // アイコンが見つかった場合の終わり
            // アイコンじゃない場所を右クリックしたときは、何もしないよ！ (ウィンドウ削除はCtrl+右クリックだけ！)
        } // カーソル位置が取れた場合の終わり
    }

    /// ウィンドウ削除の要求を受け付け、確認ダイアログを表示するメソッド。
    /// `remove_group_item` から呼ばれて、本当にウィンドウを消しちゃっていいかユーザーさんに聞くんだ。
    /// 「はい」って言われたら、`remove_window` を呼び出して、さよならバイバイするよ…(´；ω；｀)
    fn request_remove_window(&mut self, window_id: WindowId) {
        // 確認ダイアログを表示し、ユーザーが「はい」を押した場合のみ削除処理を実行
        if show_confirmation_dialog() {
            log_info(&format!(
                "User confirmed removal for window {:?}. Proceeding.",
                window_id
            ));
            self.remove_window(window_id);
        } else {
            log_info(&format!(
                "User cancelled removal for window {:?}.",
                window_id
            ));
        }
    }

    /// 指定されたウィンドウIDに対応する子ウィンドウと関連データを削除するメソッド。
    /// ウィンドウを本当に消しちゃう処理だよ！ `children` マップから削除して、設定ファイルからも消して、後片付けもちゃんとするんだ。
    fn remove_window(&mut self, window_id: WindowId) {
        // 1. 設定ファイルから削除するために id_str を取得
        let id_str_to_remove = if let Some(child) = self.children.get(&window_id) {
            child.id_str.clone() // 後で使うためにクローンしておく
        } else {
            log_error(&format!(
                "Cannot remove window {:?}: ChildWindow not found before removal.",
                window_id
            ));
            return; // 削除対象が見つからなければ終了
        };

        // 2. WindowManager の管理下から ChildWindow を削除
        if let Some(removed_child) = self.children.remove(&window_id) {
            log_info(&format!(
                "Removed ChildWindow (id_str: {}) from manager.",
                removed_child.id_str
            ));
            // removed_child がドロップされることで、Rc<Window> の参照カウントが減る。
            // 参照が他になければ、Window もドロップされ、OSレベルで閉じられるはず。
        } else {
            // remove_group_item で存在確認しているので、通常ここには来ないはず
            log_error(&format!(
                "Cannot remove window {:?}: ChildWindow not found during removal.",
                window_id
            ));
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
        {
            // 書き込みロックのスコープ
            let mut settings = get_settings_writer();
            if settings.children.remove(&id_str_to_remove).is_some() {
                log_info(&format!(
                    "Removed settings entry for id_str: {}",
                    id_str_to_remove
                ));
            } else {
                log_warn(&format!(
                    "Settings entry for id_str {} not found during removal.",
                    id_str_to_remove
                ));
            }
        } // 書き込みロック解放

        // --- ★設定保存処理を追加 (ウィンドウ削除後) ---
        log_info(&format!(
            "Window {:?} and its data removed successfully.",
            window_id
        ));
        save_settings(); // ウィンドウ削除は重要な操作なので、即時保存する
    }

    /// 指定されたウィンドウの、指定されたインデックスにあるアイコンを削除します。
    /// 「このアイコン、もういらないや～」って時に呼ばれて、リストから削除して再描画をお願いするよ。設定も忘れずに保存！
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
                // --- ★設定保存処理を更新 ---
                self.update_child_settings_in_memory(window_id);
                self.settings_are_dirty = true;
            } else {
                // 無効なインデックスの場合 (通常は起こらないはず)
                log_error(&format!(
                    "無効なインデックス {} でグループアイテムを削除しようとしました (グループ数: {})",
                    index,
                    child.groups.len()
                ));
            }
        }
    }

    /// 指定されたウィンドウ内の特定の物理座標 (`PhysicalPosition`) に
    /// どのアイコンが存在するかを判定します。
    /// マウスカーソルが動いた時に、「今、どの子ウィンドウのどのアイコンの上にいるのかな～？」って調べるのに使うよ！
    /// アイコンの描画範囲 (`get_item_rect_f32`) と見比べて判断するんだ。
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
                    if cursor_x >= item_rect.x()
                        && cursor_x < item_rect.x() + item_rect.width()
                        && cursor_y >= item_rect.y()
                        && cursor_y < item_rect.y() + item_rect.height()
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
