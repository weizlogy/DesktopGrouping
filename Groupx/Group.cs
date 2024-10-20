using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Desktop_Grouping.Groupx {
  /// <summary>
  /// Desktop Groupingの１グループ単位
  /// </summary>
  public class Group : INotifyPropertyChanged {
    /// <summary>
    /// IDはnew groupしたときのハンドル
    /// </summary>
    public string GroupID { get; set; } = "";
    /// <summary>
    /// 内包するものたち
    /// </summary>
    public IList<GroupItem> GroupItems { get; set; } = new ObservableCollection<GroupItem>();
    /// <summary>
    /// 表示座標
    /// </summary>
    public Coordinate Coordinates { get; set; } = new Coordinate();
    /// <summary>
    /// 背景色ARGB
    /// </summary>
    public string BGColor { get; set; } = "#00FFFFFF";
    /// <summary>
    /// Windowの透過度
    /// </summary>
    public double Opacity { get; set; } = 0.5;

    /// <summary>
    /// 変更通知
    /// </summary>
    public event PropertyChangedEventHandler? PropertyChanged;
  }

  /// <summary>
  /// 座標
  /// </summary>
  public class Coordinate {
    /// <summary>
    /// Y軸
    /// </summary>
    public double Top { get; set; } = -1;
    /// <summary>
    /// X軸
    /// </summary>
    public double Left { get; set; } = -1;
    /// <summary>
    /// 幅
    /// </summary>
    public double Width { get; set; } = -1;
    /// <summary>
    /// 高さ
    /// </summary>
    public double Height { get; set; } = -1;
  }
}
