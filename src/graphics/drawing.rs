use ab_glyph::{point, Font, FontRef, GlyphId, PxScale, ScaleFont, self};
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, PixmapPaint, PremultipliedColorU8, Rect, Shader, Stroke, Transform};
use windows::Win32::Graphics::Gdi::{BITMAPINFO, BI_RGB};

use super::layout::calculate_text_width;

/// アイコンの描画に失敗しちゃった時に、代わりに表示するプレースホルダー（仮の印）を描画するよ！
///
/// 今は半透明の赤い四角を描画するようになってるんだ。これが出たら「あれれ？アイコンがうまく取れなかったのかな？」って分かるね！
fn draw_placeholder_icon(pixmap: &mut Pixmap, x: u32, y: u32, width: u32, height: u32) {
    // Rect::from_xywh は失敗する可能性があるため unwrap_or_else で代替値を設定
    let rect = Rect::from_xywh(x as f32, y as f32, width as f32, height as f32)
      .unwrap_or_else(|| {
        Rect::from_xywh(x as f32, y as f32, 1.0, 1.0).unwrap() // 最小サイズ保証
      });
    let mut paint = Paint::default();
    // 少し目立つ色に変更 (例: 半透明の赤)
    paint.set_color_rgba8(0xFF, 0x00, 0x00, 0xAA);
    paint.anti_alias = true; // プレースホルダーも滑らかに
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

    // println!(
    //   "Icon dimensions: width={}, height={}, bpp={}, compression={}.",
    //   width, height, bpp, header.biCompression
    // );

    // --- サポートするフォーマットかチェック ---
    // 幅か高さが0、またはサポート外の bpp、または非圧縮でない場合はプレースホルダーを描画
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
    // PixmapMut を使ってピクセルデータへの可変参照を取得
    let mut icon_pixmap_mut = icon_pixmap.as_mut();
    let icon_pixels = icon_pixmap_mut.pixels_mut(); // 可変ピクセルスライスを取得

    let bytes_per_pixel = (bpp / 8) as usize;
    let stride = ((width as usize * bytes_per_pixel + 3) & !3) as u32;
    let expected_data_size = (stride * height) as usize;

    // --- ピクセルデータサイズのチェック --- 
    if pixel_data.len() < expected_data_size {
      draw_placeholder_icon(pixmap, x, y, width.max(1), height.max(1));
      return;
    }

    // --- ピクセルデータを一時的な Pixmap にコピー＆変換 ---
    // unsafe ブロックを削除し、pixel_data スライスを使用
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
          let dest_index = (y_dest * width + x_dest) as usize;
          if let Some(pixel) = icon_pixels.get_mut(dest_index) {
            *pixel = color;
          }
        }
      }
    }

    // --- 作成した icon_pixmap をメインの pixmap に描画 ---
    // PixmapPaint::default() は BlendMode::SrcOver (アルファブレンディング) を使用する
    // 描画品質を向上させるために FilterQuality を変更する
    let mut paint = tiny_skia::PixmapPaint::default(); // PixmapPaint を作成
    paint.quality = tiny_skia::FilterQuality::Bicubic; // Bilinear (デフォルト) から Bicubic へ変更
    pixmap.draw_pixmap(
      x as i32,
      y as i32,
      icon_pixmap.as_ref(), // icon_pixmap を参照として渡す
      &paint, // カスタマイズした paint を使用
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
pub fn draw_text(pixmap: &mut Pixmap, font: &FontRef<'static>, text_font_size: f32, text: &str, startx: f32, starty: f32, max_width: f32) {
    // スケーリング済みのフォントサイズ (text_font_size) を使うよ！
    let scale = PxScale::from(text_font_size);
    let scaled_font = font.as_scaled(scale);

    // --- 省略表示処理 ---
    let ellipsis = "...";
    let ellipsis_width = calculate_text_width(ellipsis, font, scale);
    let mut text_to_draw = text.to_string(); // 描画するテキスト (可変)

    let original_text_width = calculate_text_width(text, font, scale);
    let mut final_text_width = original_text_width; // 最終的なテキスト幅

    if original_text_width > max_width {
      // 省略が必要
      if max_width <= ellipsis_width {
        // "..." すら入らない場合は "..." だけ表示 (あるいは空文字)
        text_to_draw = ellipsis.to_string();
      } else {
        let target_width = max_width - ellipsis_width;
        let mut current_width = 0.0;
        let mut last_glyph_id: Option<GlyphId> = None;
        let mut truncated_len = 0; // 切り詰める文字数

        for (i, c) in text.char_indices() {
          let glyph = scaled_font.scaled_glyph(c);
          if glyph.id.0 == 0 { continue; }

          let mut char_width = 0.0;
          if let Some(last_id) = last_glyph_id {
            char_width += scaled_font.kern(last_id, glyph.id);
          }
          char_width += scaled_font.h_advance(glyph.id);

          if current_width + char_width <= target_width {
            current_width += char_width;
            last_glyph_id = Some(glyph.id);
            // この文字のバイト長を取得して truncated_len を更新
            truncated_len = i + c.len_utf8();
          } else {
            // 幅を超えたのでここで打ち切る
            break;
          }
        }
        // text を truncated_len でスライスし、ellipsis を結合
        text_to_draw = format!("{}{}", &text[..truncated_len], ellipsis);
      }
      final_text_width = calculate_text_width(&text_to_draw, font, scale);
    }
    // --- 省略表示処理ここまで ---

    // --- 描画開始位置の中央揃え計算 ---
    let center_x = startx + max_width / 2.0;
    let adjusted_start_x = center_x - final_text_width / 2.0;
    // --- 計算ここまで ---

    // キャレット (文字描画開始位置) を設定
    let mut caret = point(adjusted_start_x as f32, starty as f32);

    for c in text_to_draw.chars() {
      // 文字に対応するグリフを取得 (スケール済み)
      let glyph = scaled_font.scaled_glyph(c);
      // グリフの ID を変数に格納
      let glyph_id = glyph.id;

      // println!("Char {}, Glyph ID: {:#?}", c, glyph_id); // デバッグ用

      // グリフのアウトラインを取得
      if let Some(outline) = scaled_font.outline_glyph(glyph) {
        // ピクセル単位のバウンディングボックスを取得
        let bounds = outline.px_bounds();
        // グリフのサイズが0以下の場合はスキップ (スペース文字など)
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
          // キャレットを水平方向に進める
          caret.x += scaled_font.h_advance(glyph_id);
          continue;
        }

        // グリフ用の一時的な Pixmap を正確なサイズで作成
        let glyph_width = bounds.width().ceil() as u32;
        let glyph_height = bounds.height().ceil() as u32;
        // Pixmap の寸法がゼロでないことを確認
        if glyph_width == 0 || glyph_height == 0 {
          caret.x += scaled_font.h_advance(glyph_id);
          continue;
        }

        // グリフ用 Pixmap を作成 (初期状態は透明)
        let mut glyph_pixmap = Pixmap::new(glyph_width, glyph_height).unwrap();
        // グリフ Pixmap の可変ピクセル スライスを取得
        let glyph_pixels = glyph_pixmap.pixels_mut();

        // グリフのアウトラインを一時的な Pixmap に描画
        outline.draw(|dx, dy, coverage| {
          // coverage (被覆率) が 0 より大きいピクセルのみ描画
          if coverage > 0.0 {
            // coverage に基づいてアルファ値を計算 (0-255)
            // 浮動小数点誤差を考慮し、上限を 255.0 にクランプしてから u8 にキャスト
            let alpha = (coverage * 255.0).min(255.0) as u8;

            // グリフ Pixmap スライス内のインデックスを計算
            // dx, dy はグリフのバウンディングボックスの原点からの相対座標
            let index = dy * glyph_width + dx;

            // 事前乗算アルファ色を安全に作成 (黒テキスト)
            // from_rgba は Option を返すため、if let で処理
            if let Some(color) = PremultipliedColorU8::from_rgba(0, 0, 0, alpha) {
              // スライス内のピクセルに安全にアクセスして色を割り当て
              if let Some(pixel) = glyph_pixels.get_mut(index as usize) {
                // glyph_pixmap は新規作成された透明な状態なので、単純に割り当てれば良い
                *pixel = color; // unwrap() を削除
              }
            }
          }
        });

        // 一時的なグリフ Pixmap をメインの Pixmap に描画
        // bounds.min を使用して描画位置を調整
        pixmap.draw_pixmap(
          (caret.x + bounds.min.x) as i32,
          // ベースライン (caret.y) からグリフの上端 (bounds.min.y) までのオフセットを加える
          (caret.y + bounds.min.y) as i32,
          glyph_pixmap.as_ref(), // glyph_pixmap を描画
          &PixmapPaint::default(), // デフォルトのペイントを使用 (ここではブレンド不要)
          Transform::identity(),
          None,
        );
      }
      // アウトラインがない場合 (スペースなど) でもキャレットを進める
      caret.x += scaled_font.h_advance(glyph_id);
    }
}
