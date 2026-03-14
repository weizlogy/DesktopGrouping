use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        DefWindowProcW, PostQuitMessage, WM_DESTROY, WM_PAINT, WM_SIZE, WM_ERASEBKGND,
        WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_LBUTTONUP, WM_NCHITTEST, HTCLIENT,
        WM_KEYDOWN, WM_DROPFILES, WM_LBUTTONDBLCLK, WM_RBUTTONDOWN,
        WM_WINDOWPOSCHANGING, WM_MOUSEACTIVATE, MA_NOACTIVATE, WINDOWPOS, HWND_BOTTOM,
        GetWindowLongPtrW, GWLP_USERDATA,
    },
    Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT},
};
use windows::Win32::UI::Shell::{HDROP, DragFinish};
use crate::ui::group::window::GroupWindow;
use crate::win32::api;

/// ウィンドウに対するメッセージを裁くプロシージャだよ！
pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);

        if ptr == 0 {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let window = &mut *(ptr as *mut GroupWindow);

        match msg {
            WM_NCHITTEST => {
                return LRESULT(HTCLIENT as isize);
            }
            WM_WINDOWPOSCHANGING => {
                // Zオーダーが変更されようとしているときに介入するよ！
                // 強制的に最背面 (HWND_BOTTOM) に挿入されるように設定を書き換えるんだ。
                let window_pos = &mut *(lparam.0 as *mut WINDOWPOS);
                window_pos.hwndInsertAfter = HWND_BOTTOM;
                // ここでは DefWindowProcW を呼ばずに 0 を返してもいいし, 
                // 書き換えた状態でそのまま流してもいいよ。
                return LRESULT(0);
            }
            WM_MOUSEACTIVATE => {
                // クリックしてもアクティブ化（最前面化）させないようにするよ。
                return LRESULT(MA_NOACTIVATE as isize);
            }
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                BeginPaint(hwnd, &mut ps);
                if let Err(e) = window.draw() {
                    log::error!("Draw error: {}", e);
                }
                EndPaint(hwnd, &ps);
                return LRESULT(0);
            }
            WM_SIZE => {
                let width = (lparam.0 & 0xFFFF) as u32;
                let height = ((lparam.0 >> 16) & 0xFFFF) as u32;
                if let Err(e) = window.handle_resize(width, height) {
                    log::error!("Resize error: {}", e);
                }
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                window.handle_lbutton_down();
                return LRESULT(0);
            }
            WM_LBUTTONDBLCLK => {
                if let Err(e) = window.handle_lbutton_dblclk() {
                    log::error!("Double click error: {}", e);
                }
                return LRESULT(0);
            }
            WM_RBUTTONDOWN => {
                if let Err(e) = window.handle_rbutton_down() {
                    log::error!("Right button down error: {}", e);
                }
                return LRESULT(0);
            }
            WM_MOUSEMOVE => {
                if let Err(e) = window.handle_mouse_move() {
                    log::error!("Mouse move error: {}", e);
                }
                return LRESULT(0);
            }
            WM_LBUTTONUP => {
                window.handle_lbutton_up();
                return LRESULT(0);
            }
            WM_KEYDOWN => {
                let vk = wparam.0 as u16;
                if let Err(e) = window.handle_keydown(vk) {
                    log::error!("Keydown error: {}", e);
                }
                return LRESULT(0);
            }
            WM_DROPFILES => {
                let hdrop = HDROP(wparam.0 as isize);
                let files = api::utils::get_dropped_files(hdrop);
                if let Err(e) = window.handle_drop_files(files) {
                    log::error!("Drop files error: {}", e);
                }
                DragFinish(hdrop);
                return LRESULT(0);
            }
            WM_ERASEBKGND => {
                return LRESULT(1);
            }
            WM_DESTROY => {
                return LRESULT(0);
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
