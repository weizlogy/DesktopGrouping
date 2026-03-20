use std::rc::Rc;
use windows::Win32::{
    Foundation::HWND,
    Graphics::{
        Direct2D::{ID2D1DeviceContext, ID2D1Bitmap1},
        DirectComposition::{IDCompositionTarget, IDCompositionVisual},
        Dxgi::IDXGISwapChain1,
        // Dxgi::Common::DXGI_FORMAT_UNKNOWN,
    },
};
use crate::graphics::{api, GraphicsEngine};

/// ウィンドウごとの描画領域（キャンバス）を管理するよ！
/// スワップチェーンやデバイスコンテキストを保持し, 低負荷な描画を実現するよ。
pub struct Canvas {
    engine: Rc<GraphicsEngine>,
    swap_chain: IDXGISwapChain1,
    pub d2d_context: ID2D1DeviceContext,
    pub comp_target: IDCompositionTarget,  // never read 対策
    pub comp_visual: IDCompositionVisual,  // never read 対策
}

impl Canvas {
    /// 新しいキャンバスを作成するよ。
    /// width, height は初期サイズを指定してね。
    pub fn new(
        engine: Rc<GraphicsEngine>,
        hwnd: HWND,
        width: u32,
        height: u32,
    ) -> Result<Self, windows::core::Error> {
        // 1. スワップチェーンの作成 (DirectComposition 用)
        let swap_chain = api::dxgi::create_swap_chain_for_composition(
            &engine.d3d_device,
            width,
            height,
        )?;

        // 2. 描画コンテキストの作成
        let d2d_context = api::d2d1::create_device_context(&engine.d2d_device)?;

        // 3. DirectComposition のセットアップ (透過ウィンドウ合成用)
        let comp_target = api::dcomp::create_target_for_hwnd(&engine.dcomp_device, hwnd, true)?;
        let comp_visual = api::dcomp::create_visual(&engine.dcomp_device)?;

        unsafe {
            comp_visual.SetContent(&swap_chain)?;
            comp_target.SetRoot(&comp_visual)?;
            engine.dcomp_device.Commit()?;
        }

        let mut canvas = Self {
            engine,
            swap_chain,
            d2d_context,
            comp_target,
            comp_visual,
        };

        // レンダリングターゲット (ビットマップ) をスワップチェーンに紐付ける
        canvas.setup_render_target()?;

        Ok(canvas)
    }

    /// スワップチェーンのバックバッファを描画ターゲットとして設定するよ。
    pub fn setup_render_target(&mut self) -> Result<(), windows::core::Error> {
        unsafe {
            // スワップチェーンからバックバッファ (DXGI Surface) を取得
            let back_buffer = self.swap_chain.GetBuffer::<windows::Win32::Graphics::Dxgi::IDXGISurface>(0)?;

            // DXGI Surface から Direct2D ビットマップを作成
            let d2d_bitmap: ID2D1Bitmap1 = api::d2d1::create_bitmap_from_dxgi_surface(&self.d2d_context, &back_buffer)?;

            // コンテキストの描画先として設定
            self.d2d_context.SetTarget(&d2d_bitmap);

            // DPI を標準値に固定して, 座標計算のズレを防ぐよ！
            self.d2d_context.SetDpi(96.0, 96.0);
        }
        Ok(())
    }

    /// ウィンドウサイズが変わったときに呼び出してね。
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), windows::core::Error> {
        if width == 0 || height == 0 {
            return Ok(());
        }

        unsafe {
            // 1. ターゲットを解除 (リサイズ前に必須)
            self.d2d_context.SetTarget(None);

            // 2. スワップチェーンのバッファをリサイズ
            self.swap_chain.ResizeBuffers(
                0, // 現状維持
                width,
                height,
                windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN,
                0,
            )?;

            // 3. レンダリングターゲットを再構築
            self.setup_render_target()?;
        }
        Ok(())
    }

    /// 描画を開始するよ。
    pub fn begin_draw(&self) {
        unsafe {
            self.d2d_context.BeginDraw();
            // 背景を透明でクリア
            self.d2d_context.Clear(None);
        }
    }

    /// 描画を確定して画面に反映するよ。
    pub fn end_draw(&self) -> Result<(), windows::core::Error> {
        unsafe {
            // 1. Direct2D の描画完了
            self.d2d_context.EndDraw(None, None)?;

            // 2. スワップチェーンのバッファを入れ替え (画面更新)
            // 第1引数 1 は VSync 同期
            self.swap_chain.Present(1, 0).ok()?;

            // 3. DirectComposition の変更を確定
            self.engine.dcomp_device.Commit()?;
        }
        Ok(())
    }
}
