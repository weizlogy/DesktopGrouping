use windows::Win32::{
  Foundation::HWND,
  UI::WindowsAndMessaging::{
    SetWindowPos, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSENDCHANGING, SWP_NOSIZE
  }
};
use winit::{
  window::Window,
  raw_window_handle::{
    HasWindowHandle, RawWindowHandle
  }
};

fn handle_from_window(window: &Window) -> HWND {
  let hwnd = match window.window_handle().unwrap().as_raw() {
    RawWindowHandle::Win32(handle) => handle.hwnd.get(),
    _ => panic!("not running on Windows")
  };
  return HWND(hwnd);
}

pub fn set_window_pos_to_bottom(window: &Window) {
  unsafe {
    let _ = SetWindowPos(
      handle_from_window(window),
      HWND_BOTTOM,
      0, 0, 0, 0,
      SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOSENDCHANGING);
  }
}
