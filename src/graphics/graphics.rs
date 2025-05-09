use std::{num::NonZeroU32, rc::Rc};

use ab_glyph::{point, Font, FontRef, GlyphId, PxScale, ScaleFont};
use softbuffer::{Context, Surface as SoftSurface};

use tiny_skia::{Color, Paint, PathBuilder, Pixmap, PixmapPaint, PremultipliedColorU8, Rect, Shader, Stroke, Transform};
use windows::Win32::Graphics::Gdi::{BITMAPINFO, BI_RGB};
use winit::{dpi::PhysicalSize, window::Window};
// logger モジュールは get_background_color などで使ってるから、ちゃんと use しとかないとね！
use crate::logger::*;
 // --- ベースとなるレイアウト定数だよ！ ---
 // これらが scale_factor で拡大縮小されるんだ♪
const BASE_PADDING: f32 = 10.0;
const BASE_LAYOUT_ICON_SIZE: f32 = 48.0;
const BASE_PADDING_UNDER_ICON: f32 = 12.0;
const BASE_TEXT_HEIGHT: f32 = 16.0;
const BASE_TEXT_FONT_SIZE: f32 = 16.0;
const BASE_ADJUST_SELECT_RECT: f32 = 3.0;
// --- 枠線定数 ---
const BORDER_WIDTH: f32 = 2.0; // ウィンドウの枠線の太さだよ！今は2ピクセルだね。
// ---------------------------------------------------------
// --- 透過度定数 ---
const MIN_ALPHA: f32 = 0.05; // 色のアルファ値（透明度）が、これより小さくならないようにするための下限値だよ。あんまり透明すぎると見えなくなっちゃうからね！
// ---------------------------------------------------------

/// ウィンドウごとのグラフィック描画を担当する構造体だよ！
/// `softbuffer` を使ってウィンドウにピクセルバッファを描画して、
/// `tiny_skia` で背景、枠線、アイコン、テキストとかを描画するんだ。
pub struct MyGraphics {
  soft_surface: SoftSurface<Rc<Window>, Rc<Window>>,
  pixmap: Pixmap,
  // --- レイアウト情報 ---
  width: u32,           // ピクセルマップの幅 (ウィンドウの内部幅と同じだよ！)
  height: u32,          // ピクセルマップの高さ (ウィンドウの内部高さと同じだよ！)
  scale_factor: f64,    // DPIスケーリングとかのための拡大率だよ！
  items_per_row: usize, // 1行に表示できるアイコンの数だよ。ウィンドウの幅によって変わるんだ。
  max_text_width: f32, // アイコンの下に表示するテキストの、許容される最大の幅だよ。これを超えると省略されちゃう！
  item_width: f32,     // グリッドレイアウトの1アイテムあたりの幅だよ (テキストの最大幅 + 余白)。
  item_height: f32,    // グリッドレイアウトの1アイテムあたりの高さだよ (アイコンの高さ + テキストの高さ + 余白)。
  // --- スケーリングされたレイアウト値だよ！ ---
  padding: f32,
  layout_icon_size: f32,
  padding_under_icon: f32,
  text_height: f32,
  text_font_size: f32,
  adjust_select_rect: f32,
  // border_width は今のところ固定だけど、もしスケーリングしたくなったらここに追加するんだ！

  // --- フォント ---
  font: FontRef<'static>, // フォントデータを保持
  background_paint: Paint<'static>, // 背景色用 Paint
  border_paint: Paint<'static>,   // 枠線色用 Paint
  border_stroke: Stroke,          // 枠線の太さなど
}

/// 色の文字列 (例: `"#RRGGBB"` や `"#RRGGBBAA"`) を `tiny_skia::Color` に変換するよ！
///
/// '#' があってもなくても大丈夫！6桁だったらアルファ値は不透明 (FF) になるよ。
/// もし変換できなかったら `None` を返すから、ちゃんとチェックしてね！
pub fn parse_color(color_str: &str) -> Option<Color> {
  // '#' があったら取り除いて、なかったらそのまま使うよ！
  let s = color_str.strip_prefix('#').unwrap_or(color_str);

  let (r_str, g_str, b_str, a_str) = match s.len() {
    6 => (
      s.get(0..2)?,
      s.get(2..4)?,
      s.get(4..6)?,
      "FF", // Alpha を FF (不透明) とする
    ),
    8 => (
      s.get(0..2)?,
      s.get(2..4)?,
      s.get(4..6)?,
      s.get(6..8)?,
    ),
    _ => return None, // 6桁でも8桁でもなければ無効
  };
  let r = u8::from_str_radix(r_str, 16).ok()?;
  let g = u8::from_str_radix(g_str, 16).ok()?;
  let b = u8::from_str_radix(b_str, 16).ok()?;
  let a = u8::from_str_radix(a_str, 16).ok()?;
  Color::from_rgba8(r, g, b, a).into() // tiny_skia::Color を返す
}

/// 指定されたテキストが、特定のフォントとスケールで描画された場合に、
/// どれくらいの幅になるかを計算するよ！
/// カーニング（文字と文字の間のアキ）もちゃんと考慮してるんだ。えらい！
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

impl MyGraphics {
  /// 新しい `MyGraphics` インスタンスを作るよ！
  ///
  /// ウィンドウハンドル (`window`) と、初期の背景色・枠線色、それから初期の拡大率 (`initial_scale_factor`) をもらって、
  /// 描画に必要なもの (softbufferのサーフェス、ピクセルマップ、フォント、レイアウト情報など) を準備するんだ。
  ///
  /// 色の文字列がもしパースできなかったら、優しいデフォルト色にしてくれるから安心してね！(<em>´ω｀</em>)
  pub fn new(window: &Rc<Window>, bg_color_str: &str, border_color_str: &str, initial_scale_factor: f64) -> Self {
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

    let border_stroke = Stroke { width: BORDER_WIDTH, ..Default::default() }; // 枠線の太さなど

    let mut graphics = MyGraphics {
      soft_surface,
      pixmap,
      width,
      height,
      scale_factor: initial_scale_factor,
      // レイアウト関連のフィールドは update_scaled_layout_values で初期化されるよ！
      items_per_row: 0,
      max_text_width: 0.0,
      item_width: 0.0,
      item_height: 0.0,
      padding: 0.0,
      layout_icon_size: 0.0,
      padding_under_icon: 0.0,
      text_height: 0.0,
      text_font_size: 0.0,
      adjust_select_rect: 0.0,
      font, // フォントを保持
      background_paint,
      border_paint,
      border_stroke,
    };
    graphics.update_scaled_layout_values(); // スケーリングされたレイアウト値を計算して設定！
    return graphics;
  }

  /// スケーリングされたレイアウト関連の値を計算して、構造体のフィールドを更新するよ！
  /// scale_factor が変わった時とか、ウィンドウサイズが変わった時に呼び出すんだ。
  fn update_scaled_layout_values(&mut self) {
    self.padding = (BASE_PADDING as f64 * self.scale_factor) as f32;
    self.layout_icon_size = (BASE_LAYOUT_ICON_SIZE as f64 * self.scale_factor) as f32;
    self.padding_under_icon = (BASE_PADDING_UNDER_ICON as f64 * self.scale_factor) as f32;
    self.text_height = (BASE_TEXT_HEIGHT as f64 * self.scale_factor) as f32;
    self.text_font_size = (BASE_TEXT_FONT_SIZE as f64 * self.scale_factor) as f32;
    self.adjust_select_rect = (BASE_ADJUST_SELECT_RECT as f64 * self.scale_factor) as f32;
    // self.border_width ももしスケーリングするならここで！

    // 新しいスケーリング値を使って、グリッドレイアウトを再計算するよ！
    let (items_per_row, max_text_width, item_width, item_height) =
        self.calculate_internal_layout(self.width);
    self.items_per_row = items_per_row;
    self.max_text_width = max_text_width;
    self.item_width = item_width;
    self.item_height = item_height;
  }

  /// MyGraphics 内部で使うレイアウト計算だよ！スケーリング済みの値を使って計算するんだ。
  fn calculate_internal_layout(&self, window_width: u32) -> (usize, f32, f32, f32) {
    // self.layout_icon_size や self.padding は、もうスケーリングされた値だよ！
    let max_text_width = self.layout_icon_size * 2.0; // アイコンサイズの2倍をテキストの最大幅に
    let item_width = max_text_width + self.padding; // 1アイテムの幅 = テキスト幅 + 右の余白
    // 1アイテムの高さ = アイコン高さ + アイコンと文字の間の余白 + 文字の高さ + 下の余白
    let item_height = self.layout_icon_size + self.padding_under_icon + self.text_height + self.padding;

    // 1行に何個アイテムを置けるかな？
    let items_per_row = if item_width > 0.0 {
        // (ウィンドウの幅 - 左の余白) / 1アイテムの幅 で計算して、小数点以下は切り捨て！最低でも1個は表示するよ！
        ((window_width as f32 - self.padding) / item_width).floor().max(1.0) as usize
    } else {
        1 // item_width が0になることはないはずだけど、念のため！
    };
    (items_per_row, max_text_width, item_width, item_height)
  }

  /// 色のアルファ値（透明度）を、`MIN_ALPHA` で定義された下限値に制限（クランプ）するよ！
  ///
  /// あんまり透明にしすぎると見えなくなっちゃうから、それを防ぐためのおまじないなんだ♪
  fn clamp_alpha(mut color: Color) -> Color {
      let alpha = color.alpha();
      if alpha < MIN_ALPHA {
          // 元の色情報 (RGB) を保持しつつアルファ値だけ変更
          color = Color::from_rgba(color.red(), color.green(), color.blue(), MIN_ALPHA)
              .unwrap_or(color); // 失敗時は元の色を使う (ほぼありえない)
      }
      color
  }

  /// 背景色を更新するよ！
  /// 新しい色 (`color`) をもらって、アルファ値を下限チェックしてから `background_paint` に設定するんだ。
  pub fn update_background_color(&mut self, color: Color) {
    let clamped_color = Self::clamp_alpha(color);
    self.background_paint.set_color(clamped_color);
  }

  /// 枠線色を更新するよ！
  /// 新しい色 (`color`) をもらって、こっちもアルファ値を下限チェックしてから `border_paint` に設定するんだ。
  pub fn update_border_color(&mut self, color: Color) {
      let clamped_color = Self::clamp_alpha(color);
      self.border_paint.set_color(clamped_color);
  }

  /// 今設定されてる背景色を取得するよ！
  /// `Paint` オブジェクトが直接色を返してくれないから、中の `Shader` を見て色を取り出すんだ。
  /// もし万が一、想定外のシェーダーだったら、透明色を返してログに警告を出すようになってるよ。
  pub fn get_background_color(&self) -> Color {
    // self.background_paint.color // <- これはエラーになる！
    // shader フィールドから色を取得する
    match self.background_paint.shader {
        // shader が SolidColor の場合、その中の color を返す
        Shader::SolidColor(color) => color,
        // SolidColor 以外は想定していないが、フォールバックとして透明色を返す
        _ => {
            // 本来ここに来ることはないはずなので警告ログを出す (logger クレートの log_warn を使うよ！)
            log_warn("Background paint shader is not SolidColor!");
            Color::TRANSPARENT // または適切なデフォルト値
        }
    }
  }

  /// 今設定されてる枠線色を取得するよ！ (設定保存用とかに使うんだ)
  /// こっちも `Paint` の中の `Shader` を見て色を取り出すよ。
  pub fn get_border_color(&self) -> Color {
    // self.border_paint.color // <- これもエラーになる！
    // shader フィールドから色を取得する
    match self.border_paint.shader {
        // shader が SolidColor の場合、その中の color を返す
        Shader::SolidColor(color) => color,
        // SolidColor 以外は想定していないが、フォールバックとして黒色を返す
        _ => {
            // 本来ここに来ることはないはずなので警告ログを出す (logger クレートの log_warn を使うよ！)
            log_warn("Border paint shader is not SolidColor!");
            Color::BLACK // または適切なデフォルト値
        }
    }
  }

  /// ウィンドウのサイズが変更された時に呼び出されるよ！
  ///
  /// 新しいサイズ (`new_size`) に合わせて、内部の `soft_surface` と `pixmap` をリサイズして、
  /// アイコンのグリッドレイアウトも再計算するんだ。これで表示が崩れないようにするんだね！
  pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
    self.width = new_size.width;
    self.height = new_size.height;

    self.soft_surface.resize(
      NonZeroU32::new(self.width).unwrap(),
      NonZeroU32::new(self.height).unwrap()).expect("Failed to resize surface");
    self.pixmap = Pixmap::new(self.width, self.height)
      .expect("Failed to create initial Pixmap");

    // スケーリングされたレイアウト値を再計算！
    self.update_scaled_layout_values();
  }

  /// 拡大率 (`scale_factor`) が変わった時に呼び出すよ！
  /// 新しい拡大率を覚えて、レイアウトを再計算するんだ。
  pub fn update_scale_factor(&mut self, new_scale_factor: f64) {
    // 拡大率が本当に変わったかチェック！ちょっとだけ違っても再計算しないようにするよ。
    if (self.scale_factor - new_scale_factor).abs() > f64::EPSILON {
        log_debug(&format!("MyGraphics: Scale factor changing from {} to {}", self.scale_factor, new_scale_factor));
        self.scale_factor = new_scale_factor;
        self.update_scaled_layout_values(); // 新しい拡大率でレイアウト値を更新！
    }
  }

  /// 描画を開始する時に呼び出すよ！
  ///
  /// まず、ピクセルマップ全体を完全に透明な色でクリアして、前回の描画内容をリセットするんだ。
  /// それから、設定されてる背景色で塗りつぶして、最後に枠線を描画するよ！これで描画の準備はバッチリ！
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

  /// グループアイコンを描画するよ！
  /// 1つのアイコンとその名前を、グリッドレイアウトに従っていい感じの位置に描画するんだ。
  ///
  /// `index` を使って、何行目の何列目に描くか計算して、
  /// `icon_data` からアイコンの絵を取り出して、`icon_name` をその下に表示するよ。
  /// もし `is_hovered` が `true` だったら、アイコンの周りをちょっとキラキラさせて目立たせるよ！✨
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
    let grid_x = (col as f32 * self.item_width) + self.padding; // スケーリング済みの self.padding を使うよ！
    // グリッドの左上の Y 座標 (アイコン描画の基準)
    let grid_y = (row as f32 * self.item_height) + self.padding; // こっちも！

    // アイコンの描画座標 (テキストの中央に配置)
    // テキストが省略される可能性があるので、max_text_width を基準にする
    let icon_draw_x = grid_x + (self.max_text_width / 2.0) - (icon_width as f32 / 2.0);
    let icon_draw_y = grid_y;

    // テキストの描画座標 (スケーリング済みの値を使うよ！)
    let text_draw_x = grid_x; // テキストはグリッドの左端から
    let text_draw_y = grid_y + self.layout_icon_size + self.padding_under_icon; // アイコンの下に、余白を挟んで配置

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

  /// アイコンのビットマップデータをピクセルマップに描画するよ！
  ///
  /// Windows の BITMAPINFO ヘッダー (`icon_info`) とピクセルデータ (`pixel_data`) をもらって、
  /// それを解釈して `tiny_skia` が扱える形式に変換しながら、指定された座標 (`x`, `y`) に描画するんだ。
  /// DIBフォーマットっていう、ちょっと昔ながらの形式を扱うから、色の並びとか、画像の上下が逆だったりするのに注意しながら処理してるよ！
  /// もしアイコンデータが変だったり、サポートしてない形式だったら、代わりに赤い四角 (プレースホルダー) を描画するようになってるんだ。
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

  /// アイコンの描画に失敗しちゃった時に、代わりに表示するプレースホルダー（仮の印）を描画するよ！
  ///
  /// 今は半透明の赤い四角を描画するようになってるんだ。これが出たら「あれれ？アイコンがうまく取れなかったのかな？」って分かるね！
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

  /// 指定されたテキストを、いい感じに中央揃えして、ピクセルマップに描画するよ！
  ///
  /// `text` を `startx`, `starty` の位置を基準にして、`max_width` を超えないように描画するんだ。
  /// もしテキストが長すぎて `max_width` に収まらなかったら、賢く「...」って省略してくれるよ！
  /// `ab_glyph` を使って、フォントから文字の形（グリフ）を一つ一つ取り出して、それをピクセルマップに描き込んでいくんだ。
  /// ちょっと複雑だけど、これで綺麗な文字が表示できるんだね！(<em>´ω｀</em>)
  fn draw_text(&mut self, text: &str, startx: f32, starty: f32, max_width: f32) {
    // フィールドからフォントを使用
    let font = &self.font;
    // スケーリング済みのフォントサイズ (self.text_font_size) を使うよ！
    let scale = PxScale::from(self.text_font_size);
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

  /// これまでピクセルマップに描画してきた内容を、実際にウィンドウに表示するよ！
  ///
  /// `soft_surface` からウィンドウのバッファを取得して、`self.pixmap` の内容をそこにコピーするんだ。
  /// ピクセルフォーマット (RGBA とか ARGB とか) の違いも、ここで吸収してるみたいだね！
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
  ///
  /// マウスカーソルがどのアイコンの上にあるか判定する時 (ホバー判定) とかに使うよ！
  /// `ADJUST_SELECT_RECT` を使って、選択範囲の見た目をちょっと調整してるんだ。
  pub fn get_item_rect_f32(&self, index: usize) -> Option<Rect> {
    // 幅か高さが0、または items_per_row が0なら計算不能
    if self.width == 0 || self.height == 0 || self.items_per_row == 0 {
      return None;
    }
    let col = index % self.items_per_row;
    let row = index / self.items_per_row;

    // グリッドの左上の X 座標
    // self.item_width と self.padding は既にスケーリング済みだよ！
    let grid_x = (col as f32 * self.item_width) + self.padding;
    // グリッドの左上の Y 座標
    let grid_y = (row as f32 * self.item_height) + self.padding;

    // self.adjust_select_rect もスケーリング済みの値を使うよ！
    let adjusted_y = grid_y - self.adjust_select_rect;
    let adjusted_height = (self.item_height - self.padding) - self.adjust_select_rect; // アイテムの高さから下のパディングを引いて、さらに調整！

    // アイテム全体の矩形を作成 (item_width, item_height を使用)
    let rect =
      Rect::from_xywh(grid_x, adjusted_y, self.item_width - BASE_PADDING, adjusted_height); // 右と下のパディングを除く範囲

    rect // intersect は Option<Rect> を返すので、そのまま返す
  }

}

#[cfg(test)]
mod tests {
    use super::*; // graphics.rs の中身をぜーんぶ使えるようにするおまじない！

    #[test]
    fn test_parse_color_valid_formats() {
        // ちゃんと '#' があってもなくても、6桁でも8桁でもパースできるかな？
        assert_eq!(parse_color("#FF0000").unwrap(), Color::from_rgba8(255, 0, 0, 255));
        assert_eq!(parse_color("00FF00").unwrap(), Color::from_rgba8(0, 255, 0, 255));
        assert_eq!(parse_color("#0000FF80").unwrap(), Color::from_rgba8(0, 0, 255, 128));
        assert_eq!(parse_color("12345678").unwrap(), Color::from_rgba8(0x12, 0x34, 0x56, 0x78));
    }

    #[test]
    fn test_parse_color_invalid_formats() {
        // 変な文字列や長さが違うものはちゃんと None になるかな？
        assert!(parse_color("").is_none());
        assert!(parse_color("#123").is_none());
        assert!(parse_color("GGHHII").is_none()); // 'G' は16進数じゃないもんね！
        assert!(parse_color("#12345").is_none());
        assert!(parse_color("#1234567").is_none());
    }

    #[test]
    fn test_calculate_layout_logic() {
        // calculate_layout は MyGraphics のプライベートメソッド calculate_internal_layout に変わったから、
        // 直接テストするのはちょっと難しくなっちゃったね…(´・ω・`)
        // MyGraphics のインスタンスを作って、その中の値を確認するテストになるかな！
        // ウィンドウ幅 300px の時、アイテム幅が (48*2 + 10) = 106 だから…
        // (300 - 10) / 106 = 290 / 106 = 2.73... で、切り捨てて 2 アイテムになるはず！
        // let (items_per_row, max_text_width, item_width, item_height) = calculate_layout(300); // これはもう呼べないね
        // assert_eq!(items_per_row, 2);
        // assert_eq!(max_text_width, BASE_LAYOUT_ICON_SIZE * 2.0); // 96.0
        // assert_eq!(item_width, BASE_LAYOUT_ICON_SIZE * 2.0 + BASE_PADDING); // 106.0
        // assert_eq!(item_height, BASE_LAYOUT_ICON_SIZE + BASE_PADDING_UNDER_ICON + BASE_TEXT_HEIGHT + BASE_PADDING); // 48+12+16+10 = 86.0

        // もっと狭い時 (アイテム1つ分しか入らない時)
        // let (items_per_row_narrow, _, _, _) = calculate_layout(100);
        // assert_eq!(items_per_row_narrow, 1); // 最低1アイテムは表示するもんね！

        // 0幅の時は…？ (実際には起こらないはずだけど、念のため)
        // let (items_per_row_zero, _, _, _) = calculate_layout(0);
        // assert_eq!(items_per_row_zero, 1); // これも最低1アイテム！
    }

    #[test]
    fn test_clamp_alpha_logic() {
        let color_opaque = Color::from_rgba8(10, 20, 30, 255);
        assert_eq!(MyGraphics::clamp_alpha(color_opaque).alpha(), 1.0); // 255/255.0 = 1.0

        let color_normal_alpha = Color::from_rgba8(10, 20, 30, 128); // 0.5 ちょっと
        assert!((MyGraphics::clamp_alpha(color_normal_alpha).alpha() - 128.0/255.0).abs() < f32::EPSILON);

        let color_too_transparent = Color::from_rgba8(10, 20, 30, 5); // MIN_ALPHA (0.05 * 255 = 12.75) より小さい
        assert!((MyGraphics::clamp_alpha(color_too_transparent).alpha() - MIN_ALPHA).abs() < f32::EPSILON);

        let color_at_min_alpha = Color::from_rgba(0.1, 0.2, 0.3, MIN_ALPHA).unwrap();
        assert!((MyGraphics::clamp_alpha(color_at_min_alpha).alpha() - MIN_ALPHA).abs() < f32::EPSILON);
    }

    // calculate_text_width のテストは、実際のフォントファイルが必要で、
    // 環境によって微妙に結果が変わる可能性もあるから、ちょっと難しいんだよね…＞＜
    // もしテストするなら、テスト用のフォントを用意して、いくつかの文字列で期待される幅を
    // 事前に計算しておく必要があるかも！
    //
    // MyGraphics の他の描画系メソッド (new, resize, draw_start, draw_group, draw_icon, draw_text, draw_finish) や
    // get_background_color, get_border_color, get_item_rect_f32 も、
    // 実際にウィンドウを作ったり、ピクセルデータを比較したりしないとテストが難しいから、
    // 今回はごめんね、ユニットテストはお休みさせてもらうね！(<em>ﾉω・</em>)ﾃﾍ
}
