use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// ユーザーの操作によって発生した抽象的なアクションだよ！
pub enum InteractionAction {
    None,
    Move { dx: i32, dy: i32 },   // 前回のフレームからの移動量
    Resize { dw: i32, dh: i32 }, // 前回のフレームからのリサイズ量
}

/// ウィンドウとのインタラクション（ドラッグ、リサイズ等）を管理するよ。
pub struct InteractionHandler {
    last_screen_pos: Option<POINT>,
    is_dragging: bool,
    is_resizing: bool,
}

impl InteractionHandler {
    pub fn new() -> Self {
        Self {
            last_screen_pos: None,
            is_dragging: false,
            is_resizing: false,
        }
    }

    /// マウスボタンが押されたときの処理だよ。
    /// スクリーン座標を取得して、操作モードを確定させるよ。
    pub fn handle_lbutton_down(&mut self) {
        let mut pt = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut pt);
        }

        let is_ctrl = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };
        let is_shift = unsafe { (GetKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0 };

        if is_ctrl {
            self.is_dragging = true;
        } else if is_shift {
            self.is_resizing = true;
        }

        self.last_screen_pos = Some(pt);
    }

    /// マウスが動いたときの処理だよ。
    /// 前回のスクリーン座標との差分を計算し、アクションを返すよ。
    pub fn handle_mouse_move(&mut self) -> InteractionAction {
        let mut pt = POINT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_err() {
                return InteractionAction::None;
            }
        }

        if let Some(last_pos) = self.last_screen_pos {
            let dx = pt.x - last_pos.x;
            let dy = pt.y - last_pos.y;

            if dx == 0 && dy == 0 {
                return InteractionAction::None;
            }

            // 毎フレーム更新することで「差分」を正しく計算できるようにするよ！
            self.last_screen_pos = Some(pt);

            if self.is_dragging {
                return InteractionAction::Move { dx, dy };
            } else if self.is_resizing {
                return InteractionAction::Resize { dw: dx, dh: dy };
            }
        }
        InteractionAction::None
    }

    /// マウスボタンが離されたら、すべての状態をリセットするよ。
    pub fn handle_lbutton_up(&mut self) {
        self.is_dragging = false;
        self.is_resizing = false;
        self.last_screen_pos = None;
    }
}
