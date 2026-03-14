use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

/// アイコン1つあたりのレイアウト情報だよ！
pub struct ItemLayout {
    pub icon_rect: D2D_RECT_F,
    pub text_rect: D2D_RECT_F,
    pub hit_rect: D2D_RECT_F, // ホバー判定やドラッグ開始判定に使うよ
}

/// レイアウト定数 (スケーリングを考慮する前のベース値)
pub const ICON_SIZE: f32 = 48.0;
pub const CELL_WIDTH: f32 = 90.0;
pub const CELL_HEIGHT: f32 = 84.0; // 100.0 から縮小
pub const PADDING: f32 = 4.0;
pub const TEXT_HEIGHT: f32 = 20.0; // 28.0 から縮小

/// グリッド配置（リフロー対応）を計算するよ！
/// window_width に合わせて列数を自動調整するんだ。
pub fn calculate_grid_layout(
    window_width: f32,
    item_count: usize,
    _scale_factor: f32, // 将来的に DPI スケーリングに対応するための予約
) -> Vec<ItemLayout> {
    let mut layouts = Vec::with_capacity(item_count);
    
    // 1列に何個入るか計算 (最低1列)
    let cols = ((window_width - PADDING) / CELL_WIDTH).floor().max(1.0) as usize;
    
    for i in 0..item_count {
        let col = i % cols;
        let row = i / cols;
        
        let x = PADDING + col as f32 * CELL_WIDTH;
        let y = PADDING + row as f32 * CELL_HEIGHT;
        
        // アイコンの矩形 (セル内中央上部)
        let icon_x = x + (CELL_WIDTH - ICON_SIZE) / 2.0;
        let icon_y = y + 4.0;
        let icon_rect = D2D_RECT_F {
            left: icon_x,
            top: icon_y,
            right: icon_x + ICON_SIZE,
            bottom: icon_y + ICON_SIZE,
        };
        
        // テキストの矩形 (アイコンの下)
        let text_rect = D2D_RECT_F {
            left: x + 2.0,
            top: icon_rect.bottom + 2.0,
            right: x + CELL_WIDTH - 2.0,
            bottom: icon_rect.bottom + 2.0 + TEXT_HEIGHT,
        };
        
        // ヒットテスト用の矩形 (セル全体)
        let hit_rect = D2D_RECT_F {
            left: x,
            top: y,
            right: x + CELL_WIDTH,
            bottom: y + CELL_HEIGHT,
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
