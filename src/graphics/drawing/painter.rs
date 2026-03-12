use windows::Win32::Graphics::Direct2D::ID2D1DeviceContext;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use crate::graphics::drawing::{background, label, resources::DrawingResources};

/// グループ全体を描画するメインコーディネーターだよ！
/// Resources から必要な部品を揃えて, 各描画関数に仕事を振り分けるよ。
pub fn draw_group(
    context: &ID2D1DeviceContext,
    width: f32,
    height: f32,
    bg_color_hex: &str,
    resources: &mut DrawingResources,
) -> Result<(), windows::core::Error> {
    // 1. 背景と枠線の描画準備
    let bg_rect = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: width,
        bottom: height,
    };
    let bg_brush = resources.get_brush(context, bg_color_hex)?;
    let border_brush = resources.get_brush(context, "#FFFFFF33")?; // 半透明の白い枠線

    background::draw_rounded_rect(
        context,
        &bg_rect,
        &bg_brush,
        Some(&border_brush),
        1.5,
        8.0,
    );

    // 2. タイトルの描画準備
    let title_rect = D2D_RECT_F {
        left: 10.0,
        top: 5.0,
        right: width - 10.0,
        bottom: 30.0,
    };
    let title_brush = resources.get_brush(context, "#FFFFFFFF")?;
    let format = resources.get_text_format()?;

    label::draw_text(
        context,
        "Group Title",
        &title_rect,
        &title_brush,
        &format,
    );

    // TODO: アイコンの描画を実際に行うには, HICON のリストなどが必要だよ。
    // 今回は枠組みだけ作っておくね。

    Ok(())
}
