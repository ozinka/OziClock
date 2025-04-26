using System.Windows;

namespace Ozi.Clock;

/// <summary>
/// Interaction logic for fmTimeChecker.xaml
/// </summary>
public partial class fmTimeChecker : Window
{
    private MainWindow fFmMain;
    public DateTime curTime;

    public fmTimeChecker(MainWindow fmMain)
    {
        InitializeComponent();
        fFmMain = fmMain;
        Left = fFmMain.Left;
        Top = fFmMain.Top + fFmMain.Height;
    }
}