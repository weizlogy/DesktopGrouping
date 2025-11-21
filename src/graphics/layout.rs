use ab_glyph::{Font, GlyphId, PxScale, ScaleFont};
use tiny_skia::{Color, Rect};

// --- ベースとなるレイアウト定数だよ！ ---
// これらが scale_factor で拡大縮小されるんだ♪
pub const BASE_PADDING: f32 = 10.0;
pub const BASE_LAYOUT_ICON_SIZE: f32 = 48.0;
pub const BASE_PADDING_UNDER_ICON: f32 = 2.0;
pub const BASE_TEXT_HEIGHT: f32 = 16.0;
pub const BASE_TEXT_FONT_SIZE: f32 = 16.0;
pub const BASE_ADJUST_SELECT_RECT: f32 = 3.0;

/// 指定されたテキストが、特定のフォントとスケールで描画された場合に、
/// どれくらいの幅になるかを計算するよ！
/// カーニング（文字と文字の間のアキ）もちゃんと考慮してるんだ。えらい！
pub fn calculate_text_width(text: &str, font: &impl Font, scale: PxScale) -> f32 {
    let scaled_font = font.as_scaled(scale);
    let mut total_width = 0.0;
    let mut last_glyph_id: Option<GlyphId> = None;

    for c in text.chars() {
        let glyph = scaled_font.scaled_glyph(c);
        if glyph.id.0 == 0 {
            continue;
        } // 未定義グリフはスキップ

        // カーニングを考慮 (前のグリフがあれば)
        if let Some(last_id) = last_glyph_id {
            total_width += scaled_font.kern(last_id, glyph.id);
        }
        total_width += scaled_font.h_advance(glyph.id);
        last_glyph_id = Some(glyph.id);
    }
    total_width
}

/// MyGraphics 内部で使うレイアウト計算だよ！スケーリング済みの値を使って計算するんだ。
pub fn calculate_internal_layout(
    window_width: u32,
    layout_icon_size: f32,
    padding: f32,
    padding_under_icon: f32,
    text_height: f32,
) -> (usize, f32, f32, f32) {
    // layout_icon_size や padding は、もうスケーリングされた値だよ！
    let max_text_width = layout_icon_size * 2.0; // アイコンサイズの2倍をテキストの最大幅に
    let item_width = max_text_width + padding; // 1アイテムの幅 = テキスト幅 + 右の余白
    // 1アイテムの高さ = アイコン高さ + アイコンと文字の間の余白 + 文字の高さ + 下の余白
    let item_height = layout_icon_size + padding_under_icon + text_height + padding;

    // 1行に何個アイテムを置けるかな？
    let items_per_row = if item_width > 0.0 {
        // (ウィンドウの幅 - 左の余白) / 1アイテムの幅 で計算して、小数点以下は切り捨て！最低でも1個は表示するよ！
        ((window_width as f32 - padding) / item_width)
            .floor()
            .max(1.0) as usize
    } else {
        1 // item_width が0になることはないはずだけど、念のため！
    };
    (items_per_row, max_text_width, item_width, item_height)
}

/// 指定されたインデックスのアイテム全体（アイコン、テキスト、パディング）が
/// 描画される矩形領域（相対座標、f32）を計算します。
///
/// マウスカーソルがどのアイコンの上にあるか判定する時 (ホバー判定) とかに使うよ！
/// `ADJUST_SELECT_RECT` を使って、選択範囲の見た目をちょっと調整してるんだ。
pub fn get_item_rect_f32(
    index: usize,
    width: u32,
    height: u32,
    items_per_row: usize,
    item_width: f32,
    item_height: f32,
    padding: f32,
    adjust_select_rect: f32,
) -> Option<Rect> {
    // 幅か高さが0、または items_per_row が0なら計算不能
    if width == 0 || height == 0 || items_per_row == 0 {
        return None;
    }
    let col = index % items_per_row;
    let row = index / items_per_row;

    // グリッドの左上の X 座標
    // item_width と padding は既にスケーリング済みだよ！
    let grid_x = (col as f32 * item_width) + padding;
    // グリッドの左上の Y 座標
    let grid_y = (row as f32 * item_height) + padding;

    // adjust_select_rect もスケーリング済みの値を使うよ！
    let adjusted_y = grid_y - adjust_select_rect;
    // 選択範囲の矩形の高さを計算するよ。
    // コンテンツ（アイコン＋テキスト）の高さに、上下のパディングとして adjust_select_rect を加えるんだ。
    // これで、上下に均等な余白ができて、見た目が良くなるはず！
    let adjusted_height = (item_height - padding) + (adjust_select_rect * 2.0);

    // アイテム全体の矩形を作成 (item_width, item_height を使用)
    let rect = Rect::from_xywh(grid_x, adjusted_y, item_width - padding, adjusted_height); // 右と下のパディングを除く範囲

    rect // intersect は Option<Rect> を返すので、そのまま返す
}

/// 背景色を受け取り、コントラスト比が高い（＝見やすい）テキスト色（黒または白）を返します。
///
/// WCAG (Web Content Accessibility Guidelines) で定義されている輝度比の計算式を
/// もとにして、背景色が明るいか暗いかを判定しています。
///
/// # Arguments
///
/// * `bg_color` - 背景色を表す `tiny_skia::Color`。
///
/// # Returns
///
/// * `tiny_skia::Color` - 背景色に対して見やすいテキスト色（黒または白）。
pub fn get_contrasting_text_color(bg_color: Color) -> Color {
    // sRGBの色成分を[0, 1]の範囲の線形値に変換します。
    // tiny_skia::Color の .red() などは既に [0.0, 1.0] の f32 を返すので、それを使います。
    // See: https://www.w3.org/TR/WCAG20-TECHS/G17.html#G17-procedure
    let srgb_to_linear = |c_srgb: f32| {
        if c_srgb <= 0.03928 {
            c_srgb / 12.92
        } else {
            ((c_srgb + 0.055) / 1.055).powf(2.4)
        }
    };

    let r_linear = srgb_to_linear(bg_color.red());
    let g_linear = srgb_to_linear(bg_color.green());
    let b_linear = srgb_to_linear(bg_color.blue());

    // 相対輝度 (Luminance) を計算します。
    let luminance = 0.2126 * r_linear + 0.7152 * g_linear + 0.0722 * b_linear;

    // 輝度のしきい値に基づいて、適切なテキスト色を返します。
    if luminance > 0.22 {
        Color::from_rgba8(0, 0, 0, 255) // 明るい背景には黒いテキスト
    } else {
        Color::from_rgba8(255, 255, 255, 255) // 暗い背景には白いテキスト
    }
}
