use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        DefWindowProcW, PostQuitMessage, WM_DESTROY, WM_PAINT, WM_SIZE, WM_ERASEBKGND,
        GetWindowLongPtrW, GWLP_USERDATA,
    },
    Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT},
};
use crate::ui::group::window::GroupWindow;

/// ウィンドウに対するメッセージを裁くプロシージャだよ！
pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // GWLP_USERDATA から GroupWindow のポインタを取得するよ
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);

    if ptr != 0 {
        let window = &mut *(ptr as *mut GroupWindow);

        match msg {
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
            WM_ERASEBKGND => {
                // 背景消去を OS にさせないことでチラツキを抑えるよ (DirectX で描くから不要)
                return LRESULT(1);
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                return LRESULT(0);
            }
            _ => {}
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
