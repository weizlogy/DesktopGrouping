use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSENDCHANGING, SWP_NOSIZE, SetWindowPos,
    },
};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

/// winit の `Window` から Windows API で使う `HWND` (ウィンドウハンドル) を取得するよ！(<em>´ω｀</em>)
///
/// # 引数
/// * `window` - `HWND` を取得したい `winit::window::Window` の参照だよ。
///
/// # 戻り値
/// * `HWND` - 取得したウィンドウハンドル。もし Windows じゃない環境だったらパニックしちゃう！＞＜
fn handle_from_window(window: &Window) -> HWND {
    let hwnd = match window.window_handle().unwrap().as_raw() {
        // Windows のハンドルだったら、それを取り出すよ！
        RawWindowHandle::Win32(handle) => handle.hwnd.get(),
        // それ以外だったら、Windows じゃないからパニック！Σ(ﾟДﾟ)
        _ => panic!("not running on Windows"),
    };
    return HWND(hwnd);
}

/// 指定されたウィンドウを、他のウィンドウの一番後ろ (最背面) に移動させるよ！(ゝω・)v
///
/// # 引数
/// * `window` - 最背面に移動させたい `winit::window::Window` の参照だよ。
pub fn set_window_pos_to_bottom(window: &Window) {
    // Windows API を直接呼ぶから、`unsafe` ブロックで囲むね！ドキドキ…！
    unsafe {
        let _ = SetWindowPos(
            handle_from_window(window),
            HWND_BOTTOM,
            0,
            0,
            0,
            0,
            // 位置やサイズは変えないで、アクティブにもしないで、ただ一番後ろに送るだけ！っていうフラグだよ♪
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOSENDCHANGING,
        );
    }
}
