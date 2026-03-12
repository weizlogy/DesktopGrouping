use windows::core::ComInterface;
use windows::Win32::Graphics::Dxgi::{
    Common::{
        DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
    },
    CreateDXGIFactory2, IDXGIDevice, IDXGIFactory2, IDXGISwapChain1,
    DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
    DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use windows::Win32::Graphics::Direct3D11::ID3D11Device;

/// D3D11 デバイスから DXGI デバイスインターフェースを取得するよ！
pub fn get_dxgi_device(d3d_device: &ID3D11Device) -> Result<IDXGIDevice, windows::core::Error> {
    d3d_device.cast::<IDXGIDevice>()
}

/// DXGI ファクトリを作成するよ！
pub fn create_dxgi_factory() -> Result<IDXGIFactory2, windows::core::Error> {
    unsafe {
        CreateDXGIFactory2(0)
    }
}

/// ダイレクトコンポジション（透過合成）用のスワップチェーンを作成するよ！
pub fn create_swap_chain_for_composition(
    d3d_device: &ID3D11Device,
    width: u32,
    height: u32,
) -> Result<IDXGISwapChain1, windows::core::Error> {
    unsafe {
        let factory: IDXGIFactory2 = CreateDXGIFactory2(0)?;

        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width,
            Height: height,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM, // Direct2D と相性が良いフォーマット
            Stereo: false.into(),
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED, // 透過合成を許可！
            Flags: 0,
        };

        factory.CreateSwapChainForComposition(d3d_device, &desc, None)
    }
}
