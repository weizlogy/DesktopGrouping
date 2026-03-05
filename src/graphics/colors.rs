use colorsys::{Hsl, Rgb};
use tiny_skia::Color;

// ---------------------------------------------------------
// --- 透過度定数 ---
const MIN_ALPHA: f32 = 0.05; // 色のアルファ値（透明度）が、これより小さくならないようにするための下限値だよ。あんまり透明すぎると見えなくなっちゃうからね！
// ---------------------------------------------------------

/// 色の文字列 (例: `"#RRGGBB"` や `"#RRGGBBAA"`) を `tiny_skia::Color` に変換するよ！
///
/// '#' があってもなくても大丈夫！6桁だったらアルファ値は不透明 (FF) になるよ。
/// もし変換できなかったら `None` を返すから、ちゃんとチェックしてね！
pub fn parse_color(color_str: &str) -> Option<Color> {
    // '#' があったら取り除いて、なかったらそのまま使うよ！
    let s = color_str.strip_prefix('#').unwrap_or(color_str);

    let (r_str, g_str, b_str, a_str) = match s.len() {
        6 => (
            s.get(0..2)?,
            s.get(2..4)?,
            s.get(4..6)?,
            "FF", // Alpha を FF (不透明) とする
        ),
        8 => (s.get(0..2)?, s.get(2..4)?, s.get(4..6)?, s.get(6..8)?),
        _ => return None, // 6桁でも8桁でもなければ無効
    };
    let r = u8::from_str_radix(r_str, 16).ok()?;
    let g = u8::from_str_radix(g_str, 16).ok()?;
    let b = u8::from_str_radix(b_str, 16).ok()?;
    let a = u8::from_str_radix(a_str, 16).ok()?;
    Color::from_rgba8(r, g, b, a).into() // tiny_skia::Color を返す
}

/// 色のアルファ値（透明度）を、`MIN_ALPHA` で定義された下限値に制限（クランプ）するよ！
///
/// あんまり透明にしすぎると見えなくなっちゃうから、それを防ぐためのおまじないなんだ♪
pub fn clamp_alpha(mut color: Color) -> Color {
    let alpha = color.alpha();
    if alpha < MIN_ALPHA {
        // 元の色情報 (RGB) を保持しつつアルファ値だけ変更
        color =
            Color::from_rgba(color.red(), color.green(), color.blue(), MIN_ALPHA).unwrap_or(color); // 失敗時は元の色を使う (ほぼありえない)
    }
    color
}

/// 背景色に基づいてホバー時の塗りつぶし色を計算するよ！
///
/// 背景色をHSLに変換して、輝度と彩度を調整し、半透明の強調色を生成するんだ。
pub fn calculate_hover_fill_color(base_color: Color) -> Color {
    let base_rgb = Rgb::new(
        base_color.red() as f64 * 255.0,
        base_color.green() as f64 * 255.0,
        base_color.blue() as f64 * 255.0,
        None,
    );
    let mut hsl: Hsl = base_rgb.into();

    let lightness = hsl.lightness();
    let saturation = hsl.saturation();

    // 輝度と彩度を調整
    let new_lightness = if lightness > 70.0 {
        // 明るい背景なら、少し暗くして透明度で調整
        (lightness * 0.8).clamp(0.0, 100.0)
    } else if lightness < 30.0 {
        // 暗い背景なら、少し明るくして透明度で調整
        (lightness * 1.2).clamp(0.0, 100.0)
    } else {
        // 中間なら、少しだけ明るくする
        (lightness + 10.0).clamp(0.0, 100.0)
    };

    let new_saturation = (saturation * 1.1).clamp(0.0, 100.0); // 彩度を少し上げる

    hsl.set_lightness(new_lightness);
    hsl.set_saturation(new_saturation);

    let adjusted_rgb: Rgb = hsl.into();

    // アルファ値は固定で半透明にする
    Color::from_rgba(
        adjusted_rgb.red() as f32 / 255.0,
        adjusted_rgb.green() as f32 / 255.0,
        adjusted_rgb.blue() as f32 / 255.0,
        0.2, // 20% の透明度でオーバーレイ
    )
    .unwrap()
}

/// 背景色に基づいてホバー時の枠線色を計算するよ！
///
/// 背景色をHSLに変換して、輝度と彩度を調整し、不透明度の高い強調色を生成するんだ。
pub fn calculate_hover_border_color(base_color: Color) -> Color {
    let base_rgb = Rgb::new(
        base_color.red() as f64 * 255.0,
        base_color.green() as f64 * 255.0,
        base_color.blue() as f64 * 255.0,
        None,
    );
    let mut hsl: Hsl = base_rgb.into();

    let lightness = hsl.lightness();
    let saturation = hsl.saturation();

    // 輝度を調整してコントラストを出す
    let new_lightness = if lightness > 50.0 {
        // 明るい背景なら、枠線は暗く
        (lightness - 30.0).clamp(0.0, 100.0)
    } else {
        // 暗い背景なら、枠線は明るく
        (lightness + 30.0).clamp(0.0, 100.0)
    };

    let new_saturation = (saturation * 1.2).clamp(0.0, 100.0); // 彩度を上げて目立たせる

    hsl.set_lightness(new_lightness);
    hsl.set_saturation(new_saturation);

    let adjusted_rgb: Rgb = hsl.into();

    // 枠線は不透明に近い形で
    Color::from_rgba(
        adjusted_rgb.red() as f32 / 255.0,
        adjusted_rgb.green() as f32 / 255.0,
        adjusted_rgb.blue() as f32 / 255.0,
        0.8, // 80% の透明度
    )
    .unwrap()
}
