using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Reflection.Metadata;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Interop;

namespace Desktop_Grouping {
  /// <summary>
  /// WPFアプリケーションを最背面に置くクラス
  ///   強制的にデスクトップの子プロセスにする
  /// </summary>
  public class UnForcusWindow {

    // Hide a WPF form from Alt+Tab
    // https://stackoverflow.com/questions/56645242/hide-a-wpf-form-from-alttab
    [DllImport("user32.dll", SetLastError = true)]
    static extern int GetWindowLong(IntPtr hWnd, int nIndex);
    [DllImport("user32.dll")]
    static extern int SetWindowLong(IntPtr hWnd, int nIndex, int dwNewLong);
    private const int GWL_EX_STYLE = -20;
    private const int WS_EX_APPWINDOW = 0x00040000, WS_EX_TOOLWINDOW = 0x00000080;

    /// wpf 最背面
    ///   https://gurizuri0505.halfmoon.jp/develop/csharp/zorder
    [DllImport("user32.dll", SetLastError = true)]
    static extern IntPtr FindWindow(string? lpClassName, string lpWindowName);
    [DllImport("user32.dll")]
    static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
    static readonly IntPtr HWND_BOTTOM = new IntPtr(1);
    const UInt32 SWP_NOSIZE = 0x0001;
    const UInt32 SWP_NOMOVE = 0x0002;
    const UInt32 SWP_NOACTIVATE = 0x0010;
    const int WM_WINDOWPOSCHANGING = 0x0046;
    const uint WM_WINDOWPOSCHANGED = 0x0047;

    /// <summary>
    /// フォーカスを取らないようにする
    /// </summary>
    /// <param name="window"></param>
    public void UnForcus(Window window) {
      // Variable to hold the handle for the form
      var helper = new WindowInteropHelper(window).Handle;
      // Performing some magic to hide the form from Alt+Tab
      SetWindowLong(helper, GWL_EX_STYLE, (GetWindowLong(helper, GWL_EX_STYLE) | WS_EX_TOOLWINDOW) & ~WS_EX_APPWINDOW);
    }

    /// <summary>
    /// 最背面にする
    /// </summary>
    /// <param name="window"></param>
    public void BackMost(Window window) {
      // https://mamesfactory.com/790/
      HwndSource source = HwndSource.FromHwnd(new WindowInteropHelper(window).Handle);
      source.AddHook(new HwndSourceHook((IntPtr hWnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled) => {
        // WM_WINDOWPOSCHANGINGではマウス押下状態で前面に出てきてしまう
        if (msg == WM_WINDOWPOSCHANGED) {
          SetWindowPos(hWnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
          //handled = true;  // とかやると、LocationChanged event 発生しなくなるので......
        }
        return IntPtr.Zero;
      }));
    }

  }
}
