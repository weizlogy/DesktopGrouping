use windows::Win32::Graphics::Direct2D::ID2D1DeviceContext;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;
use crate::graphics::drawing::{background, label, icon, resources::DrawingResources};
use crate::graphics::layout;
use crate::ui::group::model::GroupModel;
use crate::win32::api::shell;

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

    // 背景にのみ透過度を適用するよ！ (仕様変更: アイコンやラベルは透過させない)
    unsafe {
        bg_brush.SetOpacity(model.opacity);
        border_brush.SetOpacity(model.opacity * 0.5); // 枠線は背景に合わせて薄く
    }

    background::draw_rounded_rect(
        context,
        &bg_rect,
        &bg_brush,
        Some(&border_brush),
        1.5,
        8.0,
    );

    // 2. タイトルの描画 (仕様変更: デスクトップをクリーンに保つため廃止)
    /*
    let title_brush = resources.get_brush(context, "#FFFFFFFF")?;
    let format = resources.get_text_format()?;
    unsafe { title_brush.SetOpacity(1.0); } // 常に不透明
    // ... draw_text ...
    */

    // 3. アイコンとラベルの描画 (グリッド配置)
    if !model.icons.is_empty() {
        let layouts = layout::calculate_grid_layout(width, model.icons.len(), 1.0);
        let icon_label_brush = resources.get_brush(context, "#000000FF")?; // 黒に変更！
        let format = resources.get_text_format()?;
        
        // アイコンとラベルは常に不透明で描画するよ！
        unsafe {
            icon_label_brush.SetOpacity(1.0);
        }

        for (i, icon_state) in model.icons.iter().enumerate() {
            if let Some(layout) = layouts.get(i) {
                // Shell API からアイコンを取得
                if let Some(hicon) = shell::get_icon_for_path(&icon_state.path) {
                    // HICON を Direct2D ビットマップに変換 (キャッシュ利用)
                    if let Ok(bitmap) = resources.get_icon_bitmap(context, hicon) {
                        // アイコンを描画 (仕様変更: 不透明度 1.0)
                        icon::draw_icon(context, &bitmap, &layout.icon_rect, 1.0);
                    }
                    
                    // 使い終わった HICON を解放
                    unsafe {
                        DestroyIcon(hicon).ok();
                    }
                }

                // アイコン名を描画 (中央寄せ)
                label::draw_text(
                    context,
                    &icon_state.name,
                    &layout.text_rect,
                    &icon_label_brush,
                    &format,
                );
            }
        }
    }

    Ok(())
}
