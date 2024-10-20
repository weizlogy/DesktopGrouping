using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Input;

namespace Desktop_Grouping {
  /// <summary>
  /// NotifyIconのコンテキストメニューからNew Groupを選んだときの動作
  /// </summary>
  public class NewGroupMenu : ICommand {
    /// <summary>
    /// 使わない
    /// </summary>
    public event EventHandler? CanExecuteChanged;

    /// <summary>
    /// 使わない
    /// </summary>
    /// <param name="parameter"></param>
    /// <returns></returns>
    public bool CanExecute(object? parameter) {
      return true;
    }

    /// <summary>
    /// 現在時刻をIDとして新しいGroupのWindowを作る
    /// </summary>
    /// <param name="parameter"></param>
    /// <exception cref="ArgumentNullException"></exception>
    public void Execute(object? parameter) {
      if (parameter == null) {
        throw new ArgumentNullException(nameof(parameter));
      }
      ((MainVM)parameter).CreateNewGroup(DateTime.Now.ToString("yyyyMMddhhmmssfff"));
    }
  }
}
