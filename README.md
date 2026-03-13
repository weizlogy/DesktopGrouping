# Desktop Grouping (v3.0.0) 🧹✨

Windows デスクトップ上のアイコンを「グループ（フェンス）」で整理整頓するための、超軽量・高画質なデスクトップユーティリティです。

## 🌟 プロジェクトの目的と設計思想

Desktop Grouping は、デスクトップを綺麗に保ちつつ、作業効率を最大化することを目的としています。
v3.0.0 では、**「常駐ソフトとしての究極の低負荷」** と **「現代の Windows にふさわしい美しい透過表現」** を追求し、技術スタックを Win32 ネイティブ + DirectX 11 へ刷新しました。

### 🚀 技術スタック (v3.0.0)
- **Language**: Rust (edition 2024)
- **Windowing**: Win32 Native API (No abstraction layers like winit)
- **Graphics**: DirectX 11 + Direct2D + DirectWrite
- **Composition**: DirectComposition (Hardware-accelerated transparency)
- **Performance**: アイドル時の CPU 使用率 0% を目指したイベント駆動型設計

---

## ✨ 主要機能

### 1. アイコンのグループ化 (Groups)
- **作成**: トレイアイコンの右クリックメニューから「New Group」を選択。
- **整理**: ファイルやショートカットをグループ内にドラッグ＆ドロップで追加。
- **実行**: アイコンをダブルクリックして実行。
- **場所確認**: 右クリックでファイルの場所（エクスプローラー）を開く。

### 2. 直感的なカスタマイズ (Shortcut Keys)
グループを直接操作して、好みのスタイルに調整できます。
- **移動**: `Ctrl + 左ドラッグ`
- **リサイズ**: `Shift + 左ドラッグ`
- **透過度調整**: `Alt + 左ドラッグ` (左右に動かすことで不透明度を連続的に変更)
- **色変更**: 16進数カラーコード（例: `#FF0000`）をクリップボードにコピーした状態でグループ上で `Ctrl + V`。`#random` でランダムな色に変更。
- **削除**: 空白部分で `Ctrl + 右クリック` (未実装)

### 3. システムトレイ常駐
- 常にバックグラウンドで動作し、トレイアイコンから設定や終了が可能です。

---

## 🛠️ 開発者向け情報

### ビルド要件
- Rust ツールチェーン (edition 2024 以降)
- Windows SDK (DirectX 関連のビルドに必要)

### ビルド手順
```bash
# ビルド
cargo build --release

# 実行
cargo run --release
```

### プロジェクト構造
- `src/win32/`: Win32 API のブラックボックス化とウィンドウ管理。
- `src/graphics/`: DirectX 11 / Direct2D による描画エンジン。
- `src/ui/`: UI コンポーネントと操作ロジック。
- `src/settings/`: 設定の永続化と管理。

---

## 📜 ライセンス
MIT License

## 📝 変更履歴
### v3.0.0 (In Progress)
- **フルスクラッチ刷新**: `winit`, `softbuffer`, `tiny-skia` を廃止し、純粋な Win32 と DirectX 11 へ移行。
- **低負荷化**: メッセージループの最適化によりアイドル時の CPU 負荷を最小化。
- **高品質透過**: DirectComposition によるネイティブ透過描画を実装。
- **永続化の実装**: 位置、サイズ、背景色、透過度を自動保存し、起動時に自動復元。
- **操作系の強化**: `Alt + ドラッグ` による透過度調整、`Ctrl + V` による動的な色変更を実装。

### v2.0.0
- C# (WPF) から Rust へ移行。パフォーマンスを大幅に改善。
