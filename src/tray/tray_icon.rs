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
  // まずは、トレイアイコンに表示するメニューを作るよ！(<em>´ω｀</em>)
  let menu = Menu::new();
  // 「新しいグループを作る」メニュー項目だよ！クリックできるように true にしてるんだ♪
  let new_group = MenuItem::new("New Group", true, None);
  // 「アプリを終了する」メニュー項目だよ！これもクリックできるように true！
  let quit_i = MenuItem::new("Quit", true, None);

  // 作ったメニュー項目たちを、メニューに追加していくよ！
  // new_group と quit_i の間には、区切り線 (セパレーター) も入れて見やすくするんだ～！(ゝω・)v
  menu.append_items(&[&new_group, &PredefinedMenuItem::separator(), &quit_i])
    .expect("Failed to append items");

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
