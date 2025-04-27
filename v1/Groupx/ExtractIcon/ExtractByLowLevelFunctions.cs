using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Runtime.InteropServices;
using System.Security.Policy;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Forms;
using System.Windows.Interop;
using System.Windows.Media.Imaging;

namespace Desktop_Grouping.Groupx.ExtractIcon {
  public class ExtractByLowLevelFunctions : IExtractIcon {

    [DllImport("User32.dll", CharSet = CharSet.Auto)]
    internal static extern UInt32 PrivateExtractIcons(
      String lpszFile, int nIconIndex, int cxIcon, int cyIcon, IntPtr[]? phicon, IntPtr[]? piconid, UInt32 nIcons, UInt32 flags);

    public static Guid IID_IImageList = new Guid("46EB5926-582E-4017-9FDF-E8998DAA0950");

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
    public struct SHFILEINFO {
      public IntPtr hIcon;
      public int iIcon;
      public uint dwAttributes;
      [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)]
      public string szDisplayName;
      [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 80)]
      public string szTypeName;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct IMAGELISTDRAWPARAMS {
      public int cbSize;
      public IntPtr himl;
      public int i;
      public IntPtr hdcDst;
      public int x;
      public int y;
      public int cx;
      public int cy;
      public int xBitmap;    // x offest from the upperleft of bitmap
      public int yBitmap;    // y offset from the upperleft of bitmap
      public int rgbBk;
      public int rgbFg;
      public int fStyle;
      public int dwRop;
      public int fState;
      public int Frame;
      public int crEffect;
    }

    [Flags]
    public enum SHGFI {
      SHGFI_ICON = 0x000000100,
      SHGFI_DISPLAYNAME = 0x000000200,
      SHGFI_TYPENAME = 0x000000400,
      SHGFI_ATTRIBUTES = 0x000000800,
      SHGFI_ICONLOCATION = 0x000001000,
      SHGFI_EXETYPE = 0x000002000,
      SHGFI_SYSICONINDEX = 0x000004000,
      SHGFI_LINKOVERLAY = 0x000008000,
      SHGFI_SELECTED = 0x000010000,
      SHGFI_ATTR_SPECIFIED = 0x000020000,
      SHGFI_LARGEICON = 0x000000000,
      SHGFI_SMALLICON = 0x000000001,
      SHGFI_OPENICON = 0x000000002,
      SHGFI_SHELLICONSIZE = 0x000000004,
      SHGFI_PIDL = 0x000000008,
      SHGFI_USEFILEATTRIBUTES = 0x000000010,
      SHGFI_ADDOVERLAYS = 0x000000020,
      SHGFI_OVERLAYINDEX = 0x000000040
    }
    [Flags]
    public enum SHIL {
      // 32x32
      SHIL_LARGE = 0x0000,
      // 48x48
      SHIL_EXTRALARGE = 0x0002,
      // 256x256
      SHIL_JUMBO = 0x0004,
    }
    [Flags]
    public enum ImageListDrawItemConstants : int {
      /// <summary>
      /// Draw item normally.
      /// </summary>
      ILD_NORMAL = 0x0,
      /// <summary>
      /// Draw item transparently.
      /// </summary>
      ILD_TRANSPARENT = 0x1,
      /// <summary>
      /// Draw item blended with 25% of the specified foreground colour
      /// or the Highlight colour if no foreground colour specified.
      /// </summary>
      ILD_BLEND25 = 0x2,
      /// <summary>
      /// Draw item blended with 50% of the specified foreground colour
      /// or the Highlight colour if no foreground colour specified.
      /// </summary>
      ILD_SELECTED = 0x4,
      /// <summary>
      /// Draw the icon's mask
      /// </summary>
      ILD_MASK = 0x10,
      /// <summary>
      /// Draw the icon image without using the mask
      /// </summary>
      ILD_IMAGE = 0x20,
      /// <summary>
      /// Draw the icon using the ROP specified.
      /// </summary>
      ILD_ROP = 0x40,
      /// <summary>
      /// ?
      /// </summary>
      ILD_OVERLAYMASK = 0xF00,
      /// <summary>
      /// Preserves the alpha channel in dest. XP only.
      /// </summary>
      ILD_PRESERVEALPHA = 0x1000, // 
      /// <summary>
      /// Scale the image to cx, cy instead of clipping it.  XP only.
      /// </summary>
      ILD_SCALE = 0x2000,
      /// <summary>
      /// Scale the image to the current DPI of the display. XP only.
      /// </summary>
      ILD_DPISCALE = 0x4000
    }

    [DllImport("shell32.dll")]
    public static extern IntPtr SHGetFileInfo(string pszPath, uint dwFileAttribs, out SHFILEINFO psfi, uint cbFileInfo, SHGFI uFlags);

    [DllImport("shell32.dll")]
    public static extern int SHGetImageList(SHIL iImageList, ref Guid riid, out IImageList ppv);

    [DllImport("comctl32.dll", SetLastError = true)]
    public static extern IntPtr ImageList_GetIcon(IntPtr himl, int i, int flags);

    [DllImport("user32.dll", CharSet = CharSet.Auto)]
    extern static bool DestroyIcon(IntPtr handle);

    // interface COM IImageList
    [ComImportAttribute()]
    [GuidAttribute("46EB5926-582E-4017-9FDF-E8998DAA0950")]
    [InterfaceTypeAttribute(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IImageList {
      [PreserveSig]
      int Add(IntPtr hbmImage, IntPtr hbmMask, ref int pi);

      [PreserveSig]
      int ReplaceIcon(int i, IntPtr hicon, ref int pi);

      [PreserveSig]
      int SetOverlayImage(int iImage, int iOverlay);

      [PreserveSig]
      int Replace(int i, IntPtr hbmImage, IntPtr hbmMask);

      [PreserveSig]
      int AddMasked(IntPtr hbmImage, int crMask, ref int pi);

      [PreserveSig]
      int Draw(ref IMAGELISTDRAWPARAMS pimldp);

      [PreserveSig]
      int Remove(int i);

      [PreserveSig]
      int GetIcon(int i, int flags, ref IntPtr picon);
    };

    /// <summary>
    /// 
    /// </summary>
    /// <param name="uri"></param>
    /// <param name="size"></param>
    /// <returns></returns>
    public BitmapSource GetIcon(string uri, int size = 0) {
      //var bitmap = UsePrivateExtractIcons(uri, size);
      var bitmap = UseSHGetFileInfo(uri, size);
      return bitmap;
    }

    private BitmapSource UseSHGetFileInfo(string uri, int size) {
      SHFILEINFO shinfo = new SHFILEINFO();

      IntPtr hImg = SHGetFileInfo(uri, 0, out shinfo, (uint)Marshal.SizeOf(typeof(SHFILEINFO)), SHGFI.SHGFI_SYSICONINDEX | SHGFI.SHGFI_USEFILEATTRIBUTES);
      if (hImg == IntPtr.Zero) {
        throw new Exception("SHGetFileInfo return 0");
      }

      SHIL currentshil = SHIL.SHIL_EXTRALARGE;

      IImageList? imglist = null;
      int rsult = SHGetImageList(currentshil, ref IID_IImageList, out imglist);

      IntPtr hicon = IntPtr.Zero;
      imglist.GetIcon(shinfo.iIcon, (int)ImageListDrawItemConstants.ILD_TRANSPARENT, ref hicon);

      using Icon myIcon = Icon.FromHandle(hicon);
      DestroyIcon(shinfo.iIcon);
      return ToBitmapSource(myIcon);
    }

    /// <summary>
    /// SHELL32.DLLなら取れたけど、そこらのexeやdllは無理だった
    /// </summary>
    /// <param name="uri"></param>
    /// <param name="size"></param>
    /// <returns></returns>
    private BitmapSource UsePrivateExtractIcons(string uri, int size) {
      var iconIndex = PrivateExtractIcons(uri, 0, 0, 0, null, null, 0, 0);

      var phicon = new IntPtr[iconIndex];
      var piconid = new IntPtr[iconIndex];
      PrivateExtractIcons(uri, 0, size, size, phicon, piconid, iconIndex, 0);

      using var icon = Icon.FromHandle(phicon[0]);
      return ToBitmapSource(icon);
    }

    private BitmapSource ToBitmapSource(Icon icon) {
      var bitmap = icon.ToBitmap();
      var hBitmap = bitmap.GetHbitmap();
      return Imaging.CreateBitmapSourceFromHBitmap(hBitmap, IntPtr.Zero, Int32Rect.Empty, BitmapSizeOptions.FromEmptyOptions());
    }

  }
}
