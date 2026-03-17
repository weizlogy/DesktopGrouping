use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

/// アイコン1つあたりのレイアウト情報だよ！
pub struct ItemLayout {
    pub icon_rect: D2D_RECT_F,
    pub text_rect: D2D_RECT_F,
    pub hit_rect: D2D_RECT_F, // ホバー判定やドラッグ開始判定に使うよ
}

pub const PADDING: f32 = 4.0;
pub const TEXT_HEIGHT_RATIO: f32 = 0.4; // アイコンサイズに対するテキスト高さの比率

/// グリッド配置（リフロー対応）を計算するよ！
/// window_width に合わせて列数を自動調整するんだ。
pub fn calculate_grid_layout(
    window_width: f32,
    item_count: usize,
    icon_size: f32,
    _scale_factor: f32, // 将来的に DPI スケーリングに対応するための予約
) -> Vec<ItemLayout> {
    let mut layouts = Vec::with_capacity(item_count);
    
    // アイコンサイズに基づいてセルサイズを決定するよ
    let cell_width = icon_size + 42.0; // 左右に余白を持たせる
    let text_height = 20.0; // 固定、または icon_size に比例させる
    let cell_height = icon_size + text_height + 16.0;

    // 1列に何個入るか計算 (最低1列)
    let cols = ((window_width - PADDING) / cell_width).floor().max(1.0) as usize;
    
    for i in 0..item_count {
        let col = i % cols;
        let row = i / cols;
        
        let x = PADDING + col as f32 * cell_width;
        let y = PADDING + row as f32 * cell_height;
        
        // アイコンの矩形 (セル内中央上部)
        let icon_x = x + (cell_width - icon_size) / 2.0;
        let icon_y = y + 4.0;
        let icon_rect = D2D_RECT_F {
            left: icon_x,
            top: icon_y,
            right: icon_x + icon_size,
            bottom: icon_y + icon_size,
        };
        
        // テキストの矩形 (アイコンの下)
        let text_rect = D2D_RECT_F {
            left: x + 2.0,
            top: icon_rect.bottom + 2.0,
            right: x + cell_width - 2.0,
            bottom: icon_rect.bottom + 2.0 + text_height,
        };
        
        // ヒットテスト用の矩形 (セル全体)
        let hit_rect = D2D_RECT_F {
            left: x,
            top: y,
            right: x + cell_width,
            bottom: y + cell_height,
        };
        
        layouts.push(ItemLayout {
            icon_rect,
            text_rect,
            hit_rect,
        });
    }
    
    layouts
}

/// 背景色から見やすいテキスト色を選択するための輝度計算
pub fn is_dark_color(r: f32, g: f32, b: f32) -> bool {
    // 相対輝度を計算 (WCAG)
    let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    luminance < 0.5
}
