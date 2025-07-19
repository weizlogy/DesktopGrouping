use std::{num::NonZeroU32, rc::Rc};

use ab_glyph::FontRef;
use softbuffer::{Context, Surface as SoftSurface};
use tiny_skia::{
    Color, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, Point, Rect, Shader,
    SpreadMode, Stroke, Transform,
};
use windows::Win32::Graphics::Gdi::BITMAPINFO;
use winit::{dpi::PhysicalSize, window::Window};

use crate::logger::*;

use super::{
    colors::{self, parse_color},
    drawing,
    layout::{
        self, BASE_ADJUST_SELECT_RECT, BASE_LAYOUT_ICON_SIZE, BASE_PADDING,
        BASE_PADDING_UNDER_ICON, BASE_TEXT_FONT_SIZE, BASE_TEXT_HEIGHT,
    },
};

// --- 枠線定数 ---
const BORDER_WIDTH: f32 = 2.0; // ウィンドウの枠線の太さだよ！今は2ピクセルだね。
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
    item_height: f32, // グリッドレイアウトの1アイテムあたりの高さだよ (アイコンの高さ + テキストの高さ + 余白)。
    // --- スケーリングされたレイアウト値だよ！ ---
    padding: f32,
    layout_icon_size: f32,
    padding_under_icon: f32,
    text_height: f32,
    text_font_size: f32,
    adjust_select_rect: f32,
    // border_width は今のところ固定だけど、もしスケーリングしたくなったらここに追加するんだ！

    // --- フォント ---
    font: FontRef<'static>,           // フォントデータを保持
    background_paint: Paint<'static>, // 背景色用 Paint
    border_paint: Paint<'static>,     // 枠線色用 Paint
    border_stroke: Stroke,            // 枠線の太さなど
}

impl MyGraphics {
    /// 新しい `MyGraphics` インスタンスを作るよ！
    ///
    /// ウィンドウハンドル (`window`) と、初期の背景色・枠線色、それから初期の拡大率 (`initial_scale_factor`) をもらって、
    /// 描画に必要なもの (softbufferのサーフェス、ピクセルマップ、フォント、レイアウト情報など) を準備するんだ。
    ///
    /// 色の文字列がもしパースできなかったら、優しいデフォルト色にしてくれるから安心してね！(<em>´ω｀</em>)
    pub fn new(
        window: &Rc<Window>,
        bg_color_str: &str,
        border_color_str: &str,
        initial_scale_factor: f64,
    ) -> Self {
        let initial_size = window.inner_size();
        let width = initial_size.width;
        let height = initial_size.height;

        let context = Context::new(window.clone()).expect("Failed to create context");
        let mut soft_surface =
            SoftSurface::new(&context, window.clone()).expect("Failed to create surface");

        // resize を呼ぶ前に Pixmap を初期化
        let pixmap = Pixmap::new(width, height).expect("Failed to create initial Pixmap");
        // soft_surface のリサイズも試みる
        soft_surface
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .expect("Failed to resize surface");

        // フォントをロードして保持
        let font_data = include_bytes!("../../resource/NotoSansJP-Medium.ttf");
        let font = FontRef::try_from_slice(font_data).expect("Failed to load font");

        // 色をパース、失敗したらデフォルト色にフォールバック
        let bg_color =
            parse_color(bg_color_str).unwrap_or_else(|| Color::from_rgba8(255, 255, 255, 153)); // Default: #FFFFFF99
        let border_color =
            parse_color(border_color_str).unwrap_or_else(|| Color::from_rgba8(0, 0, 0, 255)); // Default: #000000FF

        let mut background_paint = Paint::default();
        background_paint.set_color(bg_color);
        background_paint.anti_alias = true; // お好みで

        let mut border_paint = Paint::default();
        border_paint.set_color(border_color);
        border_paint.anti_alias = true;

        let border_stroke = Stroke {
            width: BORDER_WIDTH,
            ..Default::default()
        }; // 枠線の太さなど

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
            layout::calculate_internal_layout(
                self.width,
                self.layout_icon_size,
                self.padding,
                self.padding_under_icon,
                self.text_height,
            );
        self.items_per_row = items_per_row;
        self.max_text_width = max_text_width;
        self.item_width = item_width;
        self.item_height = item_height;
    }

    /// 背景色を更新するよ！
    /// 新しい色 (`color`) をもらって、アルファ値を下限チェックしてから `background_paint` に設定するんだ。
    pub fn update_background_color(&mut self, color: Color) {
        let clamped_color = colors::clamp_alpha(color);
        self.background_paint.set_color(clamped_color);
    }

    /// 枠線色を更新するよ！
    /// 新しい色 (`color`) をもらって、こっちもアルファ値を下限チェックしてから `border_paint` に設定するんだ。
    pub fn update_border_color(&mut self, color: Color) {
        let clamped_color = colors::clamp_alpha(color);
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

        self.soft_surface
            .resize(
                NonZeroU32::new(self.width).unwrap(),
                NonZeroU32::new(self.height).unwrap(),
            )
            .expect("Failed to resize surface");
        self.pixmap =
            Pixmap::new(self.width, self.height).expect("Failed to create initial Pixmap");

        // スケーリングされたレイアウト値を再計算！
        self.update_scaled_layout_values();
    }

    /// 拡大率 (`scale_factor`) が変わった時に呼び出すよ！
    /// 新しい拡大率を覚えて、レイアウトを再計算するんだ。
    pub fn update_scale_factor(&mut self, new_scale_factor: f64) {
        // 拡大率が本当に変わったかチェック！ちょっとだけ違っても再計算しないようにするよ。
        if (self.scale_factor - new_scale_factor).abs() > f64::EPSILON {
            log_debug(&format!(
                "MyGraphics: Scale factor changing from {} to {}",
                self.scale_factor, new_scale_factor
            ));
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
        self.pixmap.fill(Color::TRANSPARENT);

        // --- グラデーション背景の描画 ---
        let base_color = self.get_background_color();
        let (start_color, end_color) = colors::create_gradient_colors(base_color);

        let gradient_stops = vec![
            GradientStop::new(0.0, start_color),
            GradientStop::new(1.0, end_color),
        ];

        // ウィンドウの左上から右下へのグラデーション
        let start_point = Point::from_xy(0.0, 0.0);
        let end_point = Point::from_xy(self.width as f32, self.height as f32);

        if let Some(gradient) = LinearGradient::new(
            start_point,
            end_point,
            gradient_stops,
            SpreadMode::Pad, // グラデーションの範囲外は端の色で埋める
            Transform::identity(),
        ) {
            let mut gradient_paint = Paint::default();
            gradient_paint.shader = gradient;
            gradient_paint.anti_alias = true;

            let rect = Rect::from_xywh(0.0, 0.0, self.width as f32, self.height as f32).unwrap();
            self.pixmap
                .fill_rect(rect, &gradient_paint, Transform::identity(), None);
        } else {
            // グラデーション作成に失敗した場合は、単色で塗りつぶす
            let rect = Rect::from_xywh(0.0, 0.0, self.width as f32, self.height as f32).unwrap();
            self.pixmap
                .fill_rect(rect, &self.background_paint, Transform::identity(), None);
        }

        // --- 枠線描画 (MyGraphics::new で設定した border_paint と border_stroke を使う) ---
        // stroke_rect は中心線で描画されるため、半分の太さだけ内側にオフセットする
        let border_half_width = self.border_stroke.width / 2.0;
        let border_rect = Rect::from_xywh(
            border_half_width,
            border_half_width,
            (self.width as f32 - self.border_stroke.width).max(0.0), // 幅が負にならないように
            (self.height as f32 - self.border_stroke.width).max(0.0), // 高さが負にならないように
        );

        if let Some(valid_border_rect) = border_rect {
            // Rect 作成が成功した場合のみ描画
            let path = PathBuilder::from_rect(valid_border_rect);
            self.pixmap.stroke_path(
                &path,               // PathBuilder::from_rect が返すのは Path なので &path で参照を渡す
                &self.border_paint,  // 構造体フィールドの border_paint を使用
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
        is_hovered: bool,
        is_executing: bool,
    ) {
        let _header = &icon_data.0.bmiHeader;

        // --- グリッドと描画座標の計算 ---
        let col = index % self.items_per_row;
        let row = index / self.items_per_row;

        // グリッドの左上の X 座標 (テキスト描画の基準)
        let grid_x = (col as f32 * self.item_width) + self.padding;
        // グリッドの左上の Y 座標 (アイコン描画の基準)
        let grid_y = (row as f32 * self.item_height) + self.padding;

        // アイコンの描画座標 (テキストの中央に配置)
        // テキストが省略される可能性があるので、max_text_width を基準にする
        // アイコンの幅はレイアウト基準の layout_icon_size を使うことで、
        // 取得したアイコンのサイズに依らず、常にグリッドの中央に配置されるようにする。
        // これにより、アイコン取得失敗時 (幅が0) でも描画位置が定まる。
        let icon_draw_x = grid_x + (self.max_text_width / 2.0) - (self.layout_icon_size / 2.0);
        let icon_draw_y = grid_y;

        // テキストの描画座標 (スケーリング済みの値を使うよ！)
        let text_draw_x = grid_x; // テキストはグリッドの左端から
        let text_draw_y = grid_y + self.layout_icon_size + self.padding_under_icon; // アイコンの下に、余白を挟んで配置

        // --- ホバー状態なら背景を塗るよ ---
        if is_hovered {
            if let Some(rect) = self.get_item_rect_f32(index) {
                let base_bg_color = self.get_background_color(); // ウィンドウの現在の背景色を取得
                let hover_fill_color = colors::calculate_hover_fill_color(base_bg_color);

                let mut fill_paint = Paint::default();
                fill_paint.set_color(hover_fill_color);
                fill_paint.anti_alias = true;
                self.pixmap
                    .fill_rect(rect, &fill_paint, Transform::identity(), None);
            }
        }

        // --- 実行中かホバー中かで枠線を描き分けるよ ---
        if is_executing {
            if let Some(rect) = self.get_item_rect_f32(index) {
                // 実行中エフェクト (Gold の太い枠線)
                let mut exec_paint = Paint::default();
                exec_paint.set_color_rgba8(0xFF, 0xD7, 0x00, 0xCC); // Gold
                exec_paint.anti_alias = true;
                let exec_stroke = Stroke {
                    width: BORDER_WIDTH * 1.5,
                    ..Default::default()
                };
                self.pixmap.stroke_path(
                    &PathBuilder::from_rect(rect),
                    &exec_paint,
                    &exec_stroke,
                    Transform::identity(),
                    None,
                );
            }
        } else if is_hovered {
            if let Some(rect) = self.get_item_rect_f32(index) {
                let base_bg_color = self.get_background_color(); // ウィンドウの現在の背景色を取得
                let hover_border_color = colors::calculate_hover_border_color(base_bg_color);

                let mut stroke_paint = Paint::default();
                stroke_paint.set_color(hover_border_color);
                stroke_paint.anti_alias = true;
                let stroke = Stroke {
                    width: BORDER_WIDTH,
                    ..Default::default()
                };
                self.pixmap.stroke_path(
                    &PathBuilder::from_rect(rect),
                    &stroke_paint,
                    &stroke,
                    Transform::identity(),
                    None,
                );
            }
        }

        // アイコンを描画
        drawing::draw_icon(
            &mut self.pixmap,
            &icon_data.0,
            &icon_data.1,
            icon_draw_x as u32,
            icon_draw_y as u32,
            self.layout_icon_size as u32,
        );

        // テキストを描画 (最大幅を指定)
        drawing::draw_text(
            &mut self.pixmap,
            &self.font,
            self.text_font_size,
            &icon_name,
            text_draw_x,
            text_draw_y,
            self.max_text_width,
            self.text_height, // text_height を追加するよ！
        );
    }

    /// これまでピクセルマップに描画してきた内容を、実際にウィンドウに表示するよ！
    ///
    /// `soft_surface` からウィンドウのバッファを取得して、`self.pixmap` の内容をそこにコピーするんだ。
    /// ピクセルフォーマット (RGBA とか ARGB とか) の違いも、ここで吸収してるみたいだね！
    pub fn draw_finish(&mut self) {
        let mut buffer = self
            .soft_surface
            .buffer_mut()
            .expect("Failed to get buffer");

        // Pixmap のデータ (事前乗算済みアルファ RGBA) を softbuffer のバッファ (非乗算アルファ ARGB) にコピーします。
        let pixmap_data = self.pixmap.data();
        for (i, pixel) in buffer.iter_mut().enumerate() {
            // tiny_skia は事前乗算済みアルファ (premultiplied alpha) を使用します。
            let r_p = pixmap_data[i * 4 + 0];
            let g_p = pixmap_data[i * 4 + 1];
            let b_p = pixmap_data[i * 4 + 2];
            let a = pixmap_data[i * 4 + 3];

            // softbuffer は非乗算アルファ (straight alpha) を期待するため、色を元に戻す必要があります。
            let (r, g, b) = if a > 0 {
                // a で割る前に r_p, g_p, b_p を f32 にキャスト
                let r_u32 = (r_p as f32 * 255.0 / a as f32) as u32;
                let g_u32 = (g_p as f32 * 255.0 / a as f32) as u32;
                let b_u32 = (b_p as f32 * 255.0 / a as f32) as u32;
                // clamp で 0-255 の範囲に収める
                let r_u8 = r_u32.clamp(0, 255) as u8;
                let g_u8 = g_u32.clamp(0, 255) as u8;
                let b_u8 = b_u32.clamp(0, 255) as u8;
                (r_u8, g_u8, b_u8)
            } else {
                (0, 0, 0) // アルファが 0 なら RGB も 0 に
            };

            // softbuffer が期待する u32 (0xAARRGGBB) 形式に変換してピクセルを書き込みます。
            *pixel = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }

        buffer.present().expect("Failed to commit surface");
    }

    /// 指定されたインデックスのアイテム全体（アイコン、テキスト、パディング）が
    /// 描画される矩形領域（相対座標、f32）を計算します。
    ///
    /// マウスカーソルがどのアイコンの上にあるか判定する時 (ホバー判定) とかに使うよ！
    /// `ADJUST_SELECT_RECT` を使って、選択範囲の見た目をちょっと調整してるんだ。
    pub fn get_item_rect_f32(&self, index: usize) -> Option<Rect> {
        // layout モジュールに移動した関数を呼び出す
        layout::get_item_rect_f32(
            index,
            self.width,
            self.height,
            self.items_per_row,
            self.item_width,
            self.item_height,
            self.padding,
            self.adjust_select_rect,
        )
    }
}

#[cfg(test)]
mod tests {
    // use super::*; // graphics.rs の中身をぜーんぶ使えるようにするおまじない！
    // MyGraphics の他の描画系メソッド (new, resize, draw_start, draw_group, draw_icon, draw_text, draw_finish) や
    // get_background_color, get_border_color, get_item_rect_f32 も、
    // 実際にウィンドウを作ったり、ピクセルデータを比較したりしないとテストが難しいから、
    // 今回はごめんね、ユニットテストはお休みさせてもらうね！(<em>ﾉω・</em>)ﾃﾍ
}
