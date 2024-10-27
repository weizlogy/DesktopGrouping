using Desktop_Grouping.Groupx.ExtractIcon;
using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json.Serialization;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace Desktop_Grouping.Groupx {
  /// <summary>
  /// Groupの各Icon
  /// </summary>
  public class GroupItem {

    /// <summary>
    /// 表示名
    /// </summary>
    public string Name { get; set; } = "";
    /// <summary>
    /// 実体ファイルパス
    /// </summary>
    public string Uri { get; set; } = "";
    /// <summary>
    /// Icon画像
    /// </summary>
    [JsonIgnore]
    public BitmapSource? Image { get; set; }

    /// <summary>
    /// 空のコンストラクターは永続化に必要
    /// </summary>
    public GroupItem() { }

    /// <summary>
    /// コンストラクター
    /// </summary>
    /// <param name="fileDrop"></param>
    public GroupItem(string fileDrop) {
      var file = new Uri(fileDrop);
      Name = Path.GetFileNameWithoutExtension(fileDrop);
      Uri = file.OriginalString;

      CreateImage();
    }

    /// <summary>
    /// Icon画像を取得する
    /// </summary>
    public void CreateImage() {
      if (Uri == null) {
        return;
      }
      try {
        // ドラッグされたのが画像ファイルだったらこれでいい
        // そうじゃなければ例外が発生する（寧ろこっちが本命
        using var stream = new MemoryStream(File.ReadAllBytes(Uri));
        Image = new WriteableBitmap(BitmapFrame.Create(stream));
        // これだと例外発生しても参照を持ち続けるのでだめ
        //   Image = new BitmapImage(new Uri(Uri));
      } catch {
        // ExtractByLowLevelFunctionsで取れなかったらExtractByExtractAssociatedIconで取得する感じ
        try {
          Image = new ExtractByLowLevelFunctions().GetIcon(Uri);
        } catch {
          Image = new ExtractByExtractAssociatedIcon().GetIcon(Uri);
        }
      }
    }
  }
}
