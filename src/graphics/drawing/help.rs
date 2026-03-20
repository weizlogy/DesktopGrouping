use windows::Win32::Graphics::Direct2D::ID2D1DeviceContext;
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D_POINT_2F};
use crate::graphics::drawing::resources::DrawingResources;

/// 操作説明テキストを定数として定義
const OPERATION_INSTRUCTIONS: [&str; 15] = [
    "## 操作説明",
    "### ■ Create Groups:",
    "  - Right-click: トレイアイコンを右クリックしてメニューを表示し New Group.",
    "  - Drag & Drop: ファイルをドラッグ＆ドロップしてグループに簡単に追加できます。",
    "### ■ Icons:",
    "  - Left-double-click: アプリケーションが起動またはファイルが開きます。",
    "  - Right-click: そのファイルがあるフォルダが開きます。",
    "  - Ctrl + Right-click: そのアイコンを削除します。",
    "### ■ Customization:",
    "  - Move: Ctrl + ドラッグ でグループを移動します。",
    "  - Resize: Shift + ドラッグ でグループのサイズを変更します。",
    "  - Color: Ctrl + V でカラーコード (#FF0000) や「#Random」を貼り付け。",
    "  - Transparency: Alt + ドラッグ で透明度を調整します。",
    "### ■ Delete Groups:",
    "  - Ctrl + Right-click: グループの何もない場所を右クリックして削除。",
];

/// ヘルプ（操作ガイド）を描画する専用の関数だよ！
pub fn draw_help(
    context: &ID2D1DeviceContext,
    width: f32,
    height: f32, // 未使用だけどシグネチャ維持
    text_color_hex: &str,
    resources: &mut DrawingResources,
) -> Result<(), windows::core::Error> {
    let settings = crate::settings::manager::get_settings_reader();
    let font_family = &settings.app.font_family;
    let base_font_size = settings.app.font_size * 1.2;
    let brush = resources.get_brush(context, text_color_hex)?;
    let format = resources.get_help_text_format(font_family, base_font_size)?;
    let dwrite_factory = resources.dwrite_factory.clone();
    drop(settings);

    let padding = 20.0;
    let mut current_y = padding;
    let max_text_width = width - padding * 2.0;

    for line in OPERATION_INSTRUCTIONS.iter() {
        let trimmed_line = line.trim();
        let mut x_offset = padding;

        // 簡単な Markdown 記法のパース
        let display_text = if trimmed_line.starts_with("## ") {
            &trimmed_line[3..]
        } else if trimmed_line.starts_with("### ") {
            &trimmed_line[4..]
        } else if trimmed_line.starts_with("  - ") {
            x_offset = padding * 2.0;
            &trimmed_line[4..]
        } else {
            trimmed_line
        };

        if display_text.is_empty() {
            current_y += base_font_size;
            continue;
        }

        // DirectWrite TextLayout を使って、折り返し後の高さを正確に測るよ！
        let wide_text: Vec<u16> = display_text.encode_utf16().collect();
        unsafe {
            let layout = dwrite_factory.CreateTextLayout(
                &wide_text,
                &format,
                max_text_width - (x_offset - padding),
                1000.0, // 十分な高さ
            )?;

            let mut metrics = windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut metrics)?;

            // 描画実行
            context.DrawTextLayout(
                D2D_POINT_2F { x: x_offset, y: current_y },
                &layout,
                &brush,
                windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS_NONE,
            );

            // 描画した高さ分だけ Y 座標を進める
            current_y += metrics.height + (base_font_size * 0.4);
        }
    }

    Ok(())
}
