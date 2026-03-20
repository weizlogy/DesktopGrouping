use std::collections::HashMap;
use windows::core::ComInterface;
use windows::Win32::Graphics::{
    Direct2D::{ID2D1DeviceContext, ID2D1SolidColorBrush, ID2D1RenderTarget, ID2D1Bitmap},
    Direct2D::Common::{D2D1_COLOR_F},
    DirectWrite::{IDWriteTextFormat, IDWriteFactory1, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL},
    Imaging::IWICImagingFactory,
};
use windows::Win32::UI::WindowsAndMessaging::HICON;
use crate::graphics::api::wic;

/// 描画リソース (ブラシやテキストフォーマット, ビットマップ) を管理するよ！
/// リソースの生成とキャッシュに責任を持つよ。
pub struct DrawingResources {
    brushes: HashMap<String, ID2D1SolidColorBrush>,
    bitmaps: HashMap<usize, ID2D1Bitmap>,
    pub dwrite_factory: IDWriteFactory1,
    wic_factory: IWICImagingFactory,
    text_format: Option<IDWriteTextFormat>,
    help_text_format: Option<IDWriteTextFormat>,
    current_font_family: String,
    current_font_size: f32,
}

impl DrawingResources {
    /// エンジンが保持するファクトリを受け取って初期化するよ。
    pub fn new(dwrite_factory: IDWriteFactory1, wic_factory: IWICImagingFactory) -> Self {
        Self {
            brushes: HashMap::new(),
            bitmaps: HashMap::new(),
            dwrite_factory,
            wic_factory,
            text_format: None,
            help_text_format: None,
            current_font_family: String::new(),
            current_font_size: 0.0,
        }
    }
// ... (中略)
    /// ヘルプ用のテキストフォーマットを取得するよ (折り返しあり)
    pub fn get_help_text_format(&mut self, font_family: &str, font_size: f32) -> Result<IDWriteTextFormat, windows::core::Error> {
        if let Some(format) = &self.help_text_format {
            if self.current_font_family == font_family && self.current_font_size == font_size {
                return Ok(format.clone());
            }
        }

        let family_wide = crate::win32::api::utils::to_wide(font_family);
        let format: IDWriteTextFormat = unsafe {
            let f = self.dwrite_factory.CreateTextFormat(
                windows::core::PCWSTR::from_raw(family_wide.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_size,
                windows::core::w!("ja-jp"),
            )?;
            // ヘルプ用なので左寄せ・上寄せ
            f.SetTextAlignment(windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT_LEADING)?;
            f.SetParagraphAlignment(windows::Win32::Graphics::DirectWrite::DWRITE_PARAGRAPH_ALIGNMENT_NEAR)?;
            // 折り返しを有効にする
            f.SetWordWrapping(windows::Win32::Graphics::DirectWrite::DWRITE_WORD_WRAPPING_WRAP)?;
            f
        };

        self.help_text_format = Some(format.clone());
        Ok(format)
    }

    /// 指定されたカラーコードからブラシを取得するよ。
    pub fn get_brush(
        &mut self,
        context: &ID2D1DeviceContext,
        color_hex: &str,
    ) -> Result<ID2D1SolidColorBrush, windows::core::Error> {
        if let Some(brush) = self.brushes.get(color_hex) {
            return Ok(brush.clone());
        }

        let color = parse_hex_to_d2d_color(color_hex);

        let brush = unsafe {
            let rt: ID2D1RenderTarget = context.cast()?;
            rt.CreateSolidColorBrush(&color, None)?
        };
        self.brushes.insert(color_hex.to_string(), brush.clone());
        Ok(brush)
    }

    /// デフォルトのテキストフォーマットを取得するよ。
    pub fn get_text_format(&mut self, font_family: &str, font_size: f32) -> Result<IDWriteTextFormat, windows::core::Error> {
        // フォント情報が変わっていない場合はキャッシュを返すよ
        if let Some(format) = &self.text_format {
            if self.current_font_family == font_family && self.current_font_size == font_size {
                return Ok(format.clone());
            }
        }

        // フォント情報を更新して新しく作成するよ
        let family_wide = crate::win32::api::utils::to_wide(font_family);
        let format: IDWriteTextFormat = unsafe {
            let f = self.dwrite_factory.CreateTextFormat(
                windows::core::PCWSTR::from_raw(family_wide.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_size,
                windows::core::w!("ja-jp"),
            )?;
            f.SetTextAlignment(windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT_CENTER)?;
            f.SetParagraphAlignment(windows::Win32::Graphics::DirectWrite::DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;

            // 1行に収めるための設定 (WordWrap を無効にし, Trimming を有効にする)
            f.SetWordWrapping(windows::Win32::Graphics::DirectWrite::DWRITE_WORD_WRAPPING_NO_WRAP)?;

            let trimming = windows::Win32::Graphics::DirectWrite::DWRITE_TRIMMING {
                granularity: windows::Win32::Graphics::DirectWrite::DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            let sign = self.dwrite_factory.CreateEllipsisTrimmingSign(&f)?;
            f.SetTrimming(&trimming, Some(&sign))?;

            f
        };

        self.text_format = Some(format.clone());
        self.current_font_family = font_family.to_string();
        self.current_font_size = font_size;
        Ok(format)
    }

    /// HICON から ID2D1Bitmap を取得 (キャッシュ付き)
    pub fn get_icon_bitmap(
        &mut self,
        context: &ID2D1DeviceContext,
        hicon: HICON,
    ) -> Result<ID2D1Bitmap, windows::core::Error> {
        let key = hicon.0 as usize;
        if let Some(bitmap) = self.bitmaps.get(&key) {
            return Ok(bitmap.clone());
        }

        let bitmap = wic::create_bitmap_from_hicon(context, &self.wic_factory, hicon)?;
        self.bitmaps.insert(key, bitmap.clone());
        Ok(bitmap)
    }
}

/// "#RRGGBBAA" または "#RRGGBB" 形式の文字列を D2D1_COLOR_F に変換するよ
fn parse_hex_to_d2d_color(hex: &str) -> D2D1_COLOR_F {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255) as f32 / 255.0;
    let a = if hex.len() >= 8 {
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f32 / 255.0
    } else {
        1.0
    };
    D2D1_COLOR_F { r, g, b, a }
}
