using System.Windows;
using System.Windows.Input;

namespace Ozi.Utilities;

public partial class FmSettings : Window
{
    public FmSettings(FmMainWindow fmMain)
    {
        InitializeComponent();
    }

    private void btOk_Click(object sender, RoutedEventArgs e)
    {
        Close();
    }

    private void Window_MouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left)
        {
            DragMove();
        }
    }

    private void SlTransparent_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        lbOpacity.Content = $"Opacity: {(int)(slTransparent.Value * 100)}%";
    }
}