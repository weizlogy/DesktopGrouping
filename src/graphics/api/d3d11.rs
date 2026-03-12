use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_SDK_VERSION,
};

/// D3D11 デバイスとデバイスコンテキストを作成するよ！
/// Direct2D と連携するために BGRA サポートを有効にするのがポイントだよ。
pub fn create_device() -> Result<(ID3D11Device, ID3D11DeviceContext), windows::core::Error> {
    unsafe {
        let mut device = None;
        let mut context = None;
        let mut feature_level = D3D_FEATURE_LEVEL_11_0;

        // 試行する機能レベルのリスト
        let feature_levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];

        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT, // これがないと Direct2D が動かないよ
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut feature_level),
            Some(&mut context),
        )?;

        Ok((device.unwrap(), context.unwrap()))
    }
}
