// src/ui/settings_window.rs
use std::rc::Rc;

use winit::{dpi::PhysicalSize, window::Window}; // Color を使うために必要

use crate::graphics::{self, graphics::MyGraphics};

// 操作説明テキストを定数として定義
const OPERATION_INSTRUCTIONS: [&str; 17] = [
    "## 操作説明",
    "### ■ Create Groups:",
    "  - Right-click: トレイアイコンを右クリックしてメニューを表示し New Group.",
    "  - Drag & Drop: ファイルをドラッグ＆ドロップしてグループに簡単に追加できます。",
    "### ■ Icons:",
    "  - Left-double-click: アイコンを左クリックするとアプリケーションが起動またはファイルが開きます。",
    "  - Right-click: アイコンを右クリックするとそのファイルがあるフォルダが開きます。",
    "  - Ctrl + Right-click: アイコンをCtrl + 右クリックするとそのアイコンを削除します。",
    "### ■ Customization:",
    "  - Move: Ctrl + ドラッグ でグループを移動します。",
    "  - Resize: Shift + ドラッグ でグループのサイズを変更します。",
    "  - Color: Ctrl + V でグループにカラーコード (例: #FF000099, #0F0) を貼り付けると背景色が変わります。",
    "   「#Random」でランダムな色に変更できます。",
    "  - Transparency: Ctrl + マウスホイール で透明度を調整します。",
    "### ■ Delete Groups:",
    "  - Ctrl + Right-click: グループの何もない場所を Ctrl + 右クリック するとグループを削除します。",
    "", // 末尾に空行を追加
];

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

        let text_font_size = 18.0; // 操作説明用のフォントサイズを大きく
        let line_height = text_font_size * 1.5; // 行の高さ
        let mut current_y = 10.0; // Y座標の開始位置
        let base_start_x = 10.0; // 基本の開始X座標
        let label_indent = 20.0; // ラベルのインデント
        let description_indent = 150.0; // 説明のインデント
        let max_width = self.graphics.width as f32 - 20.0; // ウィンドウ幅からマージンを引く

        // タイトル
        crate::graphics::drawing::draw_text(
            &mut self.graphics.pixmap,
            &self.graphics.font,
            24.0,
            "設定",
            base_start_x,
            current_y,
            max_width,
            24.0,
        );
        current_y += 40.0; // タイトルの下、少し広めに

        // 操作説明テキストを表示
        for line in OPERATION_INSTRUCTIONS.iter() {
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() {
                current_y += line_height * 0.5; // 空行は少し行間を開ける
                continue;
            }

            // 見出しの処理 (###) を先に判定
            if trimmed_line.starts_with("###") {
                let cleaned_title =
                    strip_markdown_formatting(trimmed_line.trim_start_matches("###")).to_string();
                crate::graphics::drawing::draw_text(
                    &mut self.graphics.pixmap,
                    &self.graphics.font,
                    text_font_size * 1.1, // ###見出しは##より少し小さく
                    &cleaned_title,
                    base_start_x - 300.0, // ###見出しは少しインデント
                    current_y,
                    max_width,
                    line_height * 1.1,
                );
                current_y += line_height * 1.2; // 見出しの下は広めに
                continue;
            } else if trimmed_line.starts_with("##") {
                // その後で ## を判定
                let cleaned_title =
                    strip_markdown_formatting(trimmed_line.trim_start_matches("##")).to_string();
                crate::graphics::drawing::draw_text(
                    &mut self.graphics.pixmap,
                    &self.graphics.font,
                    text_font_size * 1.2, // 見出しは少し大きく
                    &cleaned_title,
                    base_start_x,
                    current_y,
                    max_width,
                    line_height * 1.2,
                );
                current_y += line_height * 1.5; // 見出しの下は広めに
                continue;
            }

            // リストアイテムの処理
            let clean_line = strip_markdown_formatting(trimmed_line);

            // `:` でラベルと説明を分割
            if let Some((label_raw, description_raw)) = clean_line.split_once(':') {
                let label = label_raw.trim();
                let description = description_raw.trim();

                // ラベルを描画
                crate::graphics::drawing::draw_text(
                    &mut self.graphics.pixmap,
                    &self.graphics.font,
                    text_font_size,
                    label,
                    base_start_x + label_indent, // ラベルのインデント
                    current_y,
                    description_indent - label_indent - 5.0, // ラベルの最大幅
                    line_height,
                );

                // 説明を描画 (複数行対応は draw_text に任せるが、ここでは1行として描画)
                crate::graphics::drawing::draw_text(
                    &mut self.graphics.pixmap,
                    &self.graphics.font,
                    text_font_size,
                    description,
                    base_start_x + description_indent, // 説明のインデント
                    current_y,
                    max_width - description_indent, // 説明の最大幅
                    line_height,
                );
            } else {
                // `:` がない行はそのまま表示 (主にサブ項目や空行)
                crate::graphics::drawing::draw_text(
                    &mut self.graphics.pixmap,
                    &self.graphics.font,
                    text_font_size,
                    &clean_line,
                    base_start_x + label_indent, // デフォルトのインデント
                    current_y,
                    max_width,
                    line_height,
                );
            }
            current_y += line_height;
        }

        self.graphics.draw_finish();
    }
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

// テキストからMarkdownの書式設定を取り除くヘルパー関数
fn strip_markdown_formatting(text: &str) -> String {
    let mut cleaned_text = text.replace("**", ""); // 太字マーク ** を除去
    cleaned_text = cleaned_text.replace("`", ""); // コードマーク ` を除去
    // 行頭の "- " を除去 (trim の前に実行)
    // trimmed_start_str はオリジナルのtextから先頭の空白を取り除いた文字列スライス
    let trimmed_start_str = cleaned_text.trim_start();
    if trimmed_start_str.starts_with("- ") {
        // "- " を取り除いた文字列を生成
        cleaned_text = trimmed_start_str.strip_prefix("- ").unwrap().to_string();
    } else {
        // "- " で始まらない場合はそのまま
        cleaned_text = trimmed_start_str.to_string();
    }
    cleaned_text.trim().to_string() // 前後の空白をトリム
}
