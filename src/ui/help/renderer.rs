use std::rc::Rc;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use crate::graphics::{Canvas, GraphicsEngine, DrawingResources, layout, drawing::{background, help}};

pub struct HelpRenderer {
    canvas: Canvas,
    resources: DrawingResources,
}

impl HelpRenderer {
    pub fn new(
        engine: Rc<GraphicsEngine>,
        hwnd: HWND,
        width: u32,
        height: u32,
    ) -> Result<Self, windows::core::Error> {
        let canvas = Canvas::new(engine.clone(), hwnd, width, height)?;
        let resources = DrawingResources::new(
            engine.dwrite_factory.clone(),
            engine.wic_factory.clone(),
        );

        let renderer = Self { canvas, resources };

        unsafe {
            renderer.canvas.d2d_context.SetTextAntialiasMode(windows::Win32::Graphics::Direct2D::D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        }

        Ok(renderer)
    }

    pub fn render(
        &mut self,
        width: f32,
        height: f32,
        bg_color_hex: &str,
        opacity: f32,
    ) -> Result<(), windows::core::Error> {
        self.canvas.begin_draw();

        let context = &self.canvas.d2d_context;
        let bg_rect = D2D_RECT_F { left: 0.0, top: 0.0, right: width, bottom: height };
        let bg_brush = self.resources.get_brush(context, bg_color_hex)?;

        let bg_color = unsafe { bg_brush.GetColor() };
        let is_dark = layout::is_dark_color(bg_color.r, bg_color.g, bg_color.b);
        let text_color_hex = if is_dark { "#FFFFFFFF" } else { "#000000FF" };
        let border_color_hex = if is_dark { "#FFFFFF33" } else { "#00000033" };
        let border_brush = self.resources.get_brush(context, border_color_hex)?;

        unsafe {
            bg_brush.SetOpacity(opacity);
            border_brush.SetOpacity(opacity * 0.5);
        }

        background::draw_rounded_rect(context, &bg_rect, &bg_brush, Some(&border_brush), 1.5, 12.0);

        // ヘルプテキストの描画
        help::draw_help(context, width, height, text_color_hex, &mut self.resources)?;

        self.canvas.end_draw(1)?;
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), windows::core::Error> {
        self.canvas.resize(width, height)
    }
}
