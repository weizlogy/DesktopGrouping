using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Data.Common;
using System.Diagnostics;
using System.Drawing;
using System.Linq;
using System.Reflection.Metadata;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Forms;
using System.Windows.Input;
using System.Windows.Media;

namespace Desktop_Grouping.Groupx {
  /// <summary>
  /// GroupWindowのViewModel
  /// </summary>
  public class GroupVM {

    /// <summary>
    /// <see cref="PersistanceHelper"/>
    /// </summary>
    public PersistanceHelper Helper { get; } = new PersistanceHelper();

    /// <summary>
    /// Groupのデータ
    /// </summary>
    public Group Group { get; set; } = new Group();

    /// <summary>
    /// View操作用
    /// </summary>
    public required GroupWindow View { get; set; }

    /// <summary>
    /// 空のコンストラクターは永続化で必要
    /// </summary>
    public GroupVM() { }

    /// <summary>
    /// Iconのファイルを実行する
    /// </summary>
    /// <param name="uri"></param>
    public void GroupExecute(string uri) {
      Process.Start(new ProcessStartInfo() {
        FileName = uri,
        UseShellExecute = true
      });
    }

    /// <summary>
    /// Groupの状態変更が発生した
    /// </summary>
    /// <param name="isForce"></param>
    public async void GroupChanged(bool isForce = false) {
      if (string.IsNullOrEmpty(Group.GroupID)) {
        return;
      }
      var changed = isForce;
      if (Group.Coordinates.Top != View.Top) {
        Group.Coordinates.Top = View.Top;
        changed = true;
      }
      if (Group.Coordinates.Left != View.Left) {
        Group.Coordinates.Left = View.Left;
        changed = true;
      }
      if (Group.Coordinates.Width != View.Width) {
        Group.Coordinates.Width = View.Width;
        changed = true;
      }
      if (Group.Coordinates.Height != View.Height) {
        Group.Coordinates.Height = View.Height;
        changed = true;
      }

      if (changed) {
        await Helper.Serialize(Group);
      }
    }

    /// <summary>
    /// Groupの状態を復元する
    /// </summary>
    public void GroupRestore() {
      if (string.IsNullOrEmpty(Group.GroupID)) {
        return;
      }
      var group = Helper.Deserialize(Group.GroupID);
      if (group == null) {
        this.GroupChanged(true);
        return;
      }
      // Serialize group -> Group
      Group.GroupID = group.GroupID;
      group.GroupItems.ToList().ForEach(item => {
        item.CreateImage();
        Group.GroupItems.Add(item);
      });
      Group.Coordinates.Top = group.Coordinates.Top;
      Group.Coordinates.Left = group.Coordinates.Left;
      Group.Coordinates.Width = group.Coordinates.Width;
      Group.Coordinates.Height = group.Coordinates.Height;
      Group.BGColor = group.BGColor;
      Group.BorderColor = group.BorderColor;
      Group.Opacity = group.Opacity;
      // Group -> View
      View.InvalidateMeasure();
      View.Top = Group.Coordinates.Top;
      View.Left = Group.Coordinates.Left;
      View.Width = Group.Coordinates.Width;
      View.Height = Group.Coordinates.Height;
      View.UpdateLayout();
      View.Background =
        new SolidColorBrush((System.Windows.Media.Color)System.Windows.Media.ColorConverter.ConvertFromString(Group.BGColor));
      View.BorderBrush =
        new SolidColorBrush((System.Windows.Media.Color)System.Windows.Media.ColorConverter.ConvertFromString(Group.BorderColor));
      View.Opacity = Group.Opacity;
    }

    /// <summary>
    /// Group削除
    /// </summary>
    public void GroupDelete() {
      Helper.Delete(Group.GroupID);
    }
  }
}
