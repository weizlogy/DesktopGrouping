use ab_glyph::{Font, FontRef, GlyphId, PxScale, ScaleFont, point};
use tiny_skia::{Paint, Pixmap, PixmapPaint, PremultipliedColorU8, Rect, Transform};
use windows::Win32::Graphics::Gdi::{BI_RGB, BITMAPINFO};

use super::layout::calculate_text_width;

/// アイコンの描画に失敗しちゃった時に、代わりに表示するプレースホルダー（仮の印）を描画するよ！
fn draw_placeholder_icon(pixmap: &mut Pixmap, x: u32, y: u32, width: u32, height: u32) {
    let rect = Rect::from_xywh(x as f32, y as f32, width as f32, height as f32)
        .unwrap_or_else(|| Rect::from_xywh(x as f32, y as f32, 1.0, 1.0).unwrap());
    let mut paint = Paint::default();
    paint.set_color_rgba8(0xFF, 0x00, 0x00, 0xAA);
    paint.anti_alias = true;
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

/// アイコンのビットマップデータをピクセルマップに描画するよ！
///
/// Windows の BITMAPINFO ヘッダー (`icon_info`) とピクセルデータ (`pixel_data`) をもらって、
/// それを解釈して `tiny_skia` が扱える形式に変換しながら、指定された座標 (`x`, `y`) に描画するんだ。
/// DIBフォーマットっていう、ちょっと昔ながらの形式を扱うから、色の並びとか、画像の上下が逆だったりするのに注意しながら処理してるよ！
/// もしアイコンデータが変だったり、サポートしてない形式だったら、代わりに `expected_size` で指定された大きさの
/// 赤い四角 (プレースホルダー) を描画するようになってるんだ。
pub fn draw_icon(
    pixmap: &mut Pixmap,
    icon_info: &BITMAPINFO,
    pixel_data: &[u8],
    x: u32,
    y: u32,
    expected_size: u32,
) {
    let header = &icon_info.bmiHeader;
    // biWidth は負の値の場合があるため絶対値を取る
    let width = header.biWidth.abs() as u32;
    // biHeight が負の場合はトップダウン DIB、正の場合はボトムアップ DIB
    let height = header.biHeight.abs() as u32;
    let is_top_down = header.biHeight < 0;
    let bpp = header.biBitCount; // Bits per pixel

    if width == 0
        || height == 0
        || (bpp != 32 && bpp != 24)
        || header.biCompression != BI_RGB.0 as u32
    {
        // プレースホルダーを描画
        draw_placeholder_icon(pixmap, x, y, expected_size, expected_size);
        return;
    }

    // --- アイコン用の一時的な Pixmap を作成 ---
    let mut icon_pixmap = match Pixmap::new(width, height) {
        Some(pm) => pm,
        None => {
            draw_placeholder_icon(pixmap, x, y, expected_size, expected_size);
            return;
        }
    };

    let bytes_per_pixel = (bpp / 8) as usize;
    let stride = ((width as usize * bytes_per_pixel + 3) & !3) as u32;
    let expected_data_size = (stride * height) as usize;

    if pixel_data.len() < expected_data_size {
        draw_placeholder_icon(pixmap, x, y, expected_size, expected_size);
        return;
    }

    let mut icon_pixmap_mut = icon_pixmap.as_mut(); // PixmapMut を取得
    for y_dest in 0..height {
        for x_dest in 0..width {
            let src_row_index = if is_top_down {
                y_dest
            } else {
                height - 1 - y_dest
            };
            let src_offset = (src_row_index * stride + x_dest * bytes_per_pixel as u32) as usize;

            if src_offset + bytes_per_pixel > pixel_data.len() {
                continue;
            }

            // スライスから直接読み取る
            let src_pixel_bytes = &pixel_data[src_offset..src_offset + bytes_per_pixel];

            let b_p = src_pixel_bytes[0];
            let g_p = src_pixel_bytes[1];
            let r_p = src_pixel_bytes[2];
            let a = if bytes_per_pixel == 4 {
                src_pixel_bytes[3]
            } else {
                255
            };

            // u16 に拡張して計算
            let r_p_u16 = r_p as u16;
            let g_p_u16 = g_p as u16;
            let b_p_u16 = b_p as u16;
            let a_u16 = a as u16;

            // アルファチャンネルが 0 の場合、除算を避ける
            let (r, g, b) = if a > 0 {
                (
                    (r_p_u16 * a_u16 / 255) as u8,
                    (g_p_u16 * a_u16 / 255) as u8,
                    (b_p_u16 * a_u16 / 255) as u8,
                )
            } else {
                (0, 0, 0) // 透明なピクセルの場合はすべてのチャンネルを 0 に
            };
            if let Some(color) = PremultipliedColorU8::from_rgba(r, g, b, a) {
                icon_pixmap_mut.pixels_mut()[(y_dest * width + x_dest) as usize] = color; // 直接インデックスアクセス
            }
        }
    }

    // --- アイコン本体の描画 ---
    let mut paint = PixmapPaint::default();
    paint.quality = tiny_skia::FilterQuality::Bicubic;
    pixmap.draw_pixmap(
        x as i32,
        y as i32,
        icon_pixmap.as_ref(),
        &paint,
        Transform::identity(),
        None,
    );
}

/// 指定されたテキストを、いい感じに中央揃えして、ピクセルマップに描画するよ！
///
/// `text` を `startx`, `starty` の位置を基準にして、`max_width` を超えないように描画するんだ。
/// もしテキストが長すぎて `max_width` に収まらなかったら、賢く「...」って省略してくれるよ！
/// `ab_glyph` を使って、フォントから文字の形（グリフ）を一つ一つ取り出して、それをピクセルマップに描き込んでいくんだ。
/// ちょっと複雑だけど、これで綺麗な文字が表示できるんだね！(<em>´ω｀</em>)
pub fn draw_text(
    pixmap: &mut Pixmap,
    font: &FontRef<'static>,
    text_font_size: f32,
    text: &str,
    startx: f32,
    starty: f32,
    max_width: f32,
    _text_height: f32, // text_height は今のところ使わないけど、将来のために残しておくよ！
) {
    let scale = PxScale::from(text_font_size);
    let scaled_font = font.as_scaled(scale);

    // --- 省略表示処理 ---
    let ellipsis = "...";
    let ellipsis_width = calculate_text_width(ellipsis, font, scale);
    let mut text_to_draw = text.to_string();
    let mut final_text_width = calculate_text_width(text, font, scale);

    if final_text_width > max_width {
        if max_width <= ellipsis_width {
            text_to_draw = ellipsis.to_string();
        } else {
            let target_width = max_width - ellipsis_width;
            let mut current_width = 0.0;
            let mut last_glyph_id: Option<GlyphId> = None;
            let mut truncated_len = 0;

            for (i, c) in text.char_indices() {
                let glyph = scaled_font.scaled_glyph(c);
                if glyph.id.0 == 0 {
                    continue;
                }
                let mut char_width = scaled_font.h_advance(glyph.id);
                if let Some(last_id) = last_glyph_id {
                    char_width += scaled_font.kern(last_id, glyph.id);
                }
                if current_width + char_width > target_width {
                    break;
                }
                current_width += char_width;
                last_glyph_id = Some(glyph.id);
                truncated_len = i + c.len_utf8();
            }
            text_to_draw = format!("{}{}", &text[..truncated_len], ellipsis);
        }
        final_text_width = calculate_text_width(&text_to_draw, font, scale);
    }

    // --- 描画開始位置の中央揃え計算 ---
    let center_x = startx + max_width / 2.0;
    let adjusted_start_x = center_x - final_text_width / 2.0;

    // --- 垂直位置の計算 ---
    // starty をベースラインとして扱うよ。
    // ascent() はベースラインから文字の上端までの距離。これを加えることで、文字の上端がだいたい starty に揃うようになるんだ。
    let baseline_y = starty + scaled_font.ascent();

    // --- グリフ描画時の設定を改善
    let mut paint = PixmapPaint::default();
    paint.quality = tiny_skia::FilterQuality::Bilinear;  // Bicubicだと時々過剰になるのでBilinearに
    paint.blend_mode = tiny_skia::BlendMode::SourceOver;  // アルファブレンディングの改善

    // --- 描画ループ ---
    let mut caret = point(adjusted_start_x, baseline_y);
    let mut last_glyph_id: Option<GlyphId> = None;

    for c in text_to_draw.chars() {
        let glyph_id = font.glyph_id(c);
        let glyph = glyph_id.with_scale(scale);

        // カーニングを適用
        if let Some(last_id) = last_glyph_id {
            caret.x += scaled_font.kern(last_id, glyph_id);
        }

        // グリフのアウトラインを取得
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
                caret.x += scaled_font.h_advance(glyph_id);
                last_glyph_id = Some(glyph_id);
                continue;
            }

            let glyph_width = bounds.width().ceil() as u32;
            let glyph_height = bounds.height().ceil() as u32;
            if glyph_width == 0 || glyph_height == 0 {
                caret.x += scaled_font.h_advance(glyph_id);
                last_glyph_id = Some(glyph_id);
                continue;
            }

            // グリフ用の一時的な Pixmap を作成 (初期状態は透明)
            let mut glyph_pixmap = match Pixmap::new(glyph_width, glyph_height) {
                Some(pm) => pm,
                None => {
                    // 作成失敗
                    caret.x += scaled_font.h_advance(glyph_id);
                    last_glyph_id = Some(glyph_id);
                    continue;
                }
            };

            // グリフのアウトラインを一時的な Pixmap に描画
            outline.draw(|dx, dy, coverage| {
                if coverage > 0.0 {
                    // 2. γ補正を少し緩めに
                    let gamma = 1.4;  // 1.8→1.4に調整
                    let final_alpha = coverage.powf(1.0 / gamma);
                    
                    // 3. 最終的な透明度をちょっと下げる
                    let opacity = 0.85;  // 不透明度を85%に
                    let a_u8 = ((final_alpha * 255.0 * opacity) as u8).min(255);

                    if let Some(color) = PremultipliedColorU8::from_rgba(0, 0, 0, a_u8) {
                        if let Some(pixel) = glyph_pixmap.pixels_mut().get_mut((dy * glyph_width + dx) as usize) {
                            *pixel = color;
                        }
                    }
                }
            });

            // 改善したpaintを使ってグリフを描画
            pixmap.draw_pixmap(
                (caret.x + bounds.min.x.ceil()) as i32,
                (caret.y + bounds.min.y.ceil()) as i32,
                glyph_pixmap.as_ref(),
                &paint,
                Transform::identity(),
                None,
            );
        }

        caret.x += scaled_font.h_advance(glyph_id);
        last_glyph_id = Some(glyph_id);
    }
}
