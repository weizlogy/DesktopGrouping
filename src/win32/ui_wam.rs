use std::ffi::c_void;
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Dwm::{
        DWMSBT_NONE, DWMWA_NCRENDERING_POLICY, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_TRANSITIONS_FORCEDISABLED, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
    },
    Graphics::Gdi::{InvalidateRect, UpdateWindow},
    UI::Controls::MARGINS,
    UI::Shell::{DefSubclassProc, SetWindowSubclass},
    UI::WindowsAndMessaging::{
        GCLP_HBRBACKGROUND, GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, HWND_BOTTOM, HWND_TOP,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSENDCHANGING, SWP_NOSIZE, SWP_NOZORDER,
        SetClassLongPtrW, SetWindowLongPtrW, SetWindowPos, WM_CTLCOLORDLG, WM_CTLCOLORMSGBOX,
        WM_CTLCOLORSTATIC, WM_ERASEBKGND, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCPAINT, WM_PAINT,
        WS_CAPTION, WS_EX_LAYERED, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU,
    },
};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

#[repr(C)]
struct ACCENT_POLICY {
    pub accent_state: u32,
    pub accent_flags: u32,
    pub gradient_color: u32,
    pub animation_id: u32,
}

#[repr(C)]
struct WINDOWCOMPOSITIONATTRIBDATA {
    pub attrib: u32,
    pub pv_data: *const c_void,
    pub cb_data: usize,
}

const WCA_ACCENT_POLICY: u32 = 19;
const ACCENT_ENABLE_TRANSPARENTGRADIENT: u32 = 2;

/// winit の `Window` から Windows API で使う `HWND` を取得するよ！
fn handle_from_window(window: &Window) -> HWND {
    let hwnd = match window.window_handle().unwrap().as_raw() {
        RawWindowHandle::Win32(handle) => handle.hwnd.get(),
        _ => panic!("not running on Windows"),
    };
    return HWND(hwnd);
}

/// 指定された HWND に対して DWM の属性とフレーム拡張を適用する内部関数だよ。
unsafe fn apply_dwm_settings(hwnd: HWND) {
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &BOOL::from(true) as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            &BOOL::from(true) as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &DWMSBT_NONE as *const _ as *const _,
            std::mem::size_of::<windows::Win32::Graphics::Dwm::DWM_SYSTEMBACKDROP_TYPE>() as u32,
        );
    }
    let margins = MARGINS {
        cxLeftWidth: -1,
        cxRightWidth: -1,
        cyTopHeight: -1,
        cyBottomHeight: -1,
    };
    unsafe {
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
    }
}

/// ウィンドウの描画を強制的に更新させるよ！
pub fn force_update_window(window: &Window) {
    let hwnd = handle_from_window(window);
    unsafe {
        let _ = InvalidateRect(hwnd, None, BOOL::from(true));
        let _ = UpdateWindow(hwnd);
    }
}

/// 指定されたウィンドウを最背面に移動させるよ！
pub fn set_window_pos_to_bottom(window: &Window) {
    unsafe {
        let _ = SetWindowPos(
            handle_from_window(window),
            HWND_BOTTOM,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOSENDCHANGING,
        );
    }
}

/// DWM (Desktop Window Manager) によるウィンドウ属性の設定を行うよ！
pub fn enable_dwm_composition(window: &Window) {
    let hwnd = handle_from_window(window);
    unsafe {
        apply_dwm_settings(hwnd);
    }
}

static mut LAST_COLOR: u32 = 0;
/// AccentPolicy を使ってウィンドウの背景透過度を設定するよ！
pub fn set_window_composition(window: &Window, color: tiny_skia::Color) {
    let hwnd = handle_from_window(window);

    let r = (color.red() * 255.0) as u32;
    let g = (color.green() * 255.0) as u32;
    let b = (color.blue() * 255.0) as u32;
    let a = (color.alpha() * 255.0) as u32;

    let mut gradient_color = (a << 24) | (b << 16) | (g << 8) | r;
    unsafe {
        if gradient_color == LAST_COLOR {
            gradient_color ^= 0x00000001; // 最下位ビットだけ変える
        }
        LAST_COLOR = gradient_color;
    }

    let policy = ACCENT_POLICY {
        accent_state: ACCENT_ENABLE_TRANSPARENTGRADIENT,
        accent_flags: 0,
        gradient_color: gradient_color,
        animation_id: 0,
    };

    let data = WINDOWCOMPOSITIONATTRIBDATA {
        attrib: WCA_ACCENT_POLICY,
        pv_data: &policy as *const _ as *const c_void,
        cb_data: std::mem::size_of::<ACCENT_POLICY>(),
    };

    unsafe {
        if let Ok(user32) =
            windows::Win32::System::LibraryLoader::GetModuleHandleW(windows::core::w!("user32.dll"))
        {
            if let Some(func_ptr) = windows::Win32::System::LibraryLoader::GetProcAddress(
                user32,
                windows::core::s!("SetWindowCompositionAttribute"),
            ) {
                let func: unsafe extern "system" fn(
                    HWND,
                    *const WINDOWCOMPOSITIONATTRIBDATA,
                ) -> BOOL = std::mem::transmute(func_ptr);

                let _ = func(hwnd, &data);
            }
        }
    }
}

/// ウィンドウから余計な装飾を剥ぎ取り, サブクラス化して特定のメッセージをフックするよ！
pub fn remove_window_decoration_styles(window: &Window) {
    let hwnd = handle_from_window(window);
    unsafe {
        // ウィンドウクラスの背景ブラシを NULL にして, OS による自動背景描画を止めるよ
        SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, 0);

        // スタイル設定
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let new_style = (style | WS_CAPTION.0 as isize)
            & !(WS_MINIMIZEBOX.0 as isize | WS_MAXIMIZEBOX.0 as isize | WS_SYSMENU.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

        // 拡張スタイル設定
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new_ex_style = ex_style & !(WS_EX_LAYERED.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style);

        // スタイル反映
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
        );

        // ウィンドウをサブクラス化して, メッセージをフックするよ！
        let _ = SetWindowSubclass(hwnd, Some(subclass_proc), 1, 0);
    }
}

/// サブクラスプロシージャ。特定のメッセージを横取りして独自の処理をするよ！
unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        // 背景消去要求を無視
        WM_ERASEBKGND => LRESULT(1),

        // コントロールの背景描画要求を横取りして、OS に背景を塗らせないようにするよ
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLORMSGBOX => LRESULT(1),

        // 非クライアント領域のアクティブ化要求を常に許可
        WM_NCACTIVATE => LRESULT(1),

        // 非クライアント領域の描画を抑制
        WM_NCPAINT => LRESULT(0),

        // 非クライアント領域の計算で全領域をクライアント領域にする
        WM_NCCALCSIZE => {
            if wparam.0 != 0 {
                LRESULT(0)
            } else {
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }
        }

        WM_PAINT => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },

        _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
    }
}
