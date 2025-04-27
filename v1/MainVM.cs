using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using Desktop_Grouping;

namespace Desktop_Grouping {
  /// <summary>
  /// MainWindowのViewModel
  /// </summary>
  public class MainVM {
    /// <summary>
    /// NotifyIconのContextMenu New Group に対応するCommand
    /// </summary>
    public ICommand NewGroupCommand { get; } = new NewGroupMenu();
    /// <summary>
    /// NotifyIconのContextMenu Close に対応するCommand
    /// </summary>
    public ICommand CloseCommand { get; } = new CloseMenu();

    protected PersistanceHelper helper = new PersistanceHelper();

    /// <summary>
    /// View操作用
    /// </summary>
    public required MainWindow View { get; set; }

    /// <summary>
    /// コンストラクター
    /// </summary>
    public MainVM() { }

    /// <summary>
    /// 新しいGroupを作る
    /// </summary>
    /// <param name="uid"></param>
    public void CreateNewGroup(string uid) {
      new GroupWindow() { Uid = uid }.Show();
    }

    /// <summary>
    /// 起動時にすべての設定を読み込み、Groupを復元する
    /// </summary>
    public void RestoreAllGroupWindows() {
      Directory.GetFiles(".", PersistanceHelper.PREFIX + "*").ToList().ForEach(file => {
        // Group復元
        new GroupWindow() { Uid = Path.GetFileName(file).Replace(PersistanceHelper.PREFIX, "") }.Show();
      });
    }

    /// <summary>
    /// NotifyIconのContextMenu Close の実装
    /// </summary>
    public void Close() {
      Application.Current.Shutdown();
    }
  }
}
