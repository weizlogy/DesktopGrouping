use std::rc::Rc;

use tiny_skia::Color;
use winit::{
    dpi::PhysicalSize,
    window::{ResizeDirection, Window},
};

use crate::{file_drag::IconInfo, logger::*};
use desktop_grouping::graphics::{self, graphics::MyGraphics};
use desktop_grouping::win32::ui_wam;

/// マウスホイールによるアルファ値調整のステップ量
const ALPHA_ADJUST_STEP: f32 = 0.02;

/// 子ウィンドウを表す構造体。
pub struct ChildWindow {
    pub window: Rc<Window>,
    pub graphics: MyGraphics,
    pub groups: Vec<IconInfo>,
    pub id_str: String,
    pub scale_factor: f64,
}

impl ChildWindow {
    pub fn new(window: Rc<Window>, id_str: String, bg_color_str: &str) -> ChildWindow {
        let initial_scale_factor = window.scale_factor();
        let graphics = MyGraphics::new(&window, bg_color_str, initial_scale_factor);

        // 初期状態の背景透過度を OS に伝えるよ！
        ui_wam::set_window_composition(&window, graphics.get_background_color());
        // 設定を反映させるために, 強制的に再描画を要求するよ！
        ui_wam::force_update_window(&window);

        ChildWindow {
            window,
            graphics,
            groups: Vec::new(),
            id_str,
            scale_factor: initial_scale_factor,
        }
    }

    pub fn update_scale_factor(&mut self, new_scale_factor: f64) {
        self.scale_factor = new_scale_factor;
        self.graphics.update_scale_factor(new_scale_factor);
    }

    pub fn set_background_color(&mut self, color_str: &str) {
        if let Some(bg_color) = graphics::parse_color(color_str) {
            self.graphics.update_background_color(bg_color);
            // 新しい背景色（アルファ込み）を OS の AccentPolicy に反映！
            ui_wam::set_window_composition(&self.window, bg_color);
            // 設定を即座に反映させるよ！
            ui_wam::force_update_window(&self.window);
            self.window.request_redraw();
            log_debug(&format!(
                "Window {}: BG set to {}",
                self.id_str,
                color_to_hex_string(bg_color),
            ));
        } else {
            log_warn(&format!(
                "Window {}: Invalid color string received: {}",
                self.id_str, color_str
            ));
        }
    }

    pub fn adjust_alpha(&mut self, delta: f32) {
        let current_bg_color = self.graphics.get_background_color();
        let current_alpha = current_bg_color.alpha();
        let new_alpha = (current_alpha + delta * ALPHA_ADJUST_STEP).clamp(0.0, 1.0);

        if (new_alpha - current_alpha).abs() > f32::EPSILON {
            let new_bg_color = Color::from_rgba(
                current_bg_color.red(),
                current_bg_color.green(),
                current_bg_color.blue(),
                new_alpha,
            )
            .unwrap();

            self.graphics.update_background_color(new_bg_color);
            // アルファ調整後の色を OS の AccentPolicy に反映！
            ui_wam::set_window_composition(&self.window, new_bg_color);
            // 設定を即座に反映させるよ！
            ui_wam::force_update_window(&self.window);
            self.window.request_redraw();
            log_debug(&format!(
                "Window {}: Alpha adjusted to {:.3}",
                self.id_str, new_alpha,
            ));
        }
    }

    pub fn refresh_background(&self) {
        let current_bg_color = self.graphics.get_background_color();
        // 背景色を OS の AccentPolicy に再反映！
        ui_wam::set_window_composition(&self.window, current_bg_color);
        // 設定を即座に反映させるよ！
        ui_wam::force_update_window(&self.window);
        self.window.request_redraw();
        log_debug(&format!("Window {}: Background refreshed.", self.id_str));
    }

    pub fn add(&mut self, icon: IconInfo) {
        self.groups.push(icon);
    }

    pub fn resize_graphics(&mut self, new_size: PhysicalSize<u32>) {
        self.graphics.resize(new_size);
    }

    pub fn draw(&mut self, hovered_index: Option<usize>, executing_index: Option<usize>) {
        self.graphics.draw_start();

        for (index, icon_info) in self.groups.iter_mut().enumerate() {
            let is_hovered = hovered_index.map_or(false, |h_idx| h_idx == index);
            let is_executing = executing_index.map_or(false, |e_idx| e_idx == index);

            let name = icon_info.name.clone();
            let icon_data = icon_info.get_or_load_icon();
            self.graphics
                .draw_group(index, &name, icon_data, is_hovered, is_executing);
        }
        self.graphics.draw_finish();
    }

    pub fn start_os_drag(&self) {
        if let Err(e) = self.window.drag_window() {
            crate::logger::log_error(&format!(
                "Window drag_window failed for {:?}: {}",
                self.id_str, e
            ));
        }
    }

    pub fn start_os_resize(&self, direction: ResizeDirection) {
        if let Err(e) = self.window.drag_resize_window(direction) {
            crate::logger::log_error(&format!(
                "Window drag_resize_window failed for {:?} (dir: {:?}): {}",
                self.id_str, direction, e
            ));
        }
    }
}

pub fn color_to_hex_string(color: Color) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        (color.red() * 255.0) as u8,
        (color.green() * 255.0) as u8,
        (color.blue() * 255.0) as u8,
        (color.alpha() * 255.0) as u8
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_hex_string() {
        let color1 = Color::from_rgba8(255, 0, 0, 255);
        assert_eq!(color_to_hex_string(color1), "#FF0000FF");

        let color2 = Color::from_rgba8(0, 255, 0, 128);
        assert_eq!(color_to_hex_string(color2), "#00FF0080");

        let color3 = Color::from_rgba8(16, 32, 48, 0);
        assert_eq!(color_to_hex_string(color3), "#10203000");
    }
}
