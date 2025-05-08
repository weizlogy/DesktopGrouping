use std::{num::NonZeroU32, rc::Rc};

use ab_glyph::{point, Font, FontRef, GlyphId, PxScale, ScaleFont};
use softbuffer::{Context, Surface as SoftSurface};

use tiny_skia::{Color, Paint, PathBuilder, Pixmap, PixmapPaint, PremultipliedColorU8, Rect, Shader, Stroke, Transform};
use windows::Win32::Graphics::Gdi::{BITMAPINFO, BI_RGB};
use winit::{dpi::PhysicalSize, window::Window};

// --- レイアウト定数 (MyGraphics 構造体外で定義しても良い) ---
const PADDING: f32 = 10.0;
const LAYOUT_ICON_SIZE: f32 = 48.0; // レイアウト計算用の基準アイコンサイズ
const PADDING_UNDER_ICON: f32 = 12.0;
const TEXT_HEIGHT: f32 = 16.0; // テキスト描画領域の高さ (フォントサイズとは別)
const TEXT_FONT_SIZE: f32 = 16.0; // 実際に描画するフォントのサイズ
const ADJUST_SELECT_RECT: f32 = 3.0; // ホバー時の選択矩形の調整値
// --- 枠線定数 ---
const BORDER_WIDTH: f32 = 2.0; // 枠線の太さ (例: 1px)
// ---------------------------------------------------------
// --- 透過度定数 ---
const MIN_ALPHA: f32 = 0.05; // 透過度の下限値
// ---------------------------------------------------------

pub struct MyGraphics {
  soft_surface: SoftSurface<Rc<Window>, Rc<Window>>,
  pixmap: Pixmap,
  // --- レイアウト情報 ---
  width: u32,
  height: u32,
  items_per_row: usize,
  max_text_width: f32, // アイコン幅の2倍など、テキストの最大許容幅
  item_width: f32,     // グリッドアイテムの幅 (max_text_width + padding)
  item_height: f32,    // グリッドアイテムの高さ (icon_size + text_height + padding)
  // --- フォント ---
  font: FontRef<'static>, // フォントデータを保持
  background_paint: Paint<'static>, // 背景色用 Paint
  border_paint: Paint<'static>,   // 枠線色用 Paint
  border_stroke: Stroke,          // 枠線の太さなど
}

pub fn parse_color(color_str: &str) -> Option<Color> {
  let color_str = color_str.strip_prefix('#')?; // '#' を除去
  let (r_str, g_str, b_str, a_str) = match color_str.len() {
    6 => (
      color_str.get(0..2)?,
      color_str.get(2..4)?,
      color_str.get(4..6)?,
      "FF", // Alpha を FF (不透明) とする
    ),
    8 => (
      color_str.get(0..2)?,
      color_str.get(2..4)?,
      color_str.get(4..6)?,
      color_str.get(6..8)?,
    ),
    _ => return None, // 6桁でも8桁でもなければ無効
  };
  let r = u8::from_str_radix(r_str, 16).ok()?;
  let g = u8::from_str_radix(g_str, 16).ok()?;
  let b = u8::from_str_radix(b_str, 16).ok()?;
  let a = u8::from_str_radix(a_str, 16).ok()?;
  Color::from_rgba8(r, g, b, a).into() // tiny_skia::Color を返す
}

// テキスト幅計算ヘルパー関数
fn calculate_text_width(text: &str, font: &impl Font, scale: PxScale) -> f32 {
  let scaled_font = font.as_scaled(scale);
  let mut total_width = 0.0;
  let mut last_glyph_id: Option<GlyphId> = None;

  for c in text.chars() {
    let glyph = scaled_font.scaled_glyph(c);
    if glyph.id.0 == 0 { continue; } // 未定義グリフはスキップ

    // カーニングを考慮 (前のグリフがあれば)
    if let Some(last_id) = last_glyph_id {
      total_width += scaled_font.kern(last_id, glyph.id);
    }
    total_width += scaled_font.h_advance(glyph.id);
    last_glyph_id = Some(glyph.id);
  }
  total_width
}

/// ウィンドウ幅に基づいてレイアウト情報を計算する
fn calculate_layout(window_width: u32) -> (usize, f32, f32, f32) {
  let max_text_width = LAYOUT_ICON_SIZE * 2.0;
  let item_width = max_text_width + PADDING;
  let item_height = LAYOUT_ICON_SIZE + PADDING_UNDER_ICON + TEXT_HEIGHT + PADDING; // アイコン下パディング + テキスト高さ + 行間パディング

  // 1行あたりのアイテム数を計算
  let items_per_row = if item_width > 0.0 {
    ((window_width as f32 - PADDING) / item_width).floor().max(1.0) as usize // 最低1アイテムは表示
  } else {
    1 // item_width が 0 以下になることはないはずだが、念のため
  };
  (items_per_row, max_text_width, item_width, item_height)
}

impl MyGraphics {
  pub fn new(window: &Rc<Window>, bg_color_str: &str, border_color_str: &str) -> Self {
    let initial_size = window.inner_size();
    let width = initial_size.width;
    let height = initial_size.height;

    let context =
      Context::new(window.clone()).expect("Failed to create context");
    let mut soft_surface =
      SoftSurface::new(&context, window.clone()).expect("Failed to create surface");

    // resize を呼ぶ前に Pixmap を初期化
    let pixmap =
      Pixmap::new(width, height)
        .expect("Failed to create initial Pixmap");
    // soft_surface のリサイズも試みる
    soft_surface.resize(
      NonZeroU32::new(width).unwrap(),
      NonZeroU32::new(height).unwrap()
    ).expect("Failed to resize surface");

    // フォントをロードして保持
    let font_data = include_bytes!("../../resource/NotoSansJP-Medium.ttf");
    let font = FontRef::try_from_slice(font_data).expect("Failed to load font");

    // 初期レイアウト計算
    let (items_per_row, max_text_width, item_width, item_height) =
      calculate_layout(width);

    // 色をパース、失敗したらデフォルト色にフォールバック
    let bg_color =
      parse_color(bg_color_str).unwrap_or_else(
        || Color::from_rgba8(255, 255, 255, 153)); // Default: #FFFFFF99
    let border_color =
      parse_color(border_color_str).unwrap_or_else(
      || Color::from_rgba8(0, 0, 0, 255)); // Default: #000000FF

    let mut background_paint = Paint::default();
    background_paint.set_color(bg_color);
    background_paint.anti_alias = true; // お好みで

    let mut border_paint = Paint::default();
    border_paint.set_color(border_color);
    border_paint.anti_alias = true;

    let border_stroke = Stroke { width: 1.0, ..Default::default() }; // 枠線の太さなど

    return MyGraphics {
      soft_surface,
      pixmap,
      width,
      height,
      items_per_row,
      max_text_width,
      item_width,
      item_height,
      font, // フォントを保持
      background_paint,
      border_paint,
      border_stroke,
    };
  }

  /// 色のアルファ値を MIN_ALPHA にクランプするヘルパー関数
  fn clamp_alpha(mut color: Color) -> Color {
      let alpha = color.alpha();
      if alpha < MIN_ALPHA {
          // 元の色情報 (RGB) を保持しつつアルファ値だけ変更
          color = Color::from_rgba(color.red(), color.green(), color.blue(), MIN_ALPHA)
              .unwrap_or(color); // 失敗時は元の色を使う (ほぼありえない)
      }
      color
  }

  /// 背景色を更新します。
  pub fn update_background_color(&mut self, color: Color) {
    let clamped_color = Self::clamp_alpha(color);
    self.background_paint.set_color(clamped_color);
  }

  /// 枠線色を更新します。
  pub fn update_border_color(&mut self, color: Color) {
      let clamped_color = Self::clamp_alpha(color);
      self.border_paint.set_color(clamped_color);
  }

  /// 現在の背景色を取得します (透過度調整用)。
  pub fn get_background_color(&self) -> Color {
    // self.background_paint.color // <- これはエラーになる！
    // shader フィールドから色を取得する
    match self.background_paint.shader {
        // shader が SolidColor の場合、その中の color を返す
        Shader::SolidColor(color) => color,
        // SolidColor 以外は想定していないが、フォールバックとして透明色を返す
        _ => {
            // 本来ここに来ることはないはずなので警告ログを出す
            log::warn!("Background paint shader is not SolidColor!");
            Color::TRANSPARENT // または適切なデフォルト値
        }
    }
  }

  /// 現在の枠線色を取得します (設定保存用)。
  pub fn get_border_color(&self) -> Color {
    // self.border_paint.color // <- これもエラーになる！
    // shader フィールドから色を取得する
    match self.border_paint.shader {
        // shader が SolidColor の場合、その中の color を返す
        Shader::SolidColor(color) => color,
        // SolidColor 以外は想定していないが、フォールバックとして黒色を返す
        _ => {
            // 本来ここに来ることはないはずなので警告ログを出す
            log::warn!("Border paint shader is not SolidColor!");
            Color::BLACK // または適切なデフォルト値
        }
    }
  }

  pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
    self.width = new_size.width;
    self.height = new_size.height;

    self.soft_surface.resize(
      NonZeroU32::new(self.width).unwrap(),
      NonZeroU32::new(self.height).unwrap()).expect("Failed to resize surface");
    self.pixmap = Pixmap::new(self.width, self.height)
      .expect("Failed to create initial Pixmap");

    // レイアウト情報を再計算
    let (items_per_row, max_text_width, item_width, item_height) =
      calculate_layout(self.width);
    self.items_per_row = items_per_row;
    self.max_text_width = max_text_width;
    self.item_width = item_width;
    self.item_height = item_height;
  }

  pub fn draw_start(&mut self) {
    // Pixmap 全体を完全に透明な色でクリアする
    // これにより、前回の描画の影響 (特に draw_icon/draw_text による状態破壊の可能性) をリセットする
    self.pixmap.fill(Color::TRANSPARENT);
    // 背景色でクリア (MyGraphics::new で設定した background_paint を使う)
    let rect = Rect::from_xywh(0.0, 0.0, self.width as f32, self.height as f32).unwrap();
    // PixmapMut を取得 (draw_finish から移動 or 再取得)
    // let mut buffer =
    //   self.soft_surface.buffer_mut().expect("Failed to get buffer for draw_start");
    // PixmapMut を作成 (draw_finish と共有するか、ここで作成するか検討)
    // ここでは draw_finish でバッファを取得する前提で、pixmap への描画のみ行う
    // let mut pixmap_mut = PixmapMut::from_bytes(buffer.as_mut(), self.width, self.height).unwrap();

    // 背景描画
    self.pixmap.fill_rect(rect, &self.background_paint, Transform::identity(), None);

    // --- 枠線描画 (MyGraphics::new で設定した border_paint と border_stroke を使う) ---
    // stroke_rect は中心線で描画されるため、半分の太さだけ内側にオフセットする
    let border_half_width = self.border_stroke.width / 2.0;
    let border_rect = Rect::from_xywh(
      border_half_width,
      border_half_width,
      (self.width as f32 - self.border_stroke.width).max(0.0), // 幅が負にならないように
      (self.height as f32 - self.border_stroke.width).max(0.0), // 高さが負にならないように
    );

    if let Some(valid_border_rect) = border_rect { // Rect 作成が成功した場合のみ描画
      let path = PathBuilder::from_rect(valid_border_rect);
      self.pixmap.stroke_path(
        &path, // PathBuilder::from_rect が返すのは Path なので &path で参照を渡す
        &self.border_paint, // 構造体フィールドの border_paint を使用
        &self.border_stroke, // 構造体フィールドの border_stroke を使用
        Transform::identity(),
        None,
      );
    }
    // ---------------------------------------------------------
  }

  /// グループアイコンを描画します。
  ///
  /// # 引数
  /// * `index` - グループリスト内でのインデックス (描画位置計算用)。
  /// * `name` - アイコン名 (現在は未使用、将来的に表示するかも)。
  /// * `icon_data` - アイコンのビットマップ情報とピクセルデータ。
  /// * `is_hovered` - このアイコンが現在マウスホバーされているか。
  pub fn draw_group(
    &mut self,
    index: usize,
    icon_name: &String,
    icon_data: &(BITMAPINFO, Vec<u8>),
    is_hovered: bool) {

    let header = &icon_data.0.bmiHeader;
    // アイコンサイズは取得したものを尊重するが、レイアウト基準は 48 とする
    let icon_width = header.biWidth.abs() as u32;
    let icon_height = header.biHeight.abs() as u32;

    if icon_width == 0 || icon_height == 0 {
      return;
    }

    // --- グリッドと描画座標の計算 ---
    let col = index % self.items_per_row;
    let row = index / self.items_per_row;

    // グリッドの左上の X 座標 (テキスト描画の基準)
    let grid_x = (col as f32 * self.item_width) + PADDING;
    // グリッドの左上の Y 座標 (アイコン描画の基準)
    let grid_y = (row as f32 * self.item_height) + PADDING;

    // アイコンの描画座標 (テキストの中央に配置)
    // テキストが省略される可能性があるので、max_text_width を基準にする
    let icon_draw_x = grid_x + (self.max_text_width / 2.0) - (icon_width as f32 / 2.0);
    let icon_draw_y = grid_y;

    // テキストの描画座標
    let text_draw_x = grid_x;
    let text_draw_y = grid_y + LAYOUT_ICON_SIZE + PADDING_UNDER_ICON; // アイコンの下

    // --- ホバー状態の背景描画 ---
    if is_hovered {
      if let Some(rect) = self.get_item_rect_f32(index) { // f32 版の矩形取得を使用
        // 1. 背景塗りつぶし (明るい青 - Azur 半透明)
        let mut fill_paint = Paint::default();
        fill_paint.set_color_rgba8(0xF0, 0xFF, 0xFF, 0x40); // Azur
        fill_paint.anti_alias = true; // アンチエイリアス有効
        self.pixmap.fill_rect(rect, &fill_paint, Transform::identity(), None);

        // 2. 枠線描画 (濃い青 - SteelBlue 不透明)
        let mut stroke_paint = Paint::default();
        stroke_paint.set_color_rgba8(0x46, 0x82, 0xB4, 0x80); // SteelBlue
        stroke_paint.anti_alias = true; // アンチエイリアス有効

        // 枠線の設定
        let stroke = Stroke {
          width: BORDER_WIDTH,
          ..Default::default() // 他のプロパティはデフォルト値
        };

        // 矩形からパスを作成して枠線を描画
        self.pixmap.stroke_path(
          &PathBuilder::from_rect(rect), &stroke_paint, &stroke, Transform::identity(), None);
      }
    }

    // アイコンを描画
    // 取得したアイコンが 48x48 でない場合、ここでリサイズ処理を追加することも可能
    // (今回は get_file_icon で 48x48 取得を試みているので、そのまま描画)
    self.draw_icon(&icon_data.0, &icon_data.1, icon_draw_x as u32, icon_draw_y as u32);

    // テキストを描画 (最大幅を指定)
    self.draw_text(&icon_name, text_draw_x, text_draw_y, self.max_text_width);
  }

  fn draw_icon(&mut self, icon_info: &BITMAPINFO, pixel_data: &[u8], x: u32, y: u32) {
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
      self.draw_placeholder_icon(x, y, width.max(1), height.max(1));
      return;
    }

    // --- アイコン用の一時的な Pixmap を作成 ---
    let mut icon_pixmap = match Pixmap::new(width, height) {
      Some(pm) => pm,
      None => {
        self.draw_placeholder_icon(x, y, width.max(1), height.max(1));
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
      self.draw_placeholder_icon(x, y, width.max(1), height.max(1));
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
    self.pixmap.draw_pixmap(
      x as i32,
      y as i32,
      icon_pixmap.as_ref(), // icon_pixmap を参照として渡す
      &paint, // カスタマイズした paint を使用
      Transform::identity(),
      None,
    );
  }

  /// アイコン描画失敗時のプレースホルダーを描画するヘルパー関数
  fn draw_placeholder_icon(&mut self, x: u32, y: u32, width: u32, height: u32) {
    // Rect::from_xywh は失敗する可能性があるため unwrap_or_else で代替値を設定
    let rect = Rect::from_xywh(x as f32, y as f32, width as f32, height as f32)
      .unwrap_or_else(|| {
        Rect::from_xywh(x as f32, y as f32, 1.0, 1.0).unwrap() // 最小サイズ保証
      });
    let mut paint = Paint::default();
    // 少し目立つ色に変更 (例: 半透明の赤)
    paint.set_color_rgba8(0xFF, 0x00, 0x00, 0xAA);
    paint.anti_alias = true; // プレースホルダーも滑らかに
    self.pixmap.fill_rect(rect, &paint, Transform::identity(), None);
  }

  fn draw_text(&mut self, text: &str, startx: f32, starty: f32, max_width: f32) {
    // フィールドからフォントを使用
    let font = &self.font;
    let scale = PxScale::from(TEXT_FONT_SIZE); // 定数を使用
    let scaled_font = font.as_scaled(scale);

    // --- 省略表示処理 ---
    let ellipsis = "...";
    let ellipsis_width = calculate_text_width(ellipsis, &font, scale);
    let mut text_to_draw = text.to_string(); // 描画するテキスト (可変)

    let original_text_width = calculate_text_width(text, &font, scale);
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
      final_text_width = calculate_text_width(&text_to_draw, &font, scale);
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
        self.pixmap.draw_pixmap(
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

  pub fn draw_finish(&mut self) {
    let mut buffer =
      self.soft_surface.buffer_mut().expect("Failed to get buffer");

    // Pixmap のデータ (RGBA) を softbuffer のバッファ (ARGB or XRGB) にコピー
    // softbuffer はターゲットプラットフォームに応じて最適なフォーマットを選択する
    // ここでは一般的な BGRA (Windows) を想定してコピーするのではなく、
    // RGBA -> u32 (0xAARRGGBB or 0xFFRRGGBB) への変換を行う
    let pixmap_data = self.pixmap.data();
    for (i, pixel) in buffer.iter_mut().enumerate() {
      let r = pixmap_data[i * 4 + 0];
      let g = pixmap_data[i * 4 + 1];
      let b = pixmap_data[i * 4 + 2];
      let a = pixmap_data[i * 4 + 3]; // アルファ値も取得

      // Premultiplied Alpha を考慮しない単純な変換 (必要なら調整)
      // softbuffer が期待する形式 (例: 0xAARRGGBB) に合わせる
      // 多くの場合、ホストのネイティブエンディアンに依存する
      // ここでは一般的なリトルエンディアンの ARGB (0xAARRGGBB) を想定
      *pixel = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
      // もし XRGB (アルファ無視) なら:
      // *pixel = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    }

    buffer.present().expect("Failed to commit surface");
  }

  /// 指定されたインデックスのアイテム全体（アイコン、テキスト、パディング）が
  /// 描画される矩形領域（相対座標、f32）を計算します。
  /// ホバー判定などに使用します。Y座標と高さを調整済み。
  pub fn get_item_rect_f32(&self, index: usize) -> Option<Rect> {
    // 幅か高さが0、または items_per_row が0なら計算不能
    if self.width == 0 || self.height == 0 || self.items_per_row == 0 {
      return None;
    }

    let col = index % self.items_per_row;
    let row = index / self.items_per_row;

    // グリッドの左上の X 座標
    let grid_x = (col as f32 * self.item_width) + PADDING;
    // グリッドの左上の Y 座標
    let grid_y = (row as f32 * self.item_height) + PADDING;

    let adjusted_y = grid_y - ADJUST_SELECT_RECT;
    let adjusted_height = (self.item_height - PADDING) - ADJUST_SELECT_RECT;

    // アイテム全体の矩形を作成 (item_width, item_height を使用)
    let rect =
      Rect::from_xywh(grid_x, adjusted_y, self.item_width - PADDING, adjusted_height); // 右と下のパディングを除く範囲

    rect // intersect は Option<Rect> を返すので、そのまま返す
  }

}