use windows::Win32::Graphics::Direct2D::ID2D1DeviceContext;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use crate::graphics::drawing::{background, label, resources::DrawingResources};
use crate::ui::group::model::GroupModel;

/// グループ全体を描画するメインコーディネーターだよ！
/// Resources から必要な部品を揃えて, 各描画関数に仕事を振り分けるよ。
pub fn draw_group(
    context: &ID2D1DeviceContext,
    width: f32,
    height: f32,
    model: &GroupModel,
    resources: &mut DrawingResources,
) -> Result<(), windows::core::Error> {
    // 1. 背景と枠線の描画準備
    let bg_rect = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: width,
        bottom: height,
    };
    let bg_brush = resources.get_brush(context, &model.bg_color_hex)?;
    let border_brush = resources.get_brush(context, "#FFFFFF33")?; // 半透明の白い枠線

    // ウィンドウ全体の透明度を適用するよ
    unsafe {
        bg_brush.SetOpacity(model.opacity);
        border_brush.SetOpacity(model.opacity * 0.5); // 枠線はさらに薄く
    }

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

    unsafe {
        title_brush.SetOpacity(model.opacity);
    }

    label::draw_text(
        context,
        &model.title,
        &title_rect,
        &title_brush,
        &format,
    );

    Ok(())
}
