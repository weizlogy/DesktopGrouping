use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HINSTANCE, HWND},
        UI::WindowsAndMessaging::{
            CreateWindowExW, CW_USEDEFAULT, WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_APPWINDOW,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

/// ウィンドウ作成時の設定をまとめた構造体だよ！
/// Default トレイトを実装して, 最小限の指定で済むようにするよ。
pub struct WindowOptions {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub ex_style: Option<WINDOW_EX_STYLE>,
    pub style: Option<WINDOW_STYLE>,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
            width: CW_USEDEFAULT,
            height: CW_USEDEFAULT,
            ex_style: None,
            style: None,
        }
    }
}

/// ウィンドウを作成して, そのハンドル (HWND) を返すよ！
pub fn create_window(
    instance: HINSTANCE,
    class_name: PCWSTR,
    window_name: PCWSTR,
    options: WindowOptions,
) -> Result<HWND, windows::core::Error> {
    unsafe {
        let hwnd = CreateWindowExW(
            options.ex_style.unwrap_or(WS_EX_APPWINDOW),
            class_name,
            window_name,
            options.style.unwrap_or(WS_OVERLAPPEDWINDOW),
            options.x,
            options.y,
            options.width,
            options.height,
            None,
            None,
            instance,
            None,
        );

        if hwnd.0 == 0 {
            return Err(windows::core::Error::from_win32());
        }
        Ok(hwnd)
    }
}
