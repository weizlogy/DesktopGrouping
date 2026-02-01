use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuItem, PredefinedMenuItem},
};

// トレイメニューのIDを定数化するよっ！٩(ˊᗜˋ*)و
pub const MENU_ID_NEW_GROUP: &str = "1001";
pub const MENU_ID_SETTINGS: &str = "1003";
pub const MENU_ID_QUIT: &str = "1002";

/// トレイアイコンを作成します。
///
/// # 説明
///
/// メニュー項目を設定し、トレイアイコンを初期化します。
///
/// # 戻り値
///
/// 作成されたトレイアイコン。
pub fn create_tray() -> TrayIcon {
    // まずは、トレイアイコンに表示するメニューを作るよ！(<em>´ω｀</em>)
    let menu = Menu::new();
    // 「新しいグループを作る」メニュー項目だよ！クリックできるように true にしてるんだ♪
    let new_group = MenuItem::with_id("1001", "New Group", true, None);
    let settings_item = MenuItem::with_id("1003", "Settings", true, None); // Settings メニュー項目
    let quit_i = MenuItem::with_id("1002", "Quit", true, None);

    menu.append_items(&[
        &new_group,
        &settings_item,
        &PredefinedMenuItem::separator(),
        &quit_i,
    ])
    .expect("menu append.");

    // よーし、いよいよトレイアイコン本体を作るよ！٩(ˊᗜˋ*)و
    return TrayIconBuilder::new()
        // さっき作ったメニューを、トレイアイコンにセット！
        .with_menu(Box::new(menu))
        // マウスを乗せた時に出る説明文（ツールチップ）も設定するよ！
        .with_tooltip("Desktop Grouping")
        // アプリのアイコンも忘れずに設定！リソースID 1番のアイコンを使うんだね！(・∀・)
        .with_icon(Icon::from_resource(1, None).unwrap())
        .build()
        .expect("Failed to create tray icon");
}
