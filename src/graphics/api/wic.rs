use windows::Win32::Graphics::Imaging::{
    IWICImagingFactory, IWICBitmap, GUID_WICPixelFormat32bppPBGRA, CLSID_WICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom,
};
use windows::Win32::Graphics::Direct2D::{ID2D1DeviceContext, ID2D1Bitmap};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::WindowsAndMessaging::HICON;

/// WIC ファクトリを作成するよ！
pub fn create_factory() -> Result<IWICImagingFactory, windows::core::Error> {
    unsafe {
        CoCreateInstance(
            &CLSID_WICImagingFactory,
            None,
            CLSCTX_INPROC_SERVER,
        )
    }
}

/// HICON から Direct2D ビットマップを作成する変換関数 (ブラックボックス)
pub fn create_bitmap_from_hicon(
    context: &ID2D1DeviceContext,
    wic_factory: &IWICImagingFactory,
    hicon: HICON,
) -> Result<ID2D1Bitmap, windows::core::Error> {
    unsafe {
        // 1. HICON から WIC ビットマップを作成
        let wic_bitmap: IWICBitmap = wic_factory.CreateBitmapFromHICON(hicon)?;

        // 2. ピクセル形式を Direct2D が好む 32bppPBGRA (Premultiplied Alpha) に変換
        // (HICON によっては形式が異なる場合があるため, フォーマットコンバータを通すのが安全)
        let converter = wic_factory.CreateFormatConverter()?;
        converter.Initialize(
            &wic_bitmap,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;

        // 3. WIC ビットマップから Direct2D ビットマップを作成
        let d2d_bitmap = context.CreateBitmapFromWicBitmap(&converter, None)?;

        Ok(d2d_bitmap)
    }
}
