// src/settings_window.rs
use std::rc::Rc;

use winit::{
    dpi::PhysicalSize,
    window::Window,
};

use desktop_grouping::graphics::{self, graphics::MyGraphics};
use crate::logger::*;
// settings モジュールから設定を読み書きするために必要
use crate::settings::{get_settings_reader, get_settings_writer, save_settings};

pub struct SettingsWindow {
    pub window: Rc<Window>,
    pub graphics: MyGraphics,
    pub scale_factor: f64,
    // TODO: ここに設定項目を追加する
}

impl SettingsWindow {
    pub fn new(window: Rc<Window>) -> Self {
        let initial_scale_factor = window.scale_factor();
        let default_bg_color = graphics::parse_color("#333333FF").unwrap(); // デフォルトの背景色
        let graphics = MyGraphics::new(
            &window,
            &color_to_hex_string(default_bg_color), // カラー文字列を渡す
            initial_scale_factor,
        );

        SettingsWindow {
            window,
            graphics,
            scale_factor: initial_scale_factor,
            // TODO: 設定項目の初期化
        }
    }

    pub fn update_scale_factor(&mut self, new_scale_factor: f64) {
        self.scale_factor = new_scale_factor;
        self.graphics.update_scale_factor(new_scale_factor);
    }

    pub fn resize_graphics(&mut self, new_size: PhysicalSize<u32>) {
        self.graphics.resize(new_size);
    }

    pub fn draw(&mut self) {
        self.graphics.draw_start();
        // TODO: ここに設定項目を描画するロジックを追加
        // 例えば、テキストを描画したり、簡単なUI要素を描画したり
        self.graphics.draw_text_with_bg(
            "設定",
            10.0,
            10.0,
            24.0,
            graphics::parse_color("#FFFFFFFF").unwrap(), // 白いテキスト
            graphics::parse_color("#00000000").unwrap(), // 透明な背景
            false // 枠線なし
        );

        self.graphics.draw_finish();
    }

    // `child_window.rs` から `color_to_hex_string` を拝借
    // 後で適切な場所に移動するか、`graphics` モジュールに含める
}

// `child_window.rs` からコピー
pub fn color_to_hex_string(color: tiny_skia::Color) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        (color.red() * 255.0) as u8,
        (color.green() * 255.0) as u8,
        (color.blue() * 255.0) as u8,
        (color.alpha() * 255.0) as u8
    )
}

