use std::{num::NonZeroU32, rc::Rc};

use ab_glyph::FontRef;
use softbuffer::{Context, Surface as SoftSurface};
use tiny_skia::{
    Color, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, PixmapPaint, Point, Rect, Shader,
    SpreadMode, Transform, FillRule,
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
const CORNER_RADIUS: f32 = 12.0; // ウィンドウの枠線の丸角のアール
// ---------------------------------------------------------

/// ウィンドウごとのグラフィック描画を担当する構造体だよ！
/// `softbuffer` を使ってウィンドウにピクセルバッファを描画して、
/// `tiny_skia` で背景、枠線、アイコン、テキストとかを描画するんだ。
pub struct MyGraphics {
    soft_surface: SoftSurface<Rc<Window>, Rc<Window>>,
    pub pixmap: Pixmap,
    // --- レイアウト情報 ---
    pub width: u32,           // ピクセルマップの幅 (ウィンドウの内部幅と同じだよ！)
    pub height: u32,          // ピクセルマップの高さ (ウィンドウの内部高さと同じだよ！)
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
    pub font: FontRef<'static>,           // フォントデータを保持
    background_paint: Paint<'static>, // 背景色用 Paint
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

        let mut background_paint = Paint::default();
        background_paint.set_color(bg_color);
        background_paint.anti_alias = true; // お好みで

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

        // --- 影の描画 ---
        // 現在の背景色を取得し、そのアルファ値に比例した影の色を生成する
        let base_color = self.get_background_color();
        let shadow_alpha = (base_color.alpha() * 70.0) as u8; // 最大の濃さを70とする
        let shadow_color = Color::from_rgba8(0, 0, 0, shadow_alpha);

        let shadow_offset_x = 2.0 * self.scale_factor as f32; // X方向のオフセット
        let shadow_offset_y = 3.0 * self.scale_factor as f32; // Y方向のオフセット

        // 背景と影のための角丸矩形のパスを一度だけ生成する
        let main_path = {
            let w = self.width as f32;
            let h = self.height as f32;
            // 影がはみ出さないように、描画領域を少しだけ内側にオフセットさせる
            let rect_w = w - shadow_offset_x.abs() - 4.0;
            let rect_h = h - shadow_offset_y.abs() - 4.0;
            let rect_x = (w - rect_w) / 2.0 - shadow_offset_x / 2.0;
            let rect_y = (h - rect_h) / 2.0 - shadow_offset_y / 2.0;

            let mut r: f32 = CORNER_RADIUS;
            r = r.min(rect_w / 2.0).min(rect_h / 2.0); // 半径が大きすぎる場合に調整

            if rect_w > 0.0 && rect_h > 0.0 {
                let mut pb = PathBuilder::new();
                pb.move_to(rect_x + r, rect_y);
                pb.line_to(rect_x + rect_w - r, rect_y);
                pb.quad_to(rect_x + rect_w, rect_y, rect_x + rect_w, rect_y + r);
                pb.line_to(rect_x + rect_w, rect_y + rect_h - r);
                pb.quad_to(
                    rect_x + rect_w,
                    rect_y + rect_h,
                    rect_x + rect_w - r,
                    rect_y + rect_h,
                );
                pb.line_to(rect_x + r, rect_y + rect_h);
                pb.quad_to(rect_x, rect_y + rect_h, rect_x, rect_y + rect_h - r);
                pb.line_to(rect_x, rect_y + r);
                pb.quad_to(rect_x, rect_y, rect_x + r, rect_y);
                pb.close();
                pb.finish()
            } else {
                None
            }
        };

        // --- 影の描画 ---
        if let Some(ref path) = main_path {
            let mut shadow_paint = Paint::default();
            shadow_paint.set_color(shadow_color);
            shadow_paint.anti_alias = true;
            // パスを少しオフセットさせて影を描画
            let shadow_transform = Transform::from_translate(shadow_offset_x, shadow_offset_y);
            self.pixmap.fill_path(
                path,
                &shadow_paint,
                FillRule::Winding,
                shadow_transform,
                None,
            );
        }

        // --- 角丸背景の描画 ---
        if let Some(bg_path) = main_path {
            let base_color = self.get_background_color();
            let (start_color, end_color) = colors::create_gradient_colors(base_color);
            let w = self.width as f32;
            let h = self.height as f32;

            if let Some(gradient) = LinearGradient::new(
                Point::from_xy(0.0, 0.0),
                Point::from_xy(w, h),
                vec![
                    GradientStop::new(0.0, start_color),
                    GradientStop::new(1.0, end_color),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            ) {
                let mut gradient_paint = Paint::default();
                gradient_paint.shader = gradient;
                gradient_paint.anti_alias = true;
                self.pixmap.fill_path(
                    &bg_path,
                    &gradient_paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            } else {
                // グラデーション失敗時は単色で塗りつぶし
                self.pixmap.fill_path(
                    &bg_path,
                    &self.background_paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
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
        let (header, pixel_data) = icon_data;

        // --- グリッドと描画座標の計算 ---
        let col = index % self.items_per_row;
        let row = index / self.items_per_row;

        let grid_x = (col as f32 * self.item_width) + self.padding;
        let grid_y = (row as f32 * self.item_height) + self.padding;

        let icon_draw_x = grid_x + (self.max_text_width / 2.0) - (self.layout_icon_size / 2.0);
        let icon_draw_y = grid_y;

        let text_draw_x = grid_x;
        let text_draw_y = grid_y + self.layout_icon_size + self.padding_under_icon;

        // --- ホバー時の背景 ---
        if is_hovered {
            if let Some(rect) = self.get_item_rect_f32(index) {
                let base_bg_color = self.get_background_color();
                let hover_fill_color = colors::calculate_hover_fill_color(base_bg_color);
                let mut fill_paint = Paint::default();
                fill_paint.set_color(hover_fill_color);
                fill_paint.anti_alias = true;
                self.pixmap
                    .fill_rect(rect, &fill_paint, Transform::identity(), None);
            }
        }

        // --- アイコン描画 ---
        let mut transform = Transform::identity();
        if is_executing {
            let scale = 1.05;
            let center_x = icon_draw_x + self.layout_icon_size / 2.0;
            let center_y = icon_draw_y + self.layout_icon_size / 2.0;
            transform = transform
                .post_translate(-center_x, -center_y)
                .post_scale(scale, scale)
                .post_translate(center_x, center_y);
        }

        // drawing::draw_icon のロジックを展開して transform を適用
        let temp_icon_pixmap =
            super::drawing::convert_dib_to_pixmap(header, pixel_data, self.layout_icon_size as u32);

        let mut paint = PixmapPaint::default();
        paint.quality = tiny_skia::FilterQuality::Bicubic;
        self.pixmap.draw_pixmap(
            icon_draw_x as i32,
            icon_draw_y as i32,
            temp_icon_pixmap.as_ref(),
            &paint,
            transform,
            None,
        );

        // --- 実行中エフェクト（明度アップ） ---
        if is_executing {
            let icon_rect = Rect::from_xywh(
                icon_draw_x,
                icon_draw_y,
                self.layout_icon_size,
                self.layout_icon_size,
            )
            .unwrap();
            let mut fade_paint = Paint::default();
            // 明度を20%上げる -> 20%の不透明度の白を重ねる
            fade_paint.set_color_rgba8(255, 255, 255, (255.0 * 0.2) as u8);
            fade_paint.anti_alias = true;
            self.pixmap
                .fill_rect(icon_rect, &fade_paint, Transform::identity(), None);
        }


        // テキストを描画 (最大幅を指定)
        drawing::draw_text(
            &mut self.pixmap,
            &self.font,
            self.text_font_size,
            &icon_name,
            text_draw_x,
            text_draw_y,
            self.max_text_width,
            self.text_height,
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
