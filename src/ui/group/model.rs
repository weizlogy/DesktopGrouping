/// グループウィンドウのデータを管理するよ！
/// DirectX などの描画詳細には一切依存しないピュアなデータ層。
pub struct GroupModel {
    pub title: String,
    pub bg_color_hex: String,
    pub icons: Vec<IconState>,
}

pub struct IconState {
    pub name: String,
    // TODO: ここに HICON などの情報を追加する予定
}

impl GroupModel {
    pub fn new(title: String, bg_color_hex: String) -> Self {
        Self {
            title,
            bg_color_hex,
            icons: Vec::new(),
        }
    }
}
