use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory1, DWRITE_FACTORY_TYPE_SHARED,
};

/// DirectWrite ファクトリを作成するよ！
pub fn create_factory() -> Result<IDWriteFactory1, windows::core::Error> {
    unsafe {
        DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)
    }
}
