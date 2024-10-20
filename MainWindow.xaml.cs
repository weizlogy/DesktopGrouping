using System.Runtime.InteropServices;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Shapes;

namespace Desktop_Grouping {
  /// <summary>
  /// Interaction logic for MainWindow.xaml
  /// </summary>
  public partial class MainWindow : Window {

    /// <summary>
    /// <see cref="UnForcusWindow"/>
    /// </summary>
    protected UnForcusWindow ufw = new UnForcusWindow();

    /// <summary>
    /// MainWindow is Loaded
    /// </summary>
    /// <param name="sender"></param>
    /// <param name="e"></param>
    public void Window_Loaded(object sender, RoutedEventArgs e) {
      this.ufw.UnForcus(this);
    }

    /// <summary>
    /// Initialize MainWindow
    /// </summary>
    public MainWindow() {
      InitializeComponent();
      // xamlでできないdatacontextのPropertyを設定
      ((MainVM)this.DataContext).View = this;
      // GroupWindow復元
      ((MainVM)this.DataContext).RestoreAllGroupWindows();
    }

  }
}