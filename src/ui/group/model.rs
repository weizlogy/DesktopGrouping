/// グループウィンドウのデータを管理するよ！
/// DirectX などの描画詳細には一切依存しないピュアなデータ層。
pub struct GroupModel {
    pub id: String,
    pub title: String,
    pub bg_color_hex: String,
    pub opacity: f32, // 0.0 ~ 1.0
    pub icons: Vec<IconState>,
}

pub struct IconState {
    pub name: String,
    // TODO: ここに HICON などの情報を追加する予定
}

impl GroupModel {
    pub fn new(id: String, title: String, bg_color_hex: String, opacity: f32) -> Self {
        Self {
            id,
            title,
            bg_color_hex,
            opacity,
            icons: Vec::new(),
        }
    }
}
