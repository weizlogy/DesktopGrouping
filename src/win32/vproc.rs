use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        DefWindowProcW, WM_DESTROY, WM_PAINT, WM_SIZE, WM_ERASEBKGND,
        WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_LBUTTONUP, WM_NCHITTEST, HTCLIENT,
        WM_KEYDOWN, WM_DROPFILES, WM_LBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP,
        WM_CONTEXTMENU,
        WM_WINDOWPOSCHANGING, WM_MOUSEACTIVATE, MA_NOACTIVATE, WINDOWPOS, HWND_BOTTOM,
        WM_TIMER,
        GetWindowLongPtrW, GWLP_USERDATA,
    },
    Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT},
};
use windows::Win32::UI::Shell::{HDROP, DragFinish};
use crate::ui::group::window::GroupWindow;
use crate::ui::help::window::HelpWindow;
use crate::ui::WindowType;
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

        // 最初のフィールド(WindowType)をチェックするよ！
        let window_type = *(ptr as *const WindowType);

        match window_type {
            WindowType::Group => {
                let window = &mut *(ptr as *mut GroupWindow);
                handle_group_msg(window, hwnd, msg, wparam, lparam)
            }
            WindowType::Help => {
                let window = &mut *(ptr as *mut HelpWindow);
                handle_help_msg(window, hwnd, msg, wparam, lparam)
            }
        }
    }
}

unsafe fn handle_group_msg(
    window: &mut GroupWindow,
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCHITTEST => {
            return LRESULT(HTCLIENT as isize);
        }
        WM_WINDOWPOSCHANGING => {
            let window_pos = &mut *(lparam.0 as *mut WINDOWPOS);
            window_pos.hwndInsertAfter = HWND_BOTTOM;
            return LRESULT(0);
        }
        WM_MOUSEACTIVATE => {
            return LRESULT(MA_NOACTIVATE as isize);
        }
        WM_TIMER => {
            if let Err(e) = window.handle_timer(wparam.0) {
                log::error!("Timer error: {}", e);
            }
            return LRESULT(0);
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
        WM_RBUTTONUP => {
            if let Err(e) = window.handle_rbutton_up() {
                log::error!("Right button up error: {}", e);
            }
            return LRESULT(0);
        }
        WM_CONTEXTMENU => {
            return LRESULT(0); // デスクトップにメッセージが伝わらないようにトラップするよ
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

unsafe fn handle_help_msg(
    window: &mut HelpWindow,
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCHITTEST => {
            return LRESULT(HTCLIENT as isize);
        }
        WM_WINDOWPOSCHANGING => {
            let window_pos = &mut *(lparam.0 as *mut WINDOWPOS);
            window_pos.hwndInsertAfter = HWND_BOTTOM;
            return LRESULT(0);
        }
        WM_MOUSEACTIVATE => {
            return LRESULT(MA_NOACTIVATE as isize);
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            BeginPaint(hwnd, &mut ps);
            if let Err(e) = window.draw() {
                log::error!("Help Draw error: {}", e);
            }
            EndPaint(hwnd, &ps);
            return LRESULT(0);
        }
        WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            window.handle_lbutton_down(x, y);
            return LRESULT(0);
        }
        WM_MOUSEMOVE => {
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            if let Err(e) = window.handle_mouse_move(x, y) {
                log::error!("Help Mouse move error: {}", e);
            }
            return LRESULT(0);
        }
        WM_LBUTTONUP => {
            window.handle_lbutton_up();
            return LRESULT(0);
        }
        WM_RBUTTONDOWN => {
            if let Err(e) = window.handle_rbutton_down() {
                log::error!("Help Right button down error: {}", e);
            }
            return LRESULT(0);
        }
        WM_RBUTTONUP => {
            if let Err(e) = window.handle_rbutton_up() {
                log::error!("Help Right button up error: {}", e);
            }
            return LRESULT(0);
        }
        WM_CONTEXTMENU => {
            return LRESULT(0);
        }
        WM_ERASEBKGND => {
            return LRESULT(1);
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}
