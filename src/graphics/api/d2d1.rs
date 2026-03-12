use windows::Win32::Graphics::{
    Direct2D::{
        Common::{D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT},
        D2D1CreateFactory, ID2D1Bitmap1, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1,
        D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
        D2D1_DEBUG_LEVEL_NONE, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_OPTIONS,
        D2D1_FACTORY_TYPE_SINGLE_THREADED,
    },
    Dxgi::{Common::DXGI_FORMAT_B8G8R8A8_UNORM, IDXGIDevice, IDXGISurface},
};

/// Direct2D ファクトリを作成するよ！
pub fn create_factory() -> Result<ID2D1Factory1, windows::core::Error> {
    unsafe {
        let options = D2D1_FACTORY_OPTIONS {
            debugLevel: D2D1_DEBUG_LEVEL_NONE,
        };
        D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, Some(&options))
    }
}

/// DXGI デバイスから Direct2D デバイスを作成するよ！
pub fn create_device(
    factory: &ID2D1Factory1,
    dxgi_device: &IDXGIDevice,
) -> Result<ID2D1Device, windows::core::Error> {
    unsafe { factory.CreateDevice(dxgi_device) }
}

/// Direct2D デバイスから描画用のデバイスコンテキストを作成するよ！
pub fn create_device_context(
    d2d_device: &ID2D1Device,
) -> Result<ID2D1DeviceContext, windows::core::Error> {
    unsafe { d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE) }
}

/// DXGI サーフェスから Direct2D ビットマップ（描画ターゲット）を作成するよ！
pub fn create_bitmap_from_dxgi_surface(
    context: &ID2D1DeviceContext,
    surface: &IDXGISurface,
) -> Result<ID2D1Bitmap1, windows::core::Error> {
    unsafe {
        let props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            ..Default::default()
        };
        context.CreateBitmapFromDxgiSurface(surface, Some(&props))
    }
}
