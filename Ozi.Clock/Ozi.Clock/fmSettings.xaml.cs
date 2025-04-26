using System.Windows;
using System.Windows.Input;

namespace Ozi.Clock;

public partial class FmSettings : Window
{
    private readonly MainWindow _fFmMain;

    public FmSettings(MainWindow fmMain)
    {
        InitializeComponent();
        _fFmMain = fmMain;
        slTransparent.ValueChanged += slTransparent_ValueChanged;
        cbTransparency.IsChecked = _fFmMain.IsTransparent;
        slTransparent.IsEnabled = cbTransparency.IsChecked.Value;
        slTransparent.Value = _fFmMain.Opacity;
        cbShowInTaskBar.IsChecked = _fFmMain.ShowInTaskbar;
        cbShowInTaskBar.IsChecked = _fFmMain.ShowInTaskbar;
        cbTopMost.IsChecked = _fFmMain.Topmost;
        cbAutoHide.IsChecked = _fFmMain.IsAutoFold;
        cbSnap.IsChecked = _fFmMain.UseSnap;
    }

    private void btOk_Click(object sender, RoutedEventArgs e)
    {
        Close();
    }

    private void CheckBox_Checked(object sender, RoutedEventArgs e)
    {
        _fFmMain.IsTransparent = cbTransparency.IsChecked!.Value;
        slTransparent.IsEnabled = cbTransparency.IsChecked.Value;
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        cbTransparency.IsChecked = _fFmMain.IsTransparent;
        slTransparent.Value = _fFmMain.TransparentValue;
    }

    private void Window_MouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left)
            DragMove();
    }

    private void slTransparent_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        _fFmMain.TransparentValue = slTransparent.Value;
        cbTransparency.Content = "Use transparency: (" + (int)(slTransparent.Value * 100) + "%)";
    }

    private void cbShowInTaskBar_Checked(object sender, RoutedEventArgs e)
    {
        _fFmMain.ShowInTaskbar = cbShowInTaskBar.IsChecked.Value;
    }

    private void cbTopMost_Checked(object sender, RoutedEventArgs e)
    {
        _fFmMain.Topmost = cbTopMost.IsChecked.Value;
    }

    private void cbAutoHide_Checked(object sender, RoutedEventArgs e)
    {
        _fFmMain.IsAutoFold = cbAutoHide.IsChecked.Value;
    }

    private void cbSnap_Checked(object sender, RoutedEventArgs e)
    {
        _fFmMain.UseSnap = cbSnap.IsChecked.Value;
    }
}