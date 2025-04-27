using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Input;

namespace Desktop_Grouping {
  /// <summary>
  /// NotifyIconのContextMenu Close に対応するCommand
  /// </summary>
  public class CloseMenu : ICommand {
    /// <summary>
    /// 必要ない
    /// </summary>
    public event EventHandler? CanExecuteChanged;

    /// <summary>
    /// 制御しない
    /// </summary>
    /// <param name="parameter"></param>
    /// <returns></returns>
    public bool CanExecute(object? parameter) {
      return true;
    }

    /// <summary>
    /// NotifyIconのContextMenu Close が選択された
    /// </summary>
    /// <param name="parameter"></param>
    /// <exception cref="ArgumentNullException"></exception>
    public void Execute(object? parameter) {
      if (parameter == null) {
        throw new ArgumentNullException(nameof(parameter));
      }
      ((MainVM)parameter).Close();
    }

  }
}
