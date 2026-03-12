use windows::Win32::Graphics::Direct2D::{
    ID2D1DeviceContext, D2D1_ROUNDED_RECT, ID2D1SolidColorBrush,
};

/// 矩形の背景と枠線を描画するよ！
/// 描画に必要なリソースは外部 (Resources) から提供される前提だよ。
pub fn draw_rounded_rect(
    context: &ID2D1DeviceContext,
    rect: &windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F,
    fill_brush: &ID2D1SolidColorBrush,
    border_brush: Option<&ID2D1SolidColorBrush>,
    border_width: f32,
    radius: f32,
) {
    let rounded_rect = D2D1_ROUNDED_RECT {
        rect: *rect,
        radiusX: radius,
        radiusY: radius,
    };

    unsafe {
        // 背景を塗りつぶす
        context.FillRoundedRectangle(&rounded_rect, fill_brush);

        // 枠線を描画する
        if let Some(brush) = border_brush {
            if border_width > 0.0 {
                context.DrawRoundedRectangle(&rounded_rect, brush, border_width, None);
            }
        }
    }
}
