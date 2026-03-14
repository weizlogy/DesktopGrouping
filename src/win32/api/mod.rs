pub mod create_window;
pub mod message_loop;
pub mod register_class;
pub mod show_window;
pub mod utils;
pub mod shell;

pub const WM_REMOVE_WINDOW: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;

pub use create_window::*;
pub use message_loop::*;
pub use register_class::*;
pub use show_window::*;
pub use utils::*;
pub use shell::*;
