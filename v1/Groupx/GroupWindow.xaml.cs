using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shapes;
using Desktop_Grouping.Groupx;

namespace Desktop_Grouping {
  /// <summary>
  /// GroupWindow.xaml の相互作用ロジック
  /// </summary>
  public partial class GroupWindow : Window {

    /// <summary>
    /// <see cref="UnForcusWindow"/>
    /// </summary>
    protected UnForcusWindow ufw = new UnForcusWindow();

    /// <summary>
    /// WindowLoadedのイベントが発生したかどうか
    /// </summary>
    protected bool isWindowLoaded = false;

    /// <summary>
    /// コンストラクター
    /// </summary>
    public GroupWindow() {
      InitializeComponent();
      // xamlでできないdatacontextのPropertyを設定
      ((GroupVM)this.DataContext).View = this;
    }

    /// <summary>
    /// DropしたファイルをGroupに追加する
    /// 設定ファイルに書き込む
    /// </summary>
    /// <param name="sender"></param>
    /// <param name="e"></param>
    private void ListView_Drop(object sender, DragEventArgs e) {
      if (!e.Data.GetDataPresent(DataFormats.FileDrop)) {
        return;
      }
      ((string[])e.Data.GetData(DataFormats.FileDrop)).ToList().ForEach((f) => {
        this.vm.Group.GroupItems.Add(new Groupx.GroupItem(f));
      });
      this.vm.GroupChanged(true);
    }

    /// <summary>
    /// WindowLoadedのイベントが発生した
    /// </summary>
    /// <param name="sender"></param>
    /// <param name="e"></param>
    private void Window_Loaded(object sender, RoutedEventArgs e) {
      // Focusなし
      this.ufw.UnForcus(this);
      // 背面行き
      this.ufw.BackMost(this);
      // ID取得、復元
      this.vm.Group.GroupID = this.Uid;
      this.vm.GroupRestore();
      this.isWindowLoaded = true;
    }

    /// <summary>
    /// そのIconのファイルを実行する
    /// </summary>
    /// <param name="sender"></param>
    /// <param name="e"></param>
    private void ContentControl_MouseDoubleClick(object sender, MouseButtonEventArgs e) {
      try {
        this.vm.GroupExecute(((Groupx.GroupItem)this.GroupList.SelectedItem).Uri);
      } catch (Exception ex) {
        MessageBox.Show(ex.Message);
      }
    }

    /// <summary>
    /// そのIconをGroupから削除する
    /// </summary>
    /// <param name="sender"></param>
    /// <param name="e"></param>
    private void ContentControl_MouseRightButtonUp(object sender, MouseButtonEventArgs e) {
      e.Handled = true;
      this.vm.Group.GroupItems.RemoveAt(this.GroupList.SelectedIndex);
      this.vm.GroupChanged(true);
    }

    /// <summary>
    /// リサイズ
    /// </summary>
    /// <param name="sender"></param>
    /// <param name="e"></param>
    private void Window_SizeChanged(object sender, SizeChangedEventArgs e) {
      if (!this.isWindowLoaded) {
        return;
      }
      this.vm.GroupChanged();
    }

    /// <summary>
    /// Groupの設定画面を開く
    /// </summary>
    /// <param name="sender"></param>
    /// <param name="e"></param>
    private void GroupList_MouseRightButtonUp(object sender, MouseButtonEventArgs e) {
      // UIDを渡す
      var option = new GroupOption() { Title = $"Preference@{this.vm.Group.GroupID}" };
      // 現在値の設定
      option.colorpicker.SelectedColor = ((SolidColorBrush)this.Background).Color;
      option.colorpicker_border.SelectedColor = ((SolidColorBrush)this.BorderBrush).Color;
      option.opacity_slider.Value = this.vm.Group.Opacity;
      // 設定画面でGroupの色を変えるとリアルタイムで反映する
      // ただし保存はしない
      option.colorpicker.ColorChanged += (s, e) => {
        this.vm.Group.BGColor = option.colorpicker.SelectedColor.ToString();
        this.Background =
          new SolidColorBrush((Color)ColorConverter.ConvertFromString(this.vm.Group.BGColor));
      };
      option.colorpicker_border.ColorChanged += (s, e) => {
        this.vm.Group.BorderColor = option.colorpicker_border.SelectedColor.ToString();
        this.BorderBrush =
          new SolidColorBrush((Color)ColorConverter.ConvertFromString(this.vm.Group.BorderColor));
      };
      option.opacity_slider.ValueChanged += (s, e) => {
        this.vm.Group.Opacity = option.opacity_slider.Value;
        this.Opacity = this.vm.Group.Opacity;
      };
      // 設定画面を閉じたら保存する
      option.Closed += (s, e) => {
        this.vm.GroupChanged(true);
      };
      // Delete GroupボタンでGroup自体を消す
      option.ButtonDeleteGroup.Click += (s, e) => {
        var rs = MessageBox.Show($"delete group {this.vm.Group.GroupID}?", "", MessageBoxButton.OKCancel);
        switch (rs) {
          case MessageBoxResult.OK:
            option.Close();
            this.Close();
            this.vm.GroupDelete();
            break;
          case MessageBoxResult.Cancel:
            break;
        }
      };
      option.Show();
    }

    /// <summary>
    /// Drag終わりで設定ファイルに保存
    /// </summary>
    /// <param name="sender"></param>
    /// <param name="e"></param>
    private void Window_MouseLeftButtonUp(object sender, MouseButtonEventArgs e) {
      this.vm.GroupChanged();
    }

    /// <summary>
    /// Drag状態でWindowの移動
    /// </summary>
    /// <param name="sender"></param>
    /// <param name="e"></param>
    private void Window_MouseLeftButtonDown(object sender, MouseButtonEventArgs e) {
      this.DragMove();
    }

    private void Window_MouseEnter(object sender, MouseEventArgs e) {
      this.Opacity = 1.0;
    }

    private void Window_MouseLeave(object sender, MouseEventArgs e) {
      this.Opacity = this.vm.Group.Opacity;
    }

  }
}
