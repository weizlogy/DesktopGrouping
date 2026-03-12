use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HINSTANCE, ERROR_CLASS_ALREADY_EXISTS},
        UI::WindowsAndMessaging::{
            LoadCursorW, RegisterClassExW, CS_HREDRAW, CS_VREDRAW, HICON, IDC_ARROW, WNDCLASSEXW,
            WNDPROC,
        },
    },
};

/// ウィンドウクラスを登録するよ！
pub fn register_window_class(
    instance: HINSTANCE,
    class_name: PCWSTR,
    wnd_proc: WNDPROC,
) -> Result<(), windows::core::Error> {
    unsafe {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: wnd_proc,
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: class_name,
            hIcon: HICON::default(),
            ..Default::default()
        };

        if RegisterClassExW(&wc) == 0 {
            let err = windows::core::Error::from_win32();
            // すでに登録されている場合はエラーにしないよ
            if err.code() != ERROR_CLASS_ALREADY_EXISTS.to_hresult() {
                return Err(err);
            }
        }
    }
    Ok(())
}
