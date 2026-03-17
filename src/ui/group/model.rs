use std::path::PathBuf;

/// グループウィンドウのデータを管理するよ！
/// DirectX などの描画詳細には一切依存しないピュアなデータ層。
pub struct GroupModel {
    pub id: String,
    pub title: String,
    pub bg_color_hex: String,
    pub opacity: f32, // 0.0 ~ 1.0
    pub icon_size: f32,
    pub icons: Vec<IconState>,
    pub hovered_index: Option<usize>,
    pub executing_index: Option<usize>, // 一瞬だけ光らせるための状態
}

#[derive(Clone)]
pub struct IconState {
    pub name: String,
    pub path: PathBuf,
}

impl GroupModel {
    pub fn new(
        id: String,
        title: String,
        bg_color_hex: String,
        opacity: f32,
        icon_size: f32,
        initial_icons: Vec<PathBuf>,
    ) -> Self {
        let icons = initial_icons
            .into_iter()
            .map(|path| {
                let name = path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                IconState { name, path }
            })
            .collect();

        Self {
            id,
            title,
            bg_color_hex,
            opacity,
            icon_size,
            icons,
            hovered_index: None,
            executing_index: None,
        }
    }
}
