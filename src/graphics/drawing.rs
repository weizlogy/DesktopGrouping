use ab_glyph::{point, Font, FontRef, GlyphId, PxScale, ScaleFont};
use tiny_skia::{
    Color, Paint, Pixmap, PixmapPaint, PremultipliedColorU8, Rect, Shader,
    Stroke, Transform,
};
use windows::Win32::Graphics::Gdi::{BITMAPINFO, BI_RGB};

use super::layout::calculate_text_width;

const SHADOW_OFFSET: f32 = 2.0;
const SHADOW_BLUR_RADIUS: f32 = 2.0;

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
/// もしアイコンデータが変だったり、サポートしてない形式だったら、代わりに赤い四角 (プレースホルダー) を描画するようになってるんだ。
pub fn draw_icon(pixmap: &mut Pixmap, icon_info: &BITMAPINFO, pixel_data: &[u8], x: u32, y: u32) {
    let header = &icon_info.bmiHeader;
    // biWidth は負の値の場合があるため絶対値を取る
    let width = header.biWidth.abs() as u32;
    // biHeight が負の場合はトップダウン DIB、正の場合はボトムアップ DIB
    let height = header.biHeight.abs() as u32;
    let is_top_down = header.biHeight < 0;
    let bpp = header.biBitCount; // Bits per pixel

    if width == 0 || height == 0 || (bpp != 32 && bpp != 24) || header.biCompression != BI_RGB.0 as u32 {
      // プレースホルダーを描画 (幅・高さが0の場合も考慮)
      draw_placeholder_icon(pixmap, x, y, width.max(1), height.max(1));
      return;
    }

    // --- アイコン用の一時的な Pixmap を作成 ---
    let mut icon_pixmap = match Pixmap::new(width, height) {
      Some(pm) => pm,
      None => {
        draw_placeholder_icon(pixmap, x, y, width.max(1), height.max(1));
        return;
      }
    };
    
    let bytes_per_pixel = (bpp / 8) as usize;
    let stride = ((width as usize * bytes_per_pixel + 3) & !3) as u32;
    let expected_data_size = (stride * height) as usize;

    if pixel_data.len() < expected_data_size {
      draw_placeholder_icon(pixmap, x, y, width.max(1), height.max(1));
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

        let b = src_pixel_bytes[0];
        let g = src_pixel_bytes[1];
        let r = src_pixel_bytes[2];
        let a = if bytes_per_pixel == 4 { src_pixel_bytes[3] } else { 255 };

        if let Some(color) = PremultipliedColorU8::from_rgba(r, g, b, a) {
          icon_pixmap_mut.pixels_mut()[ (y_dest * width + x_dest) as usize ] = color; // 直接インデックスアクセス
        }
      }
    }

    // --- ドロップシャドウの描画 --- (一旦コメントアウト)
    // if let Some(mut mask) = Mask::from_pixmap(icon_pixmap.as_ref(), None) {
    //     mask.blur(SHADOW_BLUR_RADIUS, None);
    //     let mut shadow_paint = Paint::default();
    //     shadow_paint.set_color_rgba8(0, 0, 0, 100); // 半透明の黒
    //     shadow_paint.anti_alias = true;
    //     pixmap.fill_mask(
    //         &mask,
    //         &shadow_paint,
    //         Transform::from_translate(x as f32 + SHADOW_OFFSET, y as f32 + SHADOW_OFFSET),
    //     );
    // }


    // --- アイコン本体の描画 ---
    let mut paint = PixmapPaint::default();
    paint.quality = tiny_skia::FilterQuality::Bicubic;
    pixmap.draw_pixmap(x as i32, y as i32, icon_pixmap.as_ref(), &paint, Transform::identity(), None);
}

/// 指定されたテキストを、いい感じに中央揃えして、ピクセルマップに描画するよ！
pub fn draw_text(
    pixmap: &mut Pixmap,
    font: &FontRef<'static>,
    text_font_size: f32,
    text: &str,
    startx: f32,
    starty: f32,
    max_width: f32,
) {
    let scale = PxScale::from(text_font_size);
    let scaled_font = font.as_scaled(scale);

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
                if glyph.id.0 == 0 { continue; }
                let mut char_width = scaled_font.h_advance(glyph.id);
                if let Some(last_id) = last_glyph_id {
                    char_width += scaled_font.kern(last_id, glyph.id);
                }
                if current_width + char_width > target_width { break; }
                current_width += char_width;
                last_glyph_id = Some(glyph.id);
                truncated_len = i + c.len_utf8();
            }
            text_to_draw = format!("{}{}", &text[..truncated_len], ellipsis);
        }
        final_text_width = calculate_text_width(&text_to_draw, font, scale);
    }

    let center_x = startx + max_width / 2.0;
    let adjusted_start_x = center_x - final_text_width / 2.0;

    let draw_text_inner = |target_pixmap: &mut Pixmap, text_to_draw: &str, pos: ab_glyph::Point, color: Color| {
        let mut caret = pos;
        let mut paint = Paint::default();
        paint.shader = tiny_skia::Shader::SolidColor(color);
        paint.anti_alias = true;

        for c in text_to_draw.chars() {
            let glyph = font.glyph_id(c).with_scale(scale);
            if let Some(outline) = font.outline_glyph(glyph) {
                outline.draw(|x, y, cov| {
                    let mut target_pixmap_mut = target_pixmap.as_mut(); // PixmapMut を取得
                    let idx = (caret.y as u32 + y) * target_pixmap_mut.width() + (caret.x as u32 + x); // インデックス計算
                    if idx < target_pixmap_mut.pixels_mut().len() as u32 {
                        let px = &mut target_pixmap_mut.pixels_mut()[idx as usize]; // 直接インデックスアクセス
                        let current_alpha = px.alpha();
                        let new_alpha = (cov * color.alpha() * 255.0) as u8;
                        
                        let blended_r = px.red() as f32 * (1.0 - cov) + color.red() * cov * 255.0;
                        let blended_g = px.green() as f32 * (1.0 - cov) + color.green() * cov * 255.0;
                        let blended_b = px.blue() as f32 * (1.0 - cov) + color.blue() * cov * 255.0;

                        *px = PremultipliedColorU8::from_rgba(
                            blended_r.min(255.0) as u8,
                            blended_g.min(255.0) as u8,
                            blended_b.min(255.0) as u8,
                            (current_alpha as f32 + new_alpha as f32 * (1.0 - current_alpha as f32 / 255.0)).min(255.0) as u8,
                        ).unwrap_or(*px);
                    }
                });
            }
            caret.x += scaled_font.h_advance(font.glyph_id(c));
        }
    };
    
    // 1. シャドウを描画 (元のコードに戻す)
    // let shadow_color = Color::from_rgba(0.0, 0.0, 0.0, 0.4).unwrap();
    // let shadow_pos = point(adjusted_start_x + SHADOW_OFFSET, starty + SHADOW_OFFSET);
    // draw_text_inner(pixmap, &text_to_draw, shadow_pos, shadow_color);

    // 2. テキスト本体を描画
    let text_color = Color::from_rgba(0.0, 0.0, 0.0, 1.0).unwrap();
    let text_pos = point(adjusted_start_x, starty);
    draw_text_inner(pixmap, &text_to_draw, text_pos, text_color);
}
