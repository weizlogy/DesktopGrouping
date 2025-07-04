extern crate embed_resource;

fn main() {
    println!("cargo:rerun-if-changed=src/app.rc"); // 聖印の書が変更されたら再実行
    println!("cargo:rerun-if-changed=resource/stainedglassalpaca_highreso_7SD_icon.ico"); // アイコンファイルが変更されたら再実行
    embed_resource::compile("src/app.rc", embed_resource::NONE); // 聖印の書をコンパイルし埋め込む
}
