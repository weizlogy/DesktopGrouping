/// モジュール化の方法
///   - フォルダーを用意する（ex: test/）
///   - モジュールの実装ファイルを作る（ex: test1.rs）
///   - mod.rsを用意する
///     > pub mod test1; // と記載する
///   - src直下にlib.rsを用意する
///     > pub mod test;  // と記載する
///   - 使いたいところでuseする
///     > use <project_name>::test::test1;  // みたいになる

pub mod win32;
pub mod tray;
pub mod graphics;

// pub mod aaa {
//   pub fn moduletest() {}
// }
