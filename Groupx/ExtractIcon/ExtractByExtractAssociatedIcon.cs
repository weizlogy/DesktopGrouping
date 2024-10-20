using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Interop;
using System.Windows;
using System.Windows.Media.Imaging;

namespace Desktop_Grouping.Groupx.ExtractIcon {
  public class ExtractByExtractAssociatedIcon : IExtractIcon {
    public BitmapSource GetIcon(string uri, int size = 0) {
      using var icon = Icon.ExtractAssociatedIcon(uri);
      if (icon == null) {
        throw new ExtractIconException($"Icon.ExtractAssociatedIcon is null. {uri}, {size}");
      }
      return Imaging.CreateBitmapSourceFromHIcon(
        icon.Handle
        , Int32Rect.Empty,
        BitmapSizeOptions.FromEmptyOptions());
    }
  }

  public class ExtractIconException : Exception {
    public ExtractIconException(string source) {
      this.Source = source;
    }
  }
}
