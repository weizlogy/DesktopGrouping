use crate::graphics::GraphicsEngine;
use crate::ui::group::interaction::{InteractionAction, InteractionHandler};
use crate::ui::group::model::GroupModel;
use crate::ui::group::renderer::GroupRenderer;
use crate::win32::api;
use std::rc::Rc;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, SetWindowLongPtrW, SetWindowPos, GWLP_USERDATA, HWND_BOTTOM, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_POPUP, WS_VISIBLE,
};
use crate::win32::vproc::window_proc;

/// グループウィンドウを統括するコンポーネントだよ！
/// 状態 (Model) と描画 (Renderer) を橋渡しする役割を担うよ。
pub struct GroupWindow {
    pub hwnd: HWND,
    pub model: GroupModel,
    pub renderer: GroupRenderer,
    pub interaction: InteractionHandler,
}

impl GroupWindow {
    /// 新しいグループウィンドウを作成して, 初期化するよ！
    /// アドレスを固定するために Box<Self> を返すようにするよ。
    pub fn create(
        engine: Rc<GraphicsEngine>,
        title: String,
        bg_color_hex: String,
        width: u32,
        height: u32,
    ) -> Result<Box<Self>, windows::core::Error> {
        let instance = unsafe { GetModuleHandleW(None)? };
        let class_name_str = "DesktopGroupingGroupClass";
        let class_name = api::utils::to_wide(class_name_str);
        let window_name = api::utils::to_wide(&title);
        let class_pcwstr = PCWSTR::from_raw(class_name.as_ptr());
        let window_pcwstr = PCWSTR::from_raw(window_name.as_ptr());

        // クラス登録
        api::register_class::register_window_class(
            instance.into(),
            class_pcwstr,
            Some(window_proc),
        )?;

        // スタイル設定とサイズ指定
        const WS_EX_NOREDIRECTIONBITMAP: windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE =
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0x00200000);

        let options = api::create_window::WindowOptions {
            width: width as i32,
            height: height as i32,
            ex_style: Some(
                WS_EX_LAYERED
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_NOREDIRECTIONBITMAP,
            ),
            style: Some(WS_POPUP | WS_VISIBLE),
            ..Default::default()
        };

        let hwnd = api::create_window::create_window(
            instance.into(),
            class_pcwstr,
            window_pcwstr,
            options,
        )?;

        // クリック判定領域をウィンドウ全体に広げるよ（これがないとリサイズ時に判定が追従しない！）
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes(
                hwnd,
                windows::Win32::Foundation::COLORREF(0),
                255,
                windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA,
            )?;
        }

        // 最背面に移動
        api::show_window::move_to_bottom(hwnd);

        let model = GroupModel::new(title, bg_color_hex);
        let renderer = GroupRenderer::new(engine, hwnd, width, height)?;
        let interaction = InteractionHandler::new();

        let window = Box::new(Self {
            hwnd,
            model,
            renderer,
            interaction,
        });

        // 自身のポインタを HWND に紐付ける (重要！)
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, &*window as *const Self as isize);
        }

        Ok(window)
    }

    /// 描画を実行するよ。
    /// WM_PAINT などのメッセージが来たときに呼び出してね。
    pub fn draw(&mut self) -> Result<(), windows::core::Error> {
        let mut rect = RECT::default();
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetClientRect(self.hwnd, &mut rect)?;
        }
        let width = (rect.right - rect.left) as f32;
        let height = (rect.bottom - rect.top) as f32;

        self.renderer.render(&self.model, width, height)
    }

    /// ウィンドウサイズが変更されたときの処理だよ。
    /// WM_SIZE などのメッセージが来たときに呼び出してね。
    pub fn handle_resize(&mut self, width: u32, height: u32) -> Result<(), windows::core::Error> {
        self.renderer.resize(width, height)
    }

    /// マウスの左ボタンが押されたときの処理だよ。
    pub fn handle_lbutton_down(&mut self) {
        self.interaction.handle_lbutton_down();
        unsafe {
            // マウスキャプチャを開始（ウィンドウ外に出ても追従するようにする）
            windows::Win32::UI::Input::KeyboardAndMouse::SetCapture(self.hwnd);
        }
    }

    /// マウスが動いたときの処理だよ。
    pub fn handle_mouse_move(&mut self) -> Result<(), windows::core::Error> {
        let action = self.interaction.handle_mouse_move();

        match action {
            InteractionAction::Move { dx, dy } => {
                let mut rect = RECT::default();
                unsafe {
                    GetWindowRect(self.hwnd, &mut rect)?;
                    SetWindowPos(
                        self.hwnd,
                        HWND_BOTTOM,
                        rect.left + dx,
                        rect.top + dy,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOACTIVATE,
                    )?;
                }
            }
            InteractionAction::Resize { dw, dh } => {
                let mut rect = RECT::default();
                unsafe {
                    GetWindowRect(self.hwnd, &mut rect)?;
                    let new_width = (rect.right - rect.left) + dw;
                    let new_height = (rect.bottom - rect.top) + dh;

                    let new_width = new_width.max(50);
                    let new_height = new_height.max(50);

                    SetWindowPos(
                        self.hwnd,
                        HWND_BOTTOM,
                        0,
                        0,
                        new_width,
                        new_height,
                        SWP_NOMOVE | SWP_NOACTIVATE,
                    )?;
                }
                // 即座に描画を更新するよ！
                if let Err(e) = self.draw() {
                    log::error!("Draw error during resize: {}", e);
                }
            }
            InteractionAction::None => {}
        }

        Ok(())
    }

    /// マウスの左ボタンが離されたときの処理だよ。
    pub fn handle_lbutton_up(&mut self) {
        self.interaction.handle_lbutton_up();
        unsafe {
            // キャプチャを解放
            windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture().ok();
        }
    }
}
