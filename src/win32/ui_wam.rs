use windows::Win32::{
    Foundation::{BOOL, HWND},
    Graphics::Dwm::{DWMWA_TRANSITIONS_FORCEDISABLED, DwmSetWindowAttribute},
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

/// DWM (Desktop Window Manager) によるウィンドウのコンポジション（透過処理とか）を有効にするよ！
/// これを使うと、ウィンドウ全体をきれいに透過させたりできるんだ。(๑•̀ㅂ•́)و✧
///
/// # 引数
/// * `window` - DWM コンポジションを有効にしたい `winit::window::Window` の参照だよ。
pub fn enable_dwm_composition(window: &Window) {
    let hwnd = handle_from_window(window);
    unsafe {
        // ウィンドウのアニメーション（最小化・最大化とか）を無効にするよ。
        // これをやると、リサイズとかの時のチラつきが抑えられるんだ！
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &BOOL::from(true) as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );

        // DwmExtendFrameIntoClientAreaを使うとタイトルバーのボタンが
        // 表示されてしまうので使わないこと
        /*
           // 余白を -1 に設定すると、ウィンドウ全体が透過対象になるんだよ！
           let margins = MARGINS {
               cxLeftWidth: -1,
               cxRightWidth: -1,
               cyTopHeight: -1,
               cyBottomHeight: -1,
           };
           let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        */
    }
}
