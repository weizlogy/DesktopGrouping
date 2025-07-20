# Desktop Grouping (v2.0.0) 🧹✨

<!-- TODO: Add a screenshot or GIF of the application in action -->

A simple and intuitive application to organize your desktop icons into groups.

デスクトップのアイコンをグループ化して整理整頓するための、シンプルで直感的なアプリケーションです。

## ✨ Features / 主な機能

- **Create Groups**: Create multiple groups on your desktop to organize icons.
  - `Right-click` the tray icon and select `New Group`.
- **Drag & Drop**: Easily add files and shortcuts to groups by dragging and dropping them.
- **Launch Items**:
  - `Left-click` an icon to launch the application or open the file.
  - `Right-click` an icon to open its containing folder.
- **Customization**:
  - **Move**: `Ctrl + Drag` a group to move it.
  - **Resize**: `Shift + Drag` a group to resize it.
  - **Color**: Paste a hex code (e.g., `#FF000099`, `#0F0`) with `Ctrl + V` onto a group to change its background color. Use `#Random` for a random color.
  - **Transparency**: Adjust transparency with `Ctrl + Mouse Wheel`.
- **Delete Groups**: `Ctrl + Right-click` on an empty area of a group to delete it.

## 🚀 Installation / インストール

1.  Go to the **Releases** page.
2.  Download the latest `Desktop Grouping_vX.X.X_installer.exe`.
3.  Run the installer and follow the on-screen instructions.

---

1.  **リリースページ**にアクセスします。
2.  最新の `Desktop Grouping_vX.X.X_installer.exe` をダウンロードします。
3.  インストーラーを実行し、画面の指示に従ってインストールを完了します。

## 🛠️ Building from Source / ソースからのビルド

If you prefer to build the application from source, you'll need to have the Rust toolchain installed.

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/your-username/desktop-grouping.git
    cd desktop-grouping
    ```

2.  **Build the project:**
    ```bash
    cargo make build-release
    ```

3.  The executable will be located at `target/release/desktop_grouping.exe`.

4.  **Create installer:**
    ```bash
    cargo make installer
    ```

## 🤝 Contributing / コントリビューション

Contributions are welcome! If you have a suggestion or find a bug, please open an issue to discuss it.
Pull requests are also greatly appreciated.

コントリビューションを歓迎します！提案やバグの発見がありましたら、気軽にIssueを立てて議論してください。
プルリクエストも大歓迎です。

## 📜 ライセンス

This project is licensed under the MIT License. See the `LICENSE` file for details.

## 📝 Changelog

### v2.0.0

- Rewritten in Rust for improved performance and responsiveness.
- Faster application startup and smoother window dragging.

### v1.0.0

- Initial release.
- Developed with C# (WPF).
