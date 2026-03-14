use windows::Win32::Foundation::{POINT, RECT, HWND};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_SHIFT, VK_MENU};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};
use crate::graphics::layout;

/// ユーザーの操作によって発生した抽象的なアクションだよ！
pub enum InteractionAction {
    None,
    Move { dx: i32, dy: i32 },   // 前回のフレームからの移動量
    Resize { dw: i32, dh: i32 }, // 前回のフレームからのリサイズ量
    ChangeOpacity { delta: f32 }, // 透明度の変化量 (不連続)
    ChangeOpacityContinuous { delta: f32 }, // 透明度の変化量 (連続)
    PasteColor,                  // クリップボードからの色貼り付け要求
    ExecuteIcon { index: usize }, // アイコンの実行
    DeleteIcon { index: usize },  // アイコンの削除
    OpenLocation { index: usize }, // ファイルの場所を開く
    DeleteGroup,                 // グループ自体の削除
    HoverChanged { index: Option<usize> }, // ホバー対象の変更
}

/// ウィンドウとのインタラクション（ドラッグ、リサイズ等）を管理するよ。
pub struct InteractionHandler {
    last_screen_pos: Option<POINT>,
    is_dragging: bool,
    is_resizing: bool,
    is_adjusting_opacity: bool,
    hovered_index: Option<usize>, // 現在ホバーされているアイコンのインデックス
}

impl InteractionHandler {
    pub fn new() -> Self {
        Self {
            last_screen_pos: None,
            is_dragging: false,
            is_resizing: false,
            is_adjusting_opacity: false,
            hovered_index: None,
        }
    }

    /// マウス座標からアイコンのインデックスを特定するよ！
    fn hit_test(hwnd: HWND, icon_count: usize) -> Option<usize> {
        let mut pt = POINT::default();
        let mut rect = RECT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_err() || GetWindowRect(hwnd, &mut rect).is_err() {
                return None;
            }
        }

        let rel_x = (pt.x - rect.left) as f32;
        let rel_y = (pt.y - rect.top) as f32;
        let width = (rect.right - rect.left) as f32;

        let layouts = layout::calculate_grid_layout(width, icon_count, 1.0);
        for (i, layout) in layouts.iter().enumerate() {
            if rel_x >= layout.hit_rect.left && rel_x <= layout.hit_rect.right &&
               rel_y >= layout.hit_rect.top && rel_y <= layout.hit_rect.bottom {
                return Some(i);
            }
        }

        None
    }

    /// マウスボタンが押されたときの処理だよ。
    pub fn handle_lbutton_down(&mut self) {
        let mut pt = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut pt);
        }

        let is_ctrl = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };
        let is_shift = unsafe { (GetKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0 };
        let is_alt = unsafe { (GetKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0 };

        if is_ctrl {
            self.is_dragging = true;
        } else if is_shift {
            self.is_resizing = true;
        } else if is_alt {
            self.is_adjusting_opacity = true;
        }

        self.last_screen_pos = Some(pt);
    }

    /// ダブルクリックされたときの処理だよ。
    pub fn handle_lbutton_dblclk(&self, hwnd: HWND, icon_count: usize) -> InteractionAction {
        if let Some(index) = Self::hit_test(hwnd, icon_count) {
            return InteractionAction::ExecuteIcon { index };
        }
        InteractionAction::None
    }

    /// 右クリックされたときの処理だよ。
    pub fn handle_rbutton_down(&self, hwnd: HWND, icon_count: usize) -> InteractionAction {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let is_ctrl = unsafe { (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };
        let hit_index = Self::hit_test(hwnd, icon_count);

        match (hit_index, is_ctrl) {
            (Some(index), true) => InteractionAction::DeleteIcon { index },
            (None, true) => InteractionAction::DeleteGroup,
            (Some(index), false) => InteractionAction::OpenLocation { index },
            _ => InteractionAction::None,
        }
    }

    /// マウスが動いたときの処理だよ。
    pub fn handle_mouse_move(&mut self, hwnd: HWND, icon_count: usize) -> InteractionAction {
        let mut pt = POINT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_err() {
                return InteractionAction::None;
            }
        }

        // 1. ホバー判定の更新
        let new_hover = Self::hit_test(hwnd, icon_count);
        if new_hover != self.hovered_index {
            self.hovered_index = new_hover;
            // 他のドラッグ中などの操作を優先しつつ, ホバー変更を通知するよ
            if !self.is_dragging && !self.is_resizing && !self.is_adjusting_opacity {
                return InteractionAction::HoverChanged { index: new_hover };
            }
        }

        // 2. ドラッグ等の差分計算
        if let Some(last_pos) = self.last_screen_pos {
            let dx = pt.x - last_pos.x;
            let dy = pt.y - last_pos.y;

            if dx == 0 && dy == 0 {
                return InteractionAction::None;
            }

            self.last_screen_pos = Some(pt);

            if self.is_dragging {
                return InteractionAction::Move { dx, dy };
            } else if self.is_resizing {
                return InteractionAction::Resize { dw: dx, dh: dy };
            } else if self.is_adjusting_opacity {
                return InteractionAction::ChangeOpacityContinuous { delta: dx as f32 * 0.005 };
            }
        }
        InteractionAction::None
    }

    pub fn handle_mouse_wheel(&self, delta: i16) -> InteractionAction {
        let is_ctrl = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };
        if is_ctrl {
            let step = 0.05;
            let delta_f = if delta > 0 { step } else { -step };
            return InteractionAction::ChangeOpacity { delta: delta_f };
        }
        InteractionAction::None
    }

    pub fn handle_keydown(&self, virtual_key: u16) -> InteractionAction {
        let is_ctrl = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };
        if is_ctrl && virtual_key == 'V' as u16 {
            return InteractionAction::PasteColor;
        }
        InteractionAction::None
    }

    pub fn handle_lbutton_up(&mut self) {
        self.is_dragging = false;
        self.is_resizing = false;
        self.is_adjusting_opacity = false;
        self.last_screen_pos = None;
    }
}
