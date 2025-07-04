// src/graphics/mod.rs

// サブモジュールを宣言
pub mod colors;
pub mod drawing;
pub mod graphics;
pub mod layout;

// MyGraphics 構造体と、外部から使う必要のある関数や型を公開
pub use colors::parse_color;
