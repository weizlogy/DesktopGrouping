use tray_icon::{
  menu::{Menu, MenuItem, PredefinedMenuItem}, Icon, TrayIcon, TrayIconBuilder
};

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
  // メニューの作成
  let menu = Menu::new();
  let new_group = MenuItem::new("New Group", true, None);
  let quit_i = MenuItem::new("Quit", true, None);

  menu.append_items(&[&new_group, &PredefinedMenuItem::separator(), &quit_i])
    .expect("Failed to append items");

  // トレイの作成
  return TrayIconBuilder::new()
    .with_menu(Box::new(menu))
    .with_tooltip("Desktop Grouping")
    .with_icon(Icon::from_resource(1, None).unwrap())
    .build()
    .expect("Failed to create tray icon");
}
