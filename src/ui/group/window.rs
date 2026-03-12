use std::rc::Rc;
use windows::Win32::Foundation::HWND;
use crate::graphics::GraphicsEngine;
use crate::ui::group::model::GroupModel;
use crate::ui::group::renderer::GroupRenderer;
use crate::win32::api;
use windows::core::PCWSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
    SetWindowLongPtrW, GWLP_USERDATA,
};
use crate::win32::vproc::window_proc;

/// グループウィンドウを統括するコンポーネントだよ！
/// 状態 (Model) と描画 (Renderer) を橋渡しする役割を担うよ。
pub struct GroupWindow {
    pub hwnd: HWND,
    pub model: GroupModel,
    pub renderer: GroupRenderer,
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
        let options = api::create_window::WindowOptions {
            width: width as i32,
            height: height as i32,
            ex_style: Some(WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE),
            style: Some(WS_POPUP | WS_VISIBLE),
            ..Default::default()
        };

        let hwnd = api::create_window::create_window(
            instance.into(),
            class_pcwstr,
            window_pcwstr,
            options,
        )?;

        // 最背面に移動
        api::show_window::move_to_bottom(hwnd);

        let model = GroupModel::new(title, bg_color_hex);
        let renderer = GroupRenderer::new(engine, hwnd, width, height)?;

        let window = Box::new(Self {
            hwnd,
            model,
            renderer,
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
        let mut rect = windows::Win32::Foundation::RECT::default();
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
}
