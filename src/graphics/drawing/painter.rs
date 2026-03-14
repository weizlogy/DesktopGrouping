use windows::Win32::Graphics::Direct2D::ID2D1DeviceContext;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;
use crate::graphics::drawing::{background, label, icon, resources::DrawingResources};
use crate::graphics::layout;
use crate::ui::group::model::GroupModel;
use crate::win32::api::shell;

/// グループ全体を描画するメインコーディネーターだよ！
pub fn draw_group(
    context: &ID2D1DeviceContext,
    width: f32,
    height: f32,
    model: &GroupModel,
    resources: &mut DrawingResources,
) -> Result<(), windows::core::Error> {
    // 1. 背景と枠線の描画
    let bg_rect = D2D_RECT_F { left: 0.0, top: 0.0, right: width, bottom: height };
    let bg_brush = resources.get_brush(context, &model.bg_color_hex)?;
    let border_brush = resources.get_brush(context, "#FFFFFF33")?; 

    unsafe {
        bg_brush.SetOpacity(model.opacity);
        border_brush.SetOpacity(model.opacity * 0.5);
    }

    background::draw_rounded_rect(context, &bg_rect, &bg_brush, Some(&border_brush), 1.5, 8.0);

    // 2. アイコンとラベルの描画
    if !model.icons.is_empty() {
        let layouts = layout::calculate_grid_layout(width, model.icons.len(), 1.0);
        let icon_label_brush = resources.get_brush(context, "#000000FF")?;
        let format = resources.get_text_format()?;
        
        let highlight_bg_brush = resources.get_brush(context, "#FFFFFF22")?; 
        let highlight_border_brush = resources.get_brush(context, "#FFFFFF66")?;
        
        // 実行中のフィードバック用ブラシ (より明るく!)
        let executing_bg_brush = resources.get_brush(context, "#FFFFFF66")?;
        let executing_border_brush = resources.get_brush(context, "#FFFFFFFF")?;

        for (i, icon_state) in model.icons.iter().enumerate() {
            if let Some(layout) = layouts.get(i) {
                
                // フィードバック描画 (実行中 > ホバー)
                if model.executing_index == Some(i) {
                    background::draw_rounded_rect(
                        context, &layout.hit_rect, &executing_bg_brush, Some(&executing_border_brush), 1.5, 4.0,
                    );
                } else if model.hovered_index == Some(i) {
                    background::draw_rounded_rect(
                        context, &layout.hit_rect, &highlight_bg_brush, Some(&highlight_border_brush), 1.0, 4.0,
                    );
                }

                // アイコン取得と描画
                if let Some(hicon) = shell::get_icon_for_path(&icon_state.path) {
                    if let Ok(bitmap) = resources.get_icon_bitmap(context, hicon) {
                        icon::draw_icon(context, &bitmap, &layout.icon_rect, 1.0);
                    }
                    unsafe { DestroyIcon(hicon).ok(); }
                }

                // ラベル描画
                label::draw_text(context, &icon_state.name, &layout.text_rect, &icon_label_brush, &format);
            }
        }
    }

    Ok(())
}
