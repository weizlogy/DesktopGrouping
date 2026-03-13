use std::rc::Rc;
use windows::Win32::Foundation::HWND;
use crate::graphics::{Canvas, GraphicsEngine, drawing::resources::DrawingResources};
use crate::graphics::drawing::painter;
use crate::ui::group::model::GroupModel;

/// グループウィンドウの描画を管理するよ！
pub struct GroupRenderer {
    canvas: Canvas,
    resources: DrawingResources,
}

impl GroupRenderer {
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

        Ok(Self {
            canvas,
            resources,
        })
    }

    /// グループを描画するよ。
    pub fn render(&mut self, model: &GroupModel, width: f32, height: f32) -> Result<(), windows::core::Error> {
        self.canvas.begin_draw();

        // painter に描画を依頼するよ。
        painter::draw_group(
            &self.canvas.d2d_context,
            width,
            height,
            model,
            &mut self.resources,
        )?;

        self.canvas.end_draw()?;
        Ok(())
    }

    /// ウィンドウサイズが変わったときに呼び出してね。
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), windows::core::Error> {
        self.canvas.resize(width, height)
    }
}
