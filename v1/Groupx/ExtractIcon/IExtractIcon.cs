using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Media.Imaging;

namespace Desktop_Grouping.Groupx.ExtractIcon {
  public interface IExtractIcon {
    BitmapSource GetIcon(string uri, int size = 0);
  }
}
