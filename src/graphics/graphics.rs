use std::{num::NonZeroU32, rc::Rc};

use ab_glyph::FontRef;
use softbuffer::{Context, Surface as SoftSurface};
use tiny_skia::{Color, Pixmap, PixmapPaint, Rect, Transform};
use windows::Win32::Graphics::Gdi::BITMAPINFO;
use winit::{dpi::PhysicalSize, window::Window};

use super::{
    colors::{self, parse_color},
    drawing,
    layout::{
        self, BASE_ADJUST_SELECT_RECT, BASE_LAYOUT_ICON_SIZE, BASE_PADDING,
        BASE_PADDING_UNDER_ICON, BASE_TEXT_FONT_SIZE, BASE_TEXT_HEIGHT,
    },
};

// ---------------------------------------------------------

/// ウィンドウごとのグラフィック描画を担当する構造体だよ！
pub struct MyGraphics {
    soft_surface: SoftSurface<Rc<Window>, Rc<Window>>,
    pub pixmap: Pixmap,
    // --- レイアウト情報 ---
    pub width: u32,
    pub height: u32,
    scale_factor: f64,
    items_per_row: usize,
    max_text_width: f32,
    item_width: f32,
    item_height: f32,
    // --- スケーリングされたレイアウト値 ---
    padding: f32,
    layout_icon_size: f32,
    padding_under_icon: f32,
    text_height: f32,
    text_font_size: f32,
    adjust_select_rect: f32,

    // --- フォントと色 ---
    pub font: FontRef<'static>,
    intended_bg_color: Color, // 本来の背景色 (アルファ込み)
}

impl MyGraphics {
    pub fn new(window: &Rc<Window>, bg_color_str: &str, initial_scale_factor: f64) -> Self {
        let initial_size = window.inner_size();
        let width = initial_size.width;
        let height = initial_size.height;

        let context = Context::new(window.clone()).expect("Failed to create context");
        let mut soft_surface =
            SoftSurface::new(&context, window.clone()).expect("Failed to create surface");

        let pixmap = Pixmap::new(width, height).expect("Failed to create initial Pixmap");
        soft_surface
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .expect("Failed to resize surface");

        let font_data = include_bytes!("../../resource/NotoSansJP-Medium.ttf");
        let font = FontRef::try_from_slice(font_data).expect("Failed to load font");

        let bg_color =
            parse_color(bg_color_str).unwrap_or_else(|| Color::from_rgba8(255, 255, 255, 153));

        let mut graphics = MyGraphics {
            soft_surface,
            pixmap,
            width,
            height,
            scale_factor: initial_scale_factor,
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
            font,
            intended_bg_color: bg_color,
        };
        graphics.update_scaled_layout_values();
        return graphics;
    }

    fn update_scaled_layout_values(&mut self) {
        self.padding = (BASE_PADDING as f64 * self.scale_factor) as f32;
        self.layout_icon_size = (BASE_LAYOUT_ICON_SIZE as f64 * self.scale_factor) as f32;
        self.padding_under_icon = (BASE_PADDING_UNDER_ICON as f64 * self.scale_factor) as f32;
        self.text_height = (BASE_TEXT_HEIGHT as f64 * self.scale_factor) as f32;
        self.text_font_size = (BASE_TEXT_FONT_SIZE as f64 * self.scale_factor) as f32;
        self.adjust_select_rect = (BASE_ADJUST_SELECT_RECT as f64 * self.scale_factor) as f32;

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

    pub fn update_background_color(&mut self, color: Color) {
        let clamped_color = colors::clamp_alpha(color);
        self.intended_bg_color = clamped_color;
    }

    pub fn get_background_color(&self) -> Color {
        self.intended_bg_color
    }

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

        self.update_scaled_layout_values();
    }

    pub fn update_scale_factor(&mut self, new_scale_factor: f64) {
        if (self.scale_factor - new_scale_factor).abs() > f64::EPSILON {
            self.scale_factor = new_scale_factor;
            self.update_scaled_layout_values();
        }
    }

    pub fn draw_start(&mut self) {
        // 背景は DWM に任せるから, Pixmap は完全に透明な状態でクリアするよ！
        self.pixmap.fill(Color::TRANSPARENT);
    }

    pub fn draw_group(
        &mut self,
        index: usize,
        icon_name: &String,
        icon_data: &(BITMAPINFO, Vec<u8>),
        is_hovered: bool,
        is_executing: bool,
    ) {
        let (header, pixel_data) = icon_data;
        let col = index % self.items_per_row;
        let row = index / self.items_per_row;

        let grid_x = (col as f32 * self.item_width) + self.padding;
        let grid_y = (row as f32 * self.item_height) + self.padding;

        let icon_draw_x = grid_x + (self.max_text_width / 2.0) - (self.layout_icon_size / 2.0);
        let icon_draw_y = grid_y;

        let text_draw_x = grid_x;
        let text_draw_y = grid_y + self.layout_icon_size + self.padding_under_icon;

        if is_hovered {
            if let Some(rect) = self.get_item_rect_f32(index) {
                let base_bg_color = self.get_background_color();
                let hover_fill_color = colors::calculate_hover_fill_color(base_bg_color);
                let mut fill_paint = tiny_skia::Paint::default();
                fill_paint.set_color(hover_fill_color);
                fill_paint.anti_alias = true;
                self.pixmap
                    .fill_rect(rect, &fill_paint, Transform::identity(), None);
            }
        }

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

        if is_executing {
            let icon_rect = Rect::from_xywh(
                icon_draw_x,
                icon_draw_y,
                self.layout_icon_size,
                self.layout_icon_size,
            )
            .unwrap();
            let mut fade_paint = tiny_skia::Paint::default();
            fade_paint.set_color_rgba8(255, 255, 255, (255.0 * 0.2) as u8);
            fade_paint.anti_alias = true;
            self.pixmap
                .fill_rect(icon_rect, &fade_paint, Transform::identity(), None);
        }

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

    pub fn draw_finish(&mut self) {
        let mut buffer = self
            .soft_surface
            .buffer_mut()
            .expect("Failed to get buffer");

        let pixmap_data = self.pixmap.data();
        for (i, pixel) in buffer.iter_mut().enumerate() {
            let r = pixmap_data[i * 4 + 0];
            let g = pixmap_data[i * 4 + 1];
            let b = pixmap_data[i * 4 + 2];
            let a = pixmap_data[i * 4 + 3];

            *pixel = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }

        buffer.present().expect("Failed to commit surface");
    }

    pub fn get_item_rect_f32(&self, index: usize) -> Option<Rect> {
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
