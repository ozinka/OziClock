using System.Windows;
using System.Windows.Input;

namespace Ozi.Clock;

public partial class FmSettings : Window
{
    private MainWindow _fFmMain;

    public FmSettings(MainWindow fmMain)
    {
        InitializeComponent();
        this._fFmMain = fmMain;
        slTransparent.ValueChanged += slTransparent_ValueChanged;
        cbTransparency.IsChecked = _fFmMain.isTransparent;
        slTransparent.IsEnabled = cbTransparency.IsChecked.Value;
        slTransparent.Value = _fFmMain.Opacity;
        cbShowInTaskBar.IsChecked = _fFmMain.ShowInTaskbar;
        cbShowInTaskBar.IsChecked = _fFmMain.ShowInTaskbar;
        cbTopMost.IsChecked = _fFmMain.Topmost;
        cbAutoHide.IsChecked = _fFmMain.isAutoFold;
        cbSnap.IsChecked = _fFmMain.useSnap;
    }

    private void btOk_Click(object sender, RoutedEventArgs e)
    {
        this.Close();
    }

    private void CheckBox_Checked(object sender, RoutedEventArgs e)
    {
        _fFmMain.isTransparent = cbTransparency.IsChecked.Value;
        slTransparent.IsEnabled = cbTransparency.IsChecked.Value;
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        cbTransparency.IsChecked = _fFmMain.isTransparent;
        slTransparent.Value = _fFmMain.transparentValue;
    }

    private void Window_MouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left)
            this.DragMove();
    }

    private void slTransparent_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        _fFmMain.transparentValue = slTransparent.Value;
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
        _fFmMain.isAutoFold = cbAutoHide.IsChecked.Value;
    }

    private void cbSnap_Checked(object sender, RoutedEventArgs e)
    {
        _fFmMain.useSnap = cbSnap.IsChecked.Value;
    }
}