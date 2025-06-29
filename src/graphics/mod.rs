// src/graphics/mod.rs

// サブモジュールを宣言
pub mod colors;
pub mod layout;
pub mod drawing;
pub mod graphics;

// MyGraphics 構造体と、外部から使う必要のある関数や型を公開
pub use colors::parse_color;
