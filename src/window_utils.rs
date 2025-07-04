use windows::{
    Win32::UI::WindowsAndMessaging::{IDYES, MB_ICONWARNING, MB_YESNO, MessageBoxW},
    core::HSTRING,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event_loop::EventLoopWindowTarget,
    platform::windows::WindowBuilderExtWindows,
    window::{Window, WindowBuilder},
};

use crate::{mywindow::UserEvent, settings::ChildSettings};

/// メインウィンドウを作成します (通常は非表示)。
/// アプリケーションの生存期間中、イベントループを維持するために存在します。
///
/// # 引数
/// * `event_loop` - アプリケーションのイベントループだよ！
///                  このイベントループと関連付けられたウィンドウが作られるんだ。
///
/// # 戻り値
/// 作成された `winit::window::Window` インスタンス。
pub fn create_main_window(event_loop: &winit::event_loop::EventLoop<UserEvent>) -> Window {
    let window = WindowBuilder::new()
        .with_visible(false) // 表示しない
        .with_active(false) // アクティブにしない
        .with_title("Desktop Grouping (Main)") // 識別用のタイトル (任意)
        .build(event_loop)
        .expect("メインウィンドウの作成に失敗しました");

    return window;
}

/// 新しい子ウィンドウを作成します。
/// 設定に基づいて初期位置とサイズを設定できます。
///
/// # 引数
/// * `event_loop_target` - ウィンドウを作成するためのイベントループターゲットだよ！
///                         `main.rs` の `event_loop.run` クロージャの中で `target` として渡されるやつだね。
/// * `settings` - この子ウィンドウの初期設定 (`ChildSettings`) をオプションで渡せるよ。
///                `Some` だったらその設定値を、`None` だったらデフォルト値を使ってウィンドウを作るんだ。
///
/// # 戻り値
/// 作成された `winit::window::Window` インスタンス。
pub fn create_child_window(
    event_loop_target: &EventLoopWindowTarget<UserEvent>,
    settings: Option<&ChildSettings>,
) -> Window {
    let mut builder = WindowBuilder::new()
        .with_title("Desktop Grouping")
        .with_visible(true)
        .with_active(false)
        .with_skip_taskbar(true)
        .with_resizable(true)
        .with_transparent(true)
        .with_decorations(false);

    if let Some(s) = settings {
        builder = builder
            .with_position(PhysicalPosition::new(s.x, s.y))
            .with_inner_size(PhysicalSize::new(s.width, s.height));
    } else {
        let default_settings = ChildSettings::default();
        builder = builder
            .with_position(PhysicalPosition::new(
                default_settings.x,
                default_settings.y,
            ))
            .with_inner_size(PhysicalSize::new(
                default_settings.width,
                default_settings.height,
            ));
    }

    let window = builder
        .build(event_loop_target)
        .expect("子ウィンドウの作成に失敗しました");

    return window;
}

/// ウィンドウ削除の確認ダイアログを表示する関数。
/// Windows API の `MessageBoxW` を呼び出して、ユーザーに「はい」か「いいえ」を選んでもらうよ。
///
/// # 戻り値
/// ユーザーが「はい」(IDYES) を選んだら `true` を、それ以外なら `false` を返すよ。
pub fn show_confirmation_dialog() -> bool {
    let title = HSTRING::from("確認");
    let message =
        HSTRING::from("このグループウィンドウを削除しますか？\n(この操作は元に戻せません)");

    let result = unsafe {
        // Windows API を直接呼び出すから unsafe ブロックが必要なんだ。
        // ちょっとドキドキするけど、ちゃんと使えば大丈夫！(๑•̀ㅂ•́)و✧
        MessageBoxW(None, &message, &title, MB_YESNO | MB_ICONWARNING)
    };
    result == IDYES
}

#[cfg(test)]
mod tests {
    // use super::*; // window_utils.rs の中身をぜーんぶ使えるようにするおまじない！

    // うーん、このファイルにある関数たちは、テストを書くのがちょっと難しいんだ…(´・ω・｀)
    //
    // `create_main_window` と `create_child_window` は、
    // `winit::event_loop::EventLoop` や `winit::event_loop::EventLoopWindowTarget` が必要で、
    // これらはテストの中で作るのがちょっと大変なんだよね。
    //
    // `show_confirmation_dialog` は、呼び出すと実際にダイアログが表示されちゃって、
    // テストが止まっちゃうから、自動テストには向いてないんだ…＞＜
    //
    // だから、ごめんね、今回はテストコードはお休みさせてもらうね！(<em>ﾉω・</em>)ﾃﾍ
}
