use std::rc::Rc;

// use rand::Rng; // rand::Rng は使われてないみたいだから、コメントアウトしちゃおっか！
use tiny_skia::Color;
use winit::{
    dpi::PhysicalSize,
    window::{ResizeDirection, Window},
};

use desktop_grouping::graphics::{self, graphics::MyGraphics};
// logger モジュールは start_os_drag などで使ってるから、ちゃんと use しとかないとね！
use crate::{file_drag::IconInfo, logger::*};

/// マウスホイールによるアルファ値調整のステップ量
const ALPHA_ADJUST_STEP: f32 = 0.02;

/// 子ウィンドウを表す構造体。
/// winit の `Window`、描画を担当する `MyGraphics`、
/// グループ化されたアイコン (`IconInfo`) のリスト、
/// そして設定ファイルと紐付けるための識別子 (`id_str`) を保持します。
pub struct ChildWindow {
    /// winit のウィンドウインスタンスへの参照カウンタ付きポインタ。
    pub window: Rc<Window>,
    /// このウィンドウ専用のグラフィックス描画インスタンス。
    pub graphics: MyGraphics,
    /// このウィンドウ内に配置されたアイコン情報のベクター。
    pub groups: Vec<IconInfo>,
    /// 設定ファイル (`config.toml`) 内の `[children]` テーブルと
    /// このウィンドウインスタンスを紐付けるためのユニークな文字列ID。
    /// 通常は生成時のタイムスタンプ。
    pub id_str: String,
    /// このウィンドウの現在のDPIスケーリングファクターだよ！
    pub scale_factor: f64,
}

impl ChildWindow {
    /// 新しい子ウィンドウインスタンスを作るよ！
    ///
    /// ウィンドウの実体 (`Rc<Window>`) と、ユニークなID文字列、
    /// それから最初の背景色と枠線の色をもらって、`ChildWindow` を初期化するんだ。
    /// グラフィックスの初期化もここで行うよ！
    ///
    /// # 引数
    /// * `window` - winit のウィンドウインスタンスだよ。`Rc` で包んでね！
    /// * `id_str` - この子ウィンドウちゃんを識別するためのユニークな文字列IDだよ。
    /// * `bg_color_str` - 背景色の初期値を文字列で指定してね (例: `"#RRGGBBAA"`)。
    pub fn new(
        window: Rc<Window>,
        id_str: String,
        bg_color_str: &str,
    ) -> ChildWindow {
        // ウィンドウが作られた時の最初の拡大率を覚えておくよ！
        let initial_scale_factor = window.scale_factor();
        // MyGraphics ちゃんにも最初の拡大率を教えてあげるんだ♪
        let graphics = MyGraphics::new(
            &window,
            bg_color_str,
            initial_scale_factor,
        );
        ChildWindow {
            window,
            graphics,
            groups: Vec::new(),
            id_str,
            scale_factor: initial_scale_factor,
        }
    }

    /// 拡大率 (`scale_factor`) が変わった時に呼び出すよ！
    pub fn update_scale_factor(&mut self, new_scale_factor: f64) {
        self.scale_factor = new_scale_factor;
        self.graphics.update_scale_factor(new_scale_factor); // MyGraphics にも拡大率の変更を伝えるよ！
    }

    /// 背景色を設定するよ！
    ///
    /// 新しい背景色を文字列で受け取って、それをパースして適用するんだ。
    /// 最後に、ウィンドウに「再描画お願いね！」って伝えるんだ♪
    pub fn set_background_color(&mut self, color_str: &str) {
        if let Some(bg_color) = graphics::parse_color(color_str) {
            self.graphics.update_background_color(bg_color);
            self.window.request_redraw();
            log_debug(&format!(
                "Window {}: BG set to {}",
                self.id_str,
                color_to_hex_string(bg_color),
            ));
        } else {
            log_warn(&format!(
                "Window {}: Invalid color string received: {}",
                self.id_str, color_str
            ));
        }
    }

    /// 背景色の透過度を調整するよ！
    ///
    /// `delta` の値に応じて、今の背景色のアルファ値（透明度）をちょっとずつ変えるんだ。
    /// `ALPHA_ADJUST_STEP` で、どれくらい変えるか調整できるよ！
    /// 透明度を変えたら、再描画をお願いするよ！
    pub fn adjust_alpha(&mut self, delta: f32) {
        let current_bg_color = self.graphics.get_background_color();
        let current_alpha = current_bg_color.alpha();
        let new_alpha = (current_alpha + delta * ALPHA_ADJUST_STEP).clamp(0.0, 1.0);

        if (new_alpha - current_alpha).abs() > f32::EPSILON {
            let new_bg_color = Color::from_rgba(
                current_bg_color.red(),
                current_bg_color.green(),
                current_bg_color.blue(),
                new_alpha,
            )
            .unwrap();

            self.graphics.update_background_color(new_bg_color);
            self.window.request_redraw();
            log_debug(&format!(
                "Window {}: Alpha adjusted to {:.3}",
                self.id_str,
                new_alpha,
            ));
        }
    }

    /// この子ウィンドウにアイコン情報を追加するよ！
    ///
    /// `IconInfo` を受け取って、ウィンドウが持ってるアイコンのリスト (`groups`) に追加するだけ！シンプルだね！
    pub fn add(&mut self, icon: IconInfo) {
        self.groups.push(icon);
    }

    /// ウィンドウのサイズが変更されたときに、グラフィックス側にも教えてあげるよ！
    ///
    /// `MyGraphics` ちゃんが持ってるバッファとかを、新しいサイズに合わせて調整してもらうんだ。
    pub fn resize_graphics(&mut self, new_size: PhysicalSize<u32>) {
        self.graphics.resize(new_size);
    }

    /// ウィンドウの内容を描画するよ！
    ///
    /// まず `graphics.draw_start()` でお絵かきの準備をして、
    /// 持ってるアイコン (`groups`) を一つずつ `graphics.draw_group()` で描いてもらうんだ。
    /// もし `hovered_index` が指定されてたら、そのアイコンはちょっと目立つように描かれるかも！
    /// 最後に `graphics.draw_finish()` で画面に表示するよ！
    pub fn draw(&mut self, hovered_index: Option<usize>, executing_index: Option<usize>) {
        self.graphics.draw_start();

        // iter_mut() を使うことで、各 icon_info を可変で借用できる。
        // これにより、get_or_load_icon() 内でキャッシュの更新が可能になる。
        for (index, icon_info) in self.groups.iter_mut().enumerate() {
            let is_hovered = hovered_index.map_or(false, |h_idx| h_idx == index);
            let is_executing = executing_index.map_or(false, |e_idx| e_idx == index);

            // 借用(borrow)の競合を避けるため、先に不変の借用で名前をクローンしておく。
            // これにより、get_or_load_icon()による可変の借用と、名前の参照が同時に存在しなくなる。
            let name = icon_info.name.clone();

            // 描画が必要なこのタイミングで、アイコンデータを取得（または遅延読み込み）
            let icon_data = icon_info.get_or_load_icon();
            self.graphics
                .draw_group(index, &name, icon_data, is_hovered, is_executing);
        }
        self.graphics.draw_finish();
    }

    // --- OSへの指示を出すメソッドたちだよ！ ---
    // これらは、ウィンドウマネージャーさん (OS) に「ちょっとこれお願い！」って伝えるためのものだよ。
    // エラーが起きてもパニックしないで、ログに記録するようになってるんだ。えらい！

    /// このウィンドウをOSレベルでドラッグ開始するよう指示するよ！
    /// ユーザーがウィンドウを掴んで動かせるようにするんだ。
    pub fn start_os_drag(&self) {
        if let Err(e) = self.window.drag_window() {
            crate::logger::log_error(&format!(
                "Window drag_window failed for {:?}: {}",
                self.id_str, e
            ));
        }
    }

    /// このウィンドウをOSレベルでリサイズ開始するよう指示するよ！
    /// ユーザーがウィンドウの端を掴んで大きさを変えられるようにするんだ。どの方向かは `direction` で指定するよ。
    pub fn start_os_resize(&self, direction: ResizeDirection) {
        if let Err(e) = self.window.drag_resize_window(direction) {
            crate::logger::log_error(&format!(
                "Window drag_resize_window failed for {:?} (dir: {:?}): {}",
                self.id_str, direction, e
            ));
        }
    }

    // backmost の処理は ui_wam を使うから、WindowManager 側で child.window を渡す形の方が素直かも。
}

/// `tiny_skia::Color` を `#RRGGBBAA` 形式の文字列に変換するよ！
/// 設定ファイルに保存するときとかに便利だね！
pub fn color_to_hex_string(color: Color) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        (color.red() * 255.0) as u8,
        (color.green() * 255.0) as u8,
        (color.blue() * 255.0) as u8,
        (color.alpha() * 255.0) as u8
    )
}

#[cfg(test)]
mod tests {
    use super::*; // child_window.rs の中身をぜーんぶ使えるようにするおまじない！
    // EventLoop や WindowBuilder を使うテストは、実行環境に依存しやすかったり、
    // ちょっと重かったりするから、今回はお休みしてもらうね！ (<em>´ω｀</em>)
    // ChildWindow のインスタンスを作るテスト (new, set_background_color, adjust_alpha, add, start_os_drag, start_os_resize) は
    // Window オブジェクトが必要になるから、ここでは一旦コメントアウト、または削除するよ。
    //
    // もし、これらのロジックをテストしたい場合は、
    // Window や MyGraphics のモック (偽物オブジェクト) を作ってテストする方法があるよ！
    // ちょっと上級者向けだけど、いつか挑戦してみるのも楽しいかも！(๑•̀ㅂ•́)و✧

    #[test]
    fn test_color_to_hex_string() {
        let color1 = Color::from_rgba8(255, 0, 0, 255); // 赤
        assert_eq!(color_to_hex_string(color1), "#FF0000FF");

        let color2 = Color::from_rgba8(0, 255, 0, 128); // 半透明の緑
        assert_eq!(color_to_hex_string(color2), "#00FF0080");

        let color3 = Color::from_rgba8(16, 32, 48, 0); // 透明な暗い青 (10203000)
        assert_eq!(color_to_hex_string(color3), "#10203000");
    }
}
