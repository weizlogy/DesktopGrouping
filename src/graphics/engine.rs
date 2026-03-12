use windows::Win32::Graphics::{
    Direct2D::{ID2D1Device, ID2D1Factory1},
    Direct3D11::{ID3D11Device, ID3D11DeviceContext},
    DirectComposition::IDCompositionDevice,
    DirectWrite::IDWriteFactory1,
    Imaging::IWICImagingFactory,
};
use crate::graphics::api;

/// アプリケーション全体で共有されるグラフィックスの「心臓部」だよ！
/// D3D/D2D デバイスや各ファクトリを保持するよ。
pub struct GraphicsEngine {
    pub d3d_device: ID3D11Device,
    pub d3d_context: ID3D11DeviceContext,
    pub d2d_factory: ID2D1Factory1,
    pub d2d_device: ID2D1Device,
    pub dwrite_factory: IDWriteFactory1,
    pub dcomp_device: IDCompositionDevice,
    pub wic_factory: IWICImagingFactory,
}

impl GraphicsEngine {
    /// グラフィックスエンジンを初期化するよ！
    pub fn new() -> Result<Self, windows::core::Error> {
        let (d3d_device, d3d_context) = api::d3d11::create_device()?;
        let dxgi_device = api::dxgi::get_dxgi_device(&d3d_device)?;
        let d2d_factory = api::d2d1::create_factory()?;
        let d2d_device = api::d2d1::create_device(&d2d_factory, &dxgi_device)?;
        let dwrite_factory = api::dwrite::create_factory()?;
        let dcomp_device = api::dcomp::create_device(&dxgi_device)?;
        let wic_factory = api::wic::create_factory()?;

        log::info!("GraphicsEngine initialized successfully.");
        
        Ok(Self {
            d3d_device,
            d3d_context,
            d2d_factory,
            d2d_device,
            dwrite_factory,
            dcomp_device,
            wic_factory,
        })
    }
}
